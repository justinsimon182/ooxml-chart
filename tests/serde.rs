//! A chart declared as data. Run with `cargo test --features serde`.
#![cfg(feature = "serde")]

use ooxml_chart::{
    Axis, ChartKind, ChartSpec, DataLabels, DataTable, DisplayUnit, ErrorAmount, ErrorBarSide,
    ErrorBars, ErrorValues, Layout, LegendPosition, MarkerSymbol, Plot, PointFormat, PointLabel,
    Position, Series, SeriesName, TextStyle,
};

const TERSE: &str = r#"{
    "kind": "column_clustered",
    "title": "Revenue",
    "series": [
        { "name": { "literal": "North" }, "values": "'S'!$B$2:$B$5", "categories": "'S'!$A$2:$A$5", "color": "8E0DD1" }
    ]
}"#;

fn built() -> ChartSpec {
    ChartSpec::new(ChartKind::ColumnClustered)
        .title("Revenue")
        .series(
            Series::new(SeriesName::Literal("North".into()), "'S'!$B$2:$B$5")
                .with_categories("'S'!$A$2:$A$5")
                .with_color("8E0DD1"),
        )
}

#[test]
fn a_terse_json_spec_equals_the_same_chart_built_in_code() {
    let parsed: ChartSpec = serde_json::from_str(TERSE).expect("parses");
    assert_eq!(parsed, built());
    assert_eq!(parsed.render().unwrap(), built().render().unwrap());
}

#[test]
fn omitted_fields_take_excels_defaults_not_zeros() {
    let parsed: ChartSpec = serde_json::from_str(TERSE).expect("parses");
    let xml = String::from_utf8(parsed.render().unwrap().xml).unwrap();
    assert!(xml.contains(r#"<c:gapWidth val="150"/>"#), "{xml}");
    assert!(xml.contains(r#"<c:legendPos val="r"/>"#), "{xml}");
}

#[test]
fn a_rich_spec_survives_a_round_trip() {
    let spec = built()
        .legend(LegendPosition::Bottom)
        .value_axis(
            Axis::default()
                .title("USD")
                .gridlines(true)
                .min(0.0)
                .log(10),
        )
        .title_style(TextStyle::new().size(14.0).bold(true))
        .data_labels(DataLabels::values().with_category())
        .plot(
            Plot::new(ChartKind::LineMarkers)
                .series(Series::new(
                    SeriesName::Reference("'S'!$C$1".into()),
                    "'S'!$C$2:$C$5",
                ))
                .on_secondary_axis(),
        );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn point_overrides_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::LineMarkers).series(
        Series::new(SeriesName::Literal("A".into()), "'S'!$B$2:$B$5")
            .with_point(1, PointFormat::new().marker(MarkerSymbol::Diamond, 9))
            .with_point_label(1, PointLabel::text("Peak"))
            .with_point_label(2, PointLabel::hidden()),
    );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn error_bars_survive_a_round_trip_and_default_sensibly() {
    let spec = ChartSpec::new(ChartKind::Scatter).series(
        Series::new(SeriesName::Literal("A".into()), "'S'!$B$2:$B$5")
            .with_error_bars(ErrorBars::new(ErrorAmount::Percentage(5.0)).along_x())
            .with_error_bars(
                ErrorBars::new(ErrorAmount::Custom {
                    plus: Some(ErrorValues::Reference("'S'!$D$2:$D$5".into())),
                    minus: None,
                })
                .side(ErrorBarSide::Plus)
                .end_cap(false),
            ),
    );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);

    // Only `amount` is required; the cap is on unless turned off.
    let terse: ChartSpec = serde_json::from_str(
        r#"{"kind":"line","series":[{"name":{"literal":"A"},"values":"S!$B$2:$B$5","error_bars":[{"amount":{"fixed":2.0}}]}]}"#,
    )
    .expect("parses");
    assert!(terse.render().is_ok());
    let value = serde_json::to_value(&terse).expect("serialises");
    let bars = &value["series"][0]["error_bars"][0];
    assert_eq!(bars["end_cap"], true);
    assert_eq!(bars["side"], "both");
}

#[test]
fn layouts_survive_a_round_trip_and_inner_defaults_on() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .plot_area_layout(Layout::new(0.1, 0.2, 0.6, 0.6).outer())
        .legend_layout(Layout::new(0.75, 0.3, 0.2, 0.3));
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);

    let terse: ChartSpec = serde_json::from_str(
        r#"{"kind":"line","series":[{"name":{"literal":"A"},"values":"S!$B$2:$B$5"}],"plot_area_layout":{"x":0.1,"y":0.1,"width":0.5,"height":0.5}}"#,
    )
    .expect("parses");
    let value = serde_json::to_value(&terse).expect("serialises");
    assert_eq!(value["plot_area_layout"]["inner"], true);
}

#[test]
fn title_positions_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .title("T")
        .title_position(Position::new(0.3, 0.02))
        .value_axis(
            Axis::default()
                .title("U")
                .title_position(Position::new(0.01, 0.4)),
        );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn axis_lines_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .category_axis(Axis::default().line("8E0DD1").line_width(2.0))
        .value_axis(Axis::default().no_line());
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn data_tables_survive_a_round_trip_and_default_on() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .data_table(DataTable::new().outline(false).legend_keys(false));
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);

    let terse: ChartSpec = serde_json::from_str(
        r#"{"kind":"line","series":[{"name":{"literal":"A"},"values":"S!$B$2:$B$5"}],"data_table":{}}"#,
    )
    .expect("parses");
    let value = serde_json::to_value(&terse).expect("serialises");
    assert_eq!(value["data_table"]["outline"], true);
    assert_eq!(value["data_table"]["legend_keys"], true);
}

