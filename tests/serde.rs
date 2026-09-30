//! A chart declared as data. Run with `cargo test --features serde`.
#![cfg(feature = "serde")]

use ooxml_chart::{
    Axis, ChartKind, ChartSpec, DataLabels, LegendPosition, MarkerSymbol, Plot, PointFormat,
    PointLabel, Series, SeriesName, TextStyle,
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