#[test]
fn legend_entries_and_leader_lines_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Pie)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .hide_legend_entry(2)
        .data_labels(DataLabels::values().with_leader_lines());
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn date_tick_units_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .category_axis(
            Axis::default()
                .dates(ooxml_chart::DateUnit::Days)
                .date_major(2, ooxml_chart::DateUnit::Months)
                .date_minor(1, ooxml_chart::DateUnit::Months),
        );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn crosses_at_survives_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .category_axis(Axis::default().crosses_at(25.0));
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
}

#[test]
fn display_units_survive_a_round_trip() {
    let spec = ChartSpec::new(ChartKind::Line)
        .series(Series::new(SeriesName::Literal("A".into()), "S!$B$2:$B$5"))
        .value_axis(
            Axis::default()
                .display_units(DisplayUnit::Custom(25.0))
                .display_units_label(true),
        );
    let json = serde_json::to_string(&spec).expect("serialises");
    let back: ChartSpec = serde_json::from_str(&json).expect("parses");
    assert_eq!(back, spec);
    let terse = r#"{"kind":"line","series":[{"name":{"literal":"A"},"values":"S!$B$2:$B$5"}],"value_axis":{"display_unit":"millions"}}"#;
    let parsed: ChartSpec = serde_json::from_str(terse).expect("parses");
    assert!(parsed.render().is_ok());
}

#[test]
fn a_misspelt_field_is_an_error_not_silently_ignored() {
    let error =
        serde_json::from_str::<ChartSpec>(r#"{ "kind": "pie", "series": [], "titel": "oops" }"#)
            .unwrap_err();
    assert!(error.to_string().contains("titel"), "{error}");
}

#[test]
fn a_parsed_spec_is_still_validated_when_rendered() {
    let spec: ChartSpec = serde_json::from_str(
        r#"{ "kind": "line", "series": [ { "name": { "literal": "A" }, "values": "S!$A$1", "color": "not-a-colour" } ] }"#,
    )
    .expect("parses");
    assert!(matches!(
        spec.render(),
        Err(ooxml_chart::ChartError::InvalidColor(_))
    ));
}
