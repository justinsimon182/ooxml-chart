//! Public-API tests for text and area styling, axis options, per-point
//! formats, trendlines, bubble and filled-radar charts, and combination
//! charts.

use ooxml_chart::{
    AreaStyle, Axis, ChartError, ChartKind, ChartPart, ChartSpec, DataLabelPosition, DataLabels,
    DateUnit, Plot, PointFormat, Series, SeriesName, TextStyle, TickLabels, TickMark, Trendline,
    TrendlineKind,
};

fn series() -> Series {
    Series::new(SeriesName::Literal("A".to_string()), "'S'!$C$3:$C$5")
        .with_categories("'S'!$A$3:$A$5")
}

fn render(spec: ChartSpec) -> String {
    let xml = String::from_utf8(spec.render().expect("a chart").xml).expect("UTF-8");
    assert_well_formed(&xml);
    xml
}

fn assert_well_formed(xml: &str) {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let end = rest.find('>').expect("unterminated tag");
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if tag.starts_with('?') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            assert_eq!(stack.pop(), Some(name), "mismatched close in {xml}");
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().unwrap());
        }
    }
    assert!(stack.is_empty(), "unclosed {stack:?} in {xml}");
}

fn refused(spec: ChartSpec) -> ChartError {
    spec.render().expect_err("should be refused")
}

fn column() -> ChartSpec {
    ChartSpec::new(ChartKind::ColumnClustered).series(series())
}

// --- text and areas ----------------------------------------------------------

#[test]
fn a_styled_title_carries_run_properties_in_schema_order() {
    let out = render(
        column().title("Sales").title_style(
            TextStyle::new()
                .size(14.0)
                .bold(true)
                .color("8e0dd1")
                .font("Inter"),
        ),
    );
    assert!(
        out.contains(r#"<a:r><a:rPr lang="en-US" sz="1400" b="1"><a:solidFill><a:srgbClr val="8E0DD1"/></a:solidFill><a:latin typeface="Inter"/></a:rPr><a:t>Sales</a:t></a:r>"#),
        "{out}"
    );
}

#[test]
fn an_unstyled_title_is_unchanged() {
    let out = render(column().title("Sales"));
    assert!(out.contains("<a:r><a:t>Sales</a:t></a:r>"), "{out}");
}

#[test]
fn the_chart_wide_text_style_follows_the_chart_element() {
    let out = render(column().text_style(TextStyle::new().size(9.0).font("Inter")));
    assert!(
        out.contains(r#"</c:chart><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="900"><a:latin typeface="Inter"/></a:defRPr></a:pPr><a:endParaRPr lang="en-US"/></a:p></c:txPr></c:chartSpace>"#),
        "{out}"
    );
}

#[test]
fn chart_area_sp_pr_precedes_the_chart_wide_tx_pr() {
    let out = render(
        column()
            .text_style(TextStyle::new().size(9.0))
            .chart_area(AreaStyle::new().fill("F8F6F0")),
    );
    assert!(
        out.find("</c:chart><c:spPr>").unwrap() < out.find("<c:txPr>").unwrap(),
        "{out}"
    );
}

#[test]
fn area_styles_write_fill_and_border() {
    let out = render(
        column()
            .chart_area(
                AreaStyle::new()
                    .fill("F8F6F0")
                    .border("010102")
                    .border_width(1.5),
            )
            .plot_area(AreaStyle::new().no_fill().no_border()),
    );
    assert!(
        out.contains(r#"</c:chart><c:spPr><a:solidFill><a:srgbClr val="F8F6F0"/></a:solidFill><a:ln w="19050"><a:solidFill><a:srgbClr val="010102"/></a:solidFill></a:ln></c:spPr>"#),
        "{out}"
    );
    // The plot area's spPr is the last thing in the plot area, after the axes.
    assert!(
        out.contains(
            r#"</c:valAx><c:spPr><a:noFill/><a:ln><a:noFill/></a:ln></c:spPr></c:plotArea>"#
        ),
        "{out}"
    );
}

#[test]
fn the_legend_can_overlay_and_be_styled() {
    let out = render(
        column()
            .legend_overlay(true)
            .legend_style(TextStyle::new().italic(true)),
    );
    assert!(
        out.contains(r#"<c:legend><c:legendPos val="r"/><c:overlay val="1"/><c:txPr>"#),
        "{out}"
    );
    assert!(out.contains(r#"<a:defRPr i="1"/>"#), "{out}");
}

#[test]
fn data_label_text_is_styled_before_the_position() {
    let out = render(
        column().data_labels(
            DataLabels::values()
                .at(DataLabelPosition::Center)
                .style(TextStyle::new().size(8.0)),
        ),
    );
    assert!(
        out.find("<c:txPr>").unwrap() < out.find("<c:dLblPos").unwrap(),
        "{out}"
    );
}

#[test]
fn a_bad_text_or_area_setting_is_refused() {
    for spec in [
        column().title_style(TextStyle::new().size(0.5)),
        column().title_style(TextStyle::new().size(5000.0)),
        column().title_style(TextStyle::new().size(f64::NAN)),
        column().text_style(TextStyle::new().color("red")),
        column().chart_area(AreaStyle::new().fill("#FFFFFF")),
        column().plot_area(AreaStyle::new().border_width(-1.0)),
    ] {
        assert!(matches!(
            refused(spec),
            ChartError::InvalidColor(_) | ChartError::OutOfRange { .. }
        ));
    }
}

// --- axes --------------------------------------------------------------------

#[test]
fn a_log_axis_writes_its_base_first_in_the_scaling() {
    let out = render(column().value_axis(Axis::default().log(10).min(1.0)));
    assert!(
        out.contains(r#"<c:scaling><c:logBase val="10"/><c:orientation val="minMax"/><c:min val="1"/></c:scaling>"#),
        "{out}"
    );
}

#[test]
fn a_log_axis_needs_a_valid_base_and_a_positive_minimum() {
    assert!(matches!(
        refused(column().value_axis(Axis::default().log(1))),
        ChartError::OutOfRange { .. }
    ));
    assert!(matches!(
        refused(column().value_axis(Axis::default().log(10).min(0.0))),
        ChartError::InvalidAxisRange { .. }
    ));
}

#[test]
fn axis_ticks_labels_and_gridlines_follow_the_schema_order() {
    let out = render(
        column().value_axis(
            Axis::default()
                .gridlines(true)
                .minor_gridlines(true)
                .title("V")
                .number_format("0")
                .major_tick(TickMark::Out)
                .minor_tick(TickMark::None)
                .tick_labels(TickLabels::Low)
                .label_rotation(-45)
                .label_style(TextStyle::new().size(8.0))
                .minor_unit(0.5)
                .crosses_max(true),
        ),
    );
    let order = [
        "<c:majorGridlines/>",
        "<c:minorGridlines/>",
        "<c:title>",
        "<c:numFmt",
        r#"<c:majorTickMark val="out"/>"#,
        r#"<c:minorTickMark val="none"/>"#,
        r#"<c:tickLblPos val="low"/>"#,
        r#"<a:bodyPr rot="-2700000" vert="horz"/>"#,
        "<c:crossAx",
        r#"<c:crosses val="max"/>"#,
        "<c:crossBetween",
        r#"<c:minorUnit val="0.5"/>"#,
    ];
    let val = out.find("<c:valAx>").unwrap();
    let mut last = val;
    for needle in order {
        let at = out[val..]
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} missing in {out}"))
            + val;
        assert!(at >= last, "{needle} out of order in {out}");
        last = at;
    }
}

#[test]
fn a_date_axis_replaces_the_category_axis() {
    let out = render(
        ChartSpec::new(ChartKind::Line)
            .series(series())
            .category_axis(
                Axis::default()
                    .dates(DateUnit::Months)
                    .number_format("mmm yy"),
            ),
    );
    assert!(out.contains("<c:dateAx>"), "{out}");
    assert!(!out.contains("<c:catAx>"), "{out}");
    assert!(
        out.contains(
            r#"<c:auto val="0"/><c:lblOffset val="100"/><c:baseTimeUnit val="months"/></c:dateAx>"#
        ),
        "{out}"
    );
}

#[test]
fn a_date_axis_on_a_scatter_is_refused() {
    let error = refused(
        ChartSpec::new(ChartKind::Scatter)
            .series(series())
            .category_axis(Axis::default().dates(DateUnit::Days)),
    );
    assert!(matches!(error, ChartError::Unsupported { .. }));
}

#[test]
fn a_label_rotation_beyond_ninety_degrees_is_refused() {
    assert!(matches!(
        refused(column().category_axis(Axis::default().label_rotation(91))),
        ChartError::OutOfRange { .. }
    ));
}

// --- points and trendlines ----------------------------------------------------

#[test]
fn pie_points_carry_colour_and_explosion_in_schema_order() {
    let out = render(
        ChartSpec::new(ChartKind::Pie).series(
            series()
                .with_point(1, PointFormat::new().color("28EAE4").explosion(15))
                .with_point(0, PointFormat::new().color("8E0DD1")),
        ),
    );
    let first = out.find(r#"<c:dPt><c:idx val="0"/>"#).unwrap();
    let second = out
        .find(r#"<c:dPt><c:idx val="1"/><c:explosion val="15"/><c:spPr>"#)
        .unwrap();
    assert!(first < second, "{out}");
    assert!(
        out.find("<c:dPt>").unwrap() < out.find("<c:cat>").unwrap(),
        "{out}"
    );
}

#[test]
fn a_later_point_format_replaces_an_earlier_one() {
    let out = render(
        ChartSpec::new(ChartKind::Pie).series(
            series()
                .with_point(0, PointFormat::new().color("111111"))
                .with_point(0, PointFormat::new().color("222222")),
        ),
    );
    assert_eq!(out.matches("<c:dPt>").count(), 1, "{out}");
    assert!(out.contains("222222") && !out.contains("111111"), "{out}");
}

#[test]
fn points_are_ignored_where_a_point_has_no_shape() {
    let out = render(
        ChartSpec::new(ChartKind::Area)
            .series(series().with_point(0, PointFormat::new().color("111111"))),
    );
    assert!(!out.contains("<c:dPt>"), "{out}");
}

#[test]
fn an_out_of_range_explosion_is_refused() {
    let error = refused(
        ChartSpec::new(ChartKind::Pie)
            .series(series().with_point(0, PointFormat::new().explosion(401))),
    );
    assert!(matches!(error, ChartError::OutOfRange { .. }));
}

#[test]
fn a_linear_trendline_sits_between_the_points_and_the_categories() {
    let out = render(
        ChartSpec::new(ChartKind::Line).series(
            series().with_trendline(
                Trendline::new(TrendlineKind::Linear)
                    .name("Trend")
                    .color("104991")
                    .width(1.5)
                    .forward(2.0)
                    .show_equation()
                    .show_r_squared(),
            ),
        ),
    );
    assert!(
        out.contains(r#"<c:trendline><c:name>Trend</c:name><c:spPr><a:ln w="19050" cap="rnd"><a:solidFill><a:srgbClr val="104991"/></a:solidFill></a:ln></c:spPr><c:trendlineType val="linear"/><c:forward val="2"/><c:dispRSqr val="1"/><c:dispEq val="1"/></c:trendline><c:cat>"#),
        "{out}"
    );
}

#[test]
fn polynomial_and_moving_average_trendlines_carry_their_parameter() {
    let poly = render(
        ChartSpec::new(ChartKind::Scatter)
            .series(series().with_trendline(Trendline::new(TrendlineKind::Polynomial(3)))),
    );
    assert!(
        poly.contains(r#"<c:trendlineType val="poly"/><c:order val="3"/>"#),
        "{poly}"
    );
    let average = render(
        ChartSpec::new(ChartKind::Line)
            .series(series().with_trendline(Trendline::new(TrendlineKind::MovingAverage(4)))),
    );
    assert!(
        average.contains(r#"<c:trendlineType val="movingAvg"/><c:period val="4"/>"#),
        "{average}"
    );
}

#[test]
fn a_trendline_where_excel_forbids_one_is_refused() {
    let trend = || series().with_trendline(Trendline::new(TrendlineKind::Linear));
    for kind in [
        ChartKind::ColumnStacked,
        ChartKind::BarPercentStacked,
        ChartKind::AreaStacked,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Radar,
    ] {
        assert!(
            matches!(
                refused(ChartSpec::new(kind).series(trend())),
                ChartError::Unsupported { .. }
            ),
            "{kind:?}"
        );
    }
}

#[test]
fn a_trendline_parameter_out_of_range_is_refused() {
    for kind in [
        TrendlineKind::Polynomial(1),
        TrendlineKind::Polynomial(7),
        TrendlineKind::MovingAverage(1),
    ] {
        let error = refused(
            ChartSpec::new(ChartKind::Line).series(series().with_trendline(Trendline::new(kind))),
        );
        assert!(matches!(error, ChartError::OutOfRange { .. }), "{kind:?}");
    }
}

// --- bubble and filled radar -----------------------------------------------------

#[test]
fn a_bubble_series_carries_x_y_and_sizes() {
    let out = render(
        ChartSpec::new(ChartKind::Bubble)
            .series(
                series()
                    .with_bubble_sizes("'S'!$D$3:$D$5")
                    .with_color("8E0DD1"),
            )
            .bubble_scale(80),
    );
    assert!(
        out.contains("<c:bubbleChart><c:varyColors val=\"0\"/>"),
        "{out}"
    );
    assert!(
        out.contains(r#"</c:yVal><c:bubbleSize><c:numRef><c:f>&#39;S&#39;!$D$3:$D$5</c:f></c:numRef></c:bubbleSize><c:bubble3D val="0"/></c:ser>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<c:bubbleScale val="80"/><c:showNegBubbles val="0"/><c:axId"#),
        "{out}"
    );
    assert_eq!(out.matches("<c:valAx>").count(), 2, "{out}");
}

#[test]
fn a_bubble_without_sizes_is_refused() {
    assert!(matches!(
        refused(ChartSpec::new(ChartKind::Bubble).series(series())),
        ChartError::MissingBubbleSizes
    ));
}

#[test]
fn a_filled_radar_is_filled_and_has_no_line_markers() {
    let out = render(ChartSpec::new(ChartKind::RadarFilled).series(series().with_color("0090B2")));
    assert!(out.contains(r#"<c:radarStyle val="filled"/>"#), "{out}");
    assert!(
        out.contains(r#"<c:spPr><a:solidFill><a:srgbClr val="0090B2"/>"#),
        "{out}"
    );
    assert!(!out.contains("<c:marker>"), "{out}");
}

// --- combination charts ----------------------------------------------------------

fn combo() -> ChartSpec {
    column().plot(
        Plot::new(ChartKind::LineMarkers)
            .series(Series::new(
                SeriesName::Literal("Rate".into()),
                "'S'!$D$3:$D$5",
            ))
            .on_secondary_axis(),
    )
}

#[test]
fn a_combo_writes_both_plots_then_all_four_axes() {
    let out = render(combo());
    let bar = out.find("<c:barChart>").unwrap();
    let line = out.find("<c:lineChart>").unwrap();
    let axes = out.find("<c:catAx>").unwrap();
    assert!(bar < line && line < axes, "{out}");
    assert_eq!(out.matches("<c:catAx>").count(), 2, "{out}");
    assert_eq!(out.matches("<c:valAx>").count(), 2, "{out}");
}

#[test]
fn the_secondary_plot_uses_the_secondary_axis_ids() {
    let out = render(combo());
    let line = &out[out.find("<c:lineChart>").unwrap()..out.find("</c:lineChart>").unwrap()];
    assert!(
        line.contains(r#"<c:axId val="333"/><c:axId val="444"/>"#),
        "{out}"
    );
    let bar = &out[out.find("<c:barChart>").unwrap()..out.find("</c:barChart>").unwrap()];
    assert!(
        bar.contains(r#"<c:axId val="111"/><c:axId val="222"/>"#),
        "{out}"
    );
}

#[test]
fn the_secondary_value_axis_sits_on_the_right_and_its_category_axis_is_hidden() {
    let out =
        render(combo().secondary_value_axis(Axis::default().title("Rate").number_format("0%")));
    let secondary = &out[out.find(r#"<c:valAx><c:axId val="444"/>"#).unwrap()..];
    let secondary = &secondary[..secondary.find("</c:valAx>").unwrap()];
    assert!(secondary.contains(r#"<c:axPos val="r"/>"#), "{out}");
    assert!(
        secondary.contains(r#"<c:crossAx val="333"/><c:crosses val="max"/>"#),
        "{out}"
    );
    assert!(secondary.contains("<a:t>Rate</a:t>"), "{out}");
    let hidden = &out[out.find(r#"<c:catAx><c:axId val="333"/>"#).unwrap()..];
    assert!(
        hidden[..hidden.find("</c:catAx>").unwrap()].contains(r#"<c:delete val="1"/>"#),
        "{out}"
    );
}

#[test]
fn series_are_numbered_across_plots() {
    let out = render(combo());
    assert!(
        out.contains(r#"<c:idx val="0"/><c:order val="0"/>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<c:idx val="1"/><c:order val="1"/>"#),
        "{out}"
    );
}

#[test]
fn a_plot_on_the_primary_axes_adds_no_secondary_axes() {
    let out = render(column().plot(Plot::new(ChartKind::Line).series(series())));
    assert_eq!(out.matches("<c:valAx>").count(), 1, "{out}");
    assert!(!out.contains(r#"val="444""#), "{out}");
}

#[test]
fn combining_kinds_that_cannot_share_a_category_axis_is_refused() {
    for spec in [
        ChartSpec::new(ChartKind::Pie)
            .series(series())
            .plot(Plot::new(ChartKind::Line).series(series())),
        column().plot(Plot::new(ChartKind::Scatter).series(series())),
        column().plot(Plot::new(ChartKind::BarClustered).series(series())),
        ChartSpec::new(ChartKind::BarClustered)
            .series(series())
            .plot(Plot::new(ChartKind::Line).series(series())),
    ] {
        assert!(matches!(refused(spec), ChartError::Unsupported { .. }));
    }
}

#[test]
fn a_plot_with_no_series_is_refused() {
    assert!(matches!(
        refused(column().plot(Plot::new(ChartKind::Line))),
        ChartError::NoSeries
    ));
}

#[test]
fn data_labels_are_checked_against_every_plot() {
    // Outside-end is fine on the columns and illegal on the line.
    let error =
        refused(combo().data_labels(DataLabels::values().at(DataLabelPosition::OutsideEnd)));
    assert!(matches!(error, ChartError::InvalidDataLabelPosition { .. }));
    render(combo().data_labels(DataLabels::values().at(DataLabelPosition::Center)));
}

// --- package helpers -----------------------------------------------------------------

#[test]
fn a_content_types_entry_names_the_part_and_its_type() {
    assert_eq!(
        ChartPart::content_types_override("/xl/charts/chart1.xml"),
        format!(
            r#"<Override PartName="/xl/charts/chart1.xml" ContentType="{}"/>"#,
            ChartPart::CONTENT_TYPE
        )
    );
}

#[test]
fn control_characters_in_text_do_not_reach_the_xml() {
    let out = render(column().title("Sales\u{0}\u{8} Q1"));
    assert!(out.contains("<a:t>Sales Q1</a:t>"), "{out}");
}

// --- Per-point markers and data labels ----------------------------------------

mod point_overrides {
    use super::*;
    use ooxml_chart::{MarkerSymbol, PointLabel};

    fn line() -> ChartSpec {
        ChartSpec::new(ChartKind::LineMarkers).series(series())
    }

    #[test]
    fn a_point_marker_is_written_as_a_dpt_on_a_line_series() {
        let xml = render(
            ChartSpec::new(ChartKind::LineMarkers).series(
                series()
                    .with_marker(MarkerSymbol::Circle, 6)
                    .with_point(2, PointFormat::new().marker(MarkerSymbol::Diamond, 10)),
            ),
        );
        assert!(
            xml.contains(r#"<c:dPt><c:idx val="2"/><c:marker><c:symbol val="diamond"/><c:size val="10"/></c:marker></c:dPt>"#),
            "{xml}"
        );
        // After the series marker, before the categories.
        let series_marker = xml.find(r#"<c:symbol val="circle"/>"#).expect("marker");
        let dpt = xml.find("<c:dPt>").expect("dPt");
        let cat = xml.find("<c:cat>").expect("cat");
        assert!(series_marker < dpt && dpt < cat, "{xml}");
    }

    #[test]
    fn a_point_colour_on_a_line_series_recolours_its_marker() {
        let xml = render(
            ChartSpec::new(ChartKind::Scatter)
                .series(series().with_point(0, PointFormat::new().color("8e0dd1"))),
        );
        assert!(
            xml.contains(
                r#"<c:dPt><c:idx val="0"/><c:marker><c:spPr><a:solidFill><a:srgbClr val="8E0DD1"/>"#
            ),
            "{xml}"
        );
        assert!(!xml.contains("<c:symbol"), "no symbol was asked for: {xml}");
    }

    #[test]
    fn a_point_marker_on_a_column_is_ignored() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(series().with_point(1, PointFormat::new().marker(MarkerSymbol::Star, 8))),
        );
        assert!(!xml.contains("<c:dPt>"), "{xml}");
    }

    #[test]
    fn a_point_marker_size_out_of_range_is_refused() {
        let error = line()
            .series(series().with_point(0, PointFormat::new().marker(MarkerSymbol::Circle, 1)))
            .render();
        assert!(
            matches!(error, Err(ChartError::OutOfRange { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn a_hidden_point_label_writes_its_own_dlbls_with_labels_off_for_the_rest() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(series().with_point_label(1, PointLabel::hidden())),
        );
        assert!(
            xml.contains(r#"<c:dLbls><c:dLbl><c:idx val="1"/><c:delete val="1"/></c:dLbl><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/></c:dLbls>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_series_without_overrides_writes_no_series_level_labels() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered)
                .data_labels(DataLabels::values())
                .series(series()),
        );
        assert_eq!(xml.matches("<c:dLbls>").count(), 1, "{xml}");
    }

    #[test]
    fn a_text_label_is_escaped_and_shows_the_value_flag() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(series().with_point_label(0, PointLabel::text("Peak & <best>"))),
        );
        assert!(
            xml.contains("<c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Peak &amp; &lt;best&gt;</a:t></a:r></a:p></c:rich></c:tx>"),
            "{xml}"
        );
        assert!(
            xml.contains(r#"<c:dLbl><c:idx val="0"/><c:tx>"#) && xml.contains(r#"<c:showVal val="1"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/></c:dLbl>"#),
            "{xml}"
        );
    }

    #[test]
    fn an_overridden_point_inherits_the_chart_wide_labels() {
        let xml =
            render(
                ChartSpec::new(ChartKind::ColumnClustered)
                    .data_labels(
                        DataLabels::values()
                            .with_category()
                            .number_format("0.0")
                            .at(DataLabelPosition::InsideEnd),
                    )
                    .series(series().with_point_label(
                        2,
                        PointLabel::text("x").at(DataLabelPosition::OutsideEnd),
                    )),
            );
        // The override keeps the number format and what to show, but moves.
        assert!(
            xml.contains(r#"<c:numFmt formatCode="0.0" sourceLinked="0"/><c:dLblPos val="outEnd"/><c:showLegendKey val="0"/><c:showVal val="1"/><c:showCatName val="1"/>"#),
            "{xml}"
        );
        // The group after the overrides repeats the chart-wide settings.
        assert!(
            xml.contains(r#"</c:dLbl><c:numFmt formatCode="0.0""#),
            "{xml}"
        );
        assert!(xml.contains(r#"<c:dLblPos val="inEnd"/>"#), "{xml}");
    }

    #[test]
    fn a_label_style_moves_onto_the_run_for_custom_text() {
        let xml = render(ChartSpec::new(ChartKind::ColumnClustered).series(
            series().with_point_label(
                0,
                PointLabel::text("Top").style(TextStyle::new().bold(true).color("8E0DD1")),
            ),
        ));
        assert!(xml.contains(r#"<a:rPr lang="en-US" b="1">"#), "{xml}");
        assert!(!xml.contains("<c:txPr>"), "{xml}");
    }

    #[test]
    fn labels_come_after_points_and_before_trendlines() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered).series(
                series()
                    .with_point(0, PointFormat::new().color("0090B2"))
                    .with_point_label(0, PointLabel::hidden())
                    .with_trendline(Trendline::new(TrendlineKind::Linear)),
            ),
        );
        let dpt = xml.find("<c:dPt>").expect("dPt");
        let dlbls = xml.find("<c:dLbls>").expect("dLbls");
        let trend = xml.find("<c:trendline>").expect("trendline");
        assert!(dpt < dlbls && dlbls < trend, "{xml}");
    }

    #[test]
    fn points_are_written_in_index_order_whatever_order_they_were_added() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered).series(
                series()
                    .with_point_label(3, PointLabel::hidden())
                    .with_point_label(1, PointLabel::hidden()),
            ),
        );
        assert!(
            xml.find(r#"<c:idx val="1"/><c:delete"#) < xml.find(r#"<c:idx val="3"/><c:delete"#),
            "{xml}"
        );
    }

    #[test]
    fn a_later_label_for_the_same_point_replaces_the_earlier() {
        let xml = render(
            ChartSpec::new(ChartKind::ColumnClustered).series(
                series()
                    .with_point_label(0, PointLabel::hidden())
                    .with_point_label(0, PointLabel::text("kept")),
            ),
        );
        assert!(
            xml.contains("kept") && !xml.contains(r#"<c:delete val="1"/>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_label_position_the_kind_does_not_allow_is_refused() {
        let error = ChartSpec::new(ChartKind::Area)
            .series(
                series().with_point_label(0, PointLabel::text("x").at(DataLabelPosition::Above)),
            )
            .render();
        assert!(
            matches!(error, Err(ChartError::InvalidDataLabelPosition { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn a_label_that_is_both_hidden_and_has_text_is_refused() {
        let mut label = PointLabel::hidden();
        label.text = Some("x".to_string());
        let error = ChartSpec::new(ChartKind::ColumnClustered)
            .series(series().with_point_label(0, label))
            .render();
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }
}

// --- Error bars ----------------------------------------------------------------

mod error_bars {
    use super::*;
    use ooxml_chart::{ErrorAmount, ErrorBarSide, ErrorBars, ErrorValues};

    fn with(kind: ChartKind, bars: ErrorBars) -> Result<String, ChartError> {
        ChartSpec::new(kind)
            .series(series().with_error_bars(bars))
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    fn ok(kind: ChartKind, bars: ErrorBars) -> String {
        let xml = with(kind, bars).expect("a chart");
        assert_well_formed(&xml);
        xml
    }

    #[test]
    fn a_column_writes_no_direction() {
        let xml = ok(
            ChartKind::ColumnClustered,
            ErrorBars::new(ErrorAmount::Fixed(5.0)),
        );
        assert!(
            xml.contains(r#"<c:errBars><c:errBarType val="both"/><c:errValType val="fixedVal"/><c:noEndCap val="0"/><c:val val="5"/></c:errBars>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_line_writes_a_y_direction() {
        let xml = ok(
            ChartKind::Line,
            ErrorBars::new(ErrorAmount::Percentage(10.0)).side(ErrorBarSide::Plus),
        );
        assert!(
            xml.contains(r#"<c:errBars><c:errDir val="y"/><c:errBarType val="plus"/><c:errValType val="percentage"/><c:noEndCap val="0"/><c:val val="10"/></c:errBars>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_scatter_takes_both_directions_x_first() {
        let xml = ChartSpec::new(ChartKind::Scatter)
            .series(
                series()
                    .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(1.0)))
                    .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(2.0)).along_x()),
            )
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
            .expect("a chart");
        assert_well_formed(&xml);
        let x = xml.find(r#"<c:errDir val="x"/>"#).expect("x bars");
        let y = xml.find(r#"<c:errDir val="y"/>"#).expect("y bars");
        assert!(x < y, "{xml}");
        assert!(xml.find("<c:errBars>") < xml.find("<c:xVal>"), "{xml}");
    }

    #[test]
    fn standard_deviation_and_error_carry_the_right_fields() {
        let sd = ok(ChartKind::Line, ErrorBars::new(ErrorAmount::StdDev(2.0)));
        assert!(
            sd.contains(r#"<c:errValType val="stdDev"/><c:noEndCap val="0"/><c:val val="2"/>"#),
            "{sd}"
        );
        let se = ok(ChartKind::Line, ErrorBars::new(ErrorAmount::StdErr));
        assert!(
            se.contains(r#"<c:errValType val="stdErr"/><c:noEndCap val="0"/></c:errBars>"#),
            "{se}"
        );
    }

    #[test]
    fn custom_amounts_can_be_references_or_literals() {
        let xml = ok(
            ChartKind::ColumnClustered,
            ErrorBars::new(ErrorAmount::Custom {
                plus: Some(ErrorValues::Reference("'S'!$D$3:$D$5".into())),
                minus: Some(ErrorValues::Literal(vec![0.5, 1.0])),
            }),
        );
        assert!(
            xml.contains(r#"<c:errValType val="cust"/><c:noEndCap val="0"/><c:plus><c:numRef><c:f>&#39;S&#39;!$D$3:$D$5</c:f></c:numRef></c:plus><c:minus><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="2"/><c:pt idx="0"><c:v>0.5</c:v></c:pt><c:pt idx="1"><c:v>1</c:v></c:pt></c:numLit></c:minus>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_one_sided_custom_bar_needs_only_its_own_side() {
        let xml = ok(
            ChartKind::ColumnClustered,
            ErrorBars::new(ErrorAmount::Custom {
                plus: Some(ErrorValues::Reference("S!$D$3:$D$5".into())),
                minus: None,
            })
            .side(ErrorBarSide::Plus),
        );
        assert!(
            xml.contains("<c:plus>") && !xml.contains("<c:minus>"),
            "{xml}"
        );
    }

    #[test]
    fn a_custom_bar_missing_a_side_it_draws_is_refused() {
        let error = with(
            ChartKind::ColumnClustered,
            ErrorBars::new(ErrorAmount::Custom {
                plus: Some(ErrorValues::Literal(vec![1.0])),
                minus: None,
            }),
        );
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn caps_colour_and_width_are_written() {
        let xml = ok(
            ChartKind::Line,
            ErrorBars::new(ErrorAmount::Fixed(1.0))
                .end_cap(false)
                .color("8e0dd1")
                .width(1.5),
        );
        assert!(xml.contains(r#"<c:noEndCap val="1"/>"#), "{xml}");
        assert!(
            xml.contains(r#"<c:val val="1"/><c:spPr><a:ln w="19050"><a:solidFill><a:srgbClr val="8E0DD1"/></a:solidFill></a:ln></c:spPr></c:errBars>"#),
            "{xml}"
        );
    }

    #[test]
    fn error_bars_come_after_trendlines_and_before_the_data() {
        let xml = ok_series(
            ChartKind::ColumnClustered,
            series()
                .with_trendline(Trendline::new(TrendlineKind::Linear))
                .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(1.0))),
        );
        let trend = xml.find("<c:trendline>").expect("trendline");
        let bars = xml.find("<c:errBars>").expect("errBars");
        let cat = xml.find("<c:cat>").expect("cat");
        assert!(trend < bars && bars < cat, "{xml}");
    }

    fn ok_series(kind: ChartKind, series: Series) -> String {
        let xml = String::from_utf8(
            ChartSpec::new(kind)
                .series(series)
                .render()
                .expect("a chart")
                .xml,
        )
        .expect("UTF-8");
        assert_well_formed(&xml);
        xml
    }

    #[test]
    fn kinds_excel_draws_no_error_bars_on_are_refused() {
        for kind in [ChartKind::Pie, ChartKind::Doughnut, ChartKind::Radar] {
            let error = with(kind, ErrorBars::new(ErrorAmount::Fixed(1.0)));
            assert!(
                matches!(error, Err(ChartError::Unsupported { .. })),
                "{kind:?}: {error:?}"
            );
        }
    }

    #[test]
    fn an_x_bar_off_a_scatter_is_refused() {
        let error = with(
            ChartKind::ColumnClustered,
            ErrorBars::new(ErrorAmount::Fixed(1.0)).along_x(),
        );
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn two_bars_in_one_direction_are_refused() {
        let error = ChartSpec::new(ChartKind::Scatter)
            .series(
                series()
                    .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(1.0)))
                    .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(2.0))),
            )
            .render();
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn nonsense_amounts_and_styles_are_refused() {
        for amount in [
            ErrorAmount::Fixed(-1.0),
            ErrorAmount::Fixed(f64::NAN),
            ErrorAmount::Percentage(f64::INFINITY),
            ErrorAmount::StdDev(0.0),
            ErrorAmount::Custom {
                plus: Some(ErrorValues::Literal(vec![])),
                minus: Some(ErrorValues::Literal(vec![1.0])),
            },
        ] {
            let error = with(ChartKind::Line, ErrorBars::new(amount.clone()));
            assert!(
                matches!(error, Err(ChartError::OutOfRange { .. })),
                "{amount:?}: {error:?}"
            );
        }
        let error = with(
            ChartKind::Line,
            ErrorBars::new(ErrorAmount::Fixed(1.0)).color("nope"),
        );
        assert!(
            matches!(error, Err(ChartError::InvalidColor { .. })),
            "{error:?}"
        );
    }
}

// --- Manual layout --------------------------------------------------------------

mod manual_layout {
    use super::*;
    use ooxml_chart::{Layout, LegendPosition};

    fn base() -> ChartSpec {
        ChartSpec::new(ChartKind::ColumnClustered).series(series())
    }

    #[test]
    fn without_a_layout_the_plot_area_keeps_its_empty_layout_and_the_legend_none() {
        let xml = render(base());
        assert!(xml.contains("<c:plotArea><c:layout/>"), "{xml}");
        assert!(!xml.contains("<c:manualLayout>"), "{xml}");
    }

    #[test]
    fn a_plot_area_layout_is_written_as_an_inner_edge_rectangle() {
        let xml = render(base().plot_area_layout(Layout::new(0.1, 0.15, 0.7, 0.65)));
        assert!(
            xml.contains(r#"<c:plotArea><c:layout><c:manualLayout><c:layoutTarget val="inner"/><c:xMode val="edge"/><c:yMode val="edge"/><c:x val="0.1"/><c:y val="0.15"/><c:w val="0.7"/><c:h val="0.65"/></c:manualLayout></c:layout>"#),
            "{xml}"
        );
    }

    #[test]
    fn an_outer_plot_area_layout_says_so() {
        let xml = render(base().plot_area_layout(Layout::new(0.0, 0.0, 1.0, 1.0).outer()));
        assert!(xml.contains(r#"<c:layoutTarget val="outer"/>"#), "{xml}");
    }

    #[test]
    fn a_legend_layout_sits_between_the_position_and_the_overlay_with_no_target() {
        let xml = render(
            base()
                .legend(LegendPosition::Right)
                .legend_layout(Layout::new(0.8, 0.3, 0.15, 0.3)),
        );
        assert!(
            xml.contains(r#"<c:legend><c:legendPos val="r"/><c:layout><c:manualLayout><c:xMode val="edge"/><c:yMode val="edge"/><c:x val="0.8"/><c:y val="0.3"/><c:w val="0.15"/><c:h val="0.3"/></c:manualLayout></c:layout><c:overlay val="0"/>"#),
            "{xml}"
        );
        assert!(
            !xml.contains(
                "<c:legend><c:legendPos val=\"r\"/><c:layout><c:manualLayout><c:layoutTarget"
            ),
            "{xml}"
        );
    }

    #[test]
    fn a_rectangle_that_only_just_fits_in_floating_point_is_accepted() {
        // Sums like this can land a hair over 1.0 in floating point; a strict
        // compare would refuse a rectangle that plainly fits.
        render(base().plot_area_layout(Layout::new(0.1, 0.1, 0.9, 0.9)));
    }

    #[test]
    fn layouts_that_do_not_fit_the_chart_are_refused() {
        for layout in [
            Layout::new(f64::NAN, 0.0, 0.5, 0.5),
            Layout::new(-0.1, 0.0, 0.5, 0.5),
            Layout::new(0.0, 1.5, 0.5, 0.5),
            Layout::new(0.0, 0.0, 0.0, 0.5),
            Layout::new(0.0, 0.0, 0.5, -1.0),
            Layout::new(0.6, 0.0, 0.5, 0.5),
            Layout::new(0.0, 0.6, 0.5, 0.5),
        ] {
            let error = base().plot_area_layout(layout).render();
            assert!(
                matches!(error, Err(ChartError::InvalidLayout { .. })),
                "{layout:?}: {error:?}"
            );
            let error = base().legend_layout(layout).render();
            assert!(
                matches!(error, Err(ChartError::InvalidLayout { .. })),
                "{layout:?}: {error:?}"
            );
        }
    }

    #[test]
    fn a_legend_layout_with_no_legend_is_refused() {
        let error = base()
            .legend(LegendPosition::None)
            .legend_layout(Layout::new(0.1, 0.1, 0.2, 0.2))
            .render();
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn a_pie_takes_a_plot_area_layout_too() {
        let xml = render(
            ChartSpec::new(ChartKind::Pie)
                .series(series())
                .plot_area_layout(Layout::new(0.2, 0.2, 0.6, 0.6)),
        );
        assert!(xml.contains("<c:manualLayout>"), "{xml}");
    }
}

// --- Date-axis tick units ---------------------------------------------------------

mod date_ticks {
    use super::*;

    fn dated(axis: Axis) -> Result<String, ChartError> {
        ChartSpec::new(ChartKind::Line)
            .category_axis(axis)
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    fn reason(result: Result<String, ChartError>) -> &'static str {
        match result {
            Err(ChartError::InvalidAxisRange { reason }) => reason,
            other => panic!("expected InvalidAxisRange, got {other:?}"),
        }
    }

    #[test]
    fn major_and_minor_units_follow_the_base_unit_in_schema_order() {
        let xml = dated(
            Axis::default()
                .dates(DateUnit::Days)
                .date_major(3, DateUnit::Months)
                .date_minor(1, DateUnit::Months),
        )
        .expect("a chart");
        assert_well_formed(&xml);
        assert!(
            xml.contains(r#"<c:baseTimeUnit val="days"/><c:majorUnit val="3"/><c:majorTimeUnit val="months"/><c:minorUnit val="1"/><c:minorTimeUnit val="months"/></c:dateAx>"#),
            "{xml}"
        );
    }

    #[test]
    fn only_a_major_unit_writes_only_its_pair() {
        let xml = dated(
            Axis::default()
                .dates(DateUnit::Months)
                .date_major(1, DateUnit::Years),
        )
        .expect("a chart");
        assert!(
            xml.contains(r#"<c:majorUnit val="1"/><c:majorTimeUnit val="years"/></c:dateAx>"#),
            "{xml}"
        );
        assert!(!xml.contains("minorUnit"), "{xml}");
    }

    #[test]
    fn a_date_axis_without_units_writes_none() {
        let xml = dated(Axis::default().dates(DateUnit::Days)).expect("a chart");
        assert!(
            xml.contains(r#"<c:baseTimeUnit val="days"/></c:dateAx>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_unit_the_same_size_as_the_base_is_allowed() {
        dated(
            Axis::default()
                .dates(DateUnit::Months)
                .date_major(2, DateUnit::Months),
        )
        .expect("a chart");
    }

    #[test]
    fn a_unit_finer_than_the_base_is_refused() {
        let why = reason(dated(
            Axis::default()
                .dates(DateUnit::Months)
                .date_major(7, DateUnit::Days),
        ));
        assert!(why.contains("finer"), "{why}");
        let why = reason(dated(
            Axis::default()
                .dates(DateUnit::Years)
                .date_minor(1, DateUnit::Months),
        ));
        assert!(why.contains("finer"), "{why}");
    }

    #[test]
    fn a_fractional_count_is_refused() {
        let why = reason(dated(
            Axis::default()
                .dates(DateUnit::Days)
                .major_unit(1.5)
                .dates(DateUnit::Days),
        ));
        assert!(why.contains("whole"), "{why}");
    }

    #[test]
    fn time_units_on_a_plain_category_axis_are_refused() {
        let why = reason(dated(Axis::default().date_major(1, DateUnit::Months)));
        assert!(why.contains("not a date axis"), "{why}");
    }

    #[test]
    fn a_time_unit_with_nothing_to_count_is_refused() {
        let mut axis = Axis::default().dates(DateUnit::Days);
        axis.major_time_unit = Some(DateUnit::Months);
        let why = reason(dated(axis));
        assert!(
            why.contains("nothing") || why.contains("no tick unit"),
            "{why}"
        );
    }

    #[test]
    fn a_zero_count_is_still_refused_as_not_positive() {
        let why = reason(dated(
            Axis::default()
                .dates(DateUnit::Days)
                .date_major(0, DateUnit::Days),
        ));
        assert!(why.contains("positive"), "{why}");
    }
}

// --- crossesAt --------------------------------------------------------------------

mod crosses_at {
    use super::*;

    fn chart(category: Axis, value: Axis) -> Result<String, ChartError> {
        ChartSpec::new(ChartKind::ColumnClustered)
            .category_axis(category)
            .value_axis(value)
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    fn axis_xml<'a>(xml: &'a str, tag: &str) -> &'a str {
        let start = xml.find(&format!("<c:{tag}>")).expect("axis");
        let end = xml[start..].find(&format!("</c:{tag}>")).expect("axis end");
        &xml[start..start + end]
    }

    #[test]
    fn a_category_axis_crossing_at_a_value_writes_crosses_at_after_crossax() {
        let xml = chart(Axis::default().crosses_at(50.0), Axis::default()).expect("a chart");
        assert_well_formed(&xml);
        let cat = axis_xml(&xml, "catAx");
        assert!(cat.ends_with(r#"<c:crossesAt val="50"/>"#), "{cat}");
        assert!(
            cat.contains(r#"<c:crossAx val="#)
                && cat.find("<c:crossAx").unwrap() < cat.find("<c:crossesAt").unwrap(),
            "{cat}"
        );
        assert!(!axis_xml(&xml, "valAx").contains("crossesAt"), "{xml}");
    }

    #[test]
    fn a_fractional_and_a_negative_value_are_written_as_given() {
        let xml = chart(Axis::default(), Axis::default().crosses_at(-2.5)).expect("a chart");
        assert!(
            axis_xml(&xml, "valAx").contains(r#"<c:crossesAt val="-2.5"/>"#),
            "{xml}"
        );
    }

    #[test]
    fn crosses_at_replaces_crosses_max_on_the_secondary_axis() {
        let xml = ChartSpec::new(ChartKind::ColumnClustered)
            .series(series())
            .plot(
                Plot::new(ChartKind::Line)
                    .series(series())
                    .on_secondary_axis(),
            )
            .secondary_value_axis(Axis::default().crosses_at(3.0))
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
            .expect("a chart");
        assert!(xml.contains(r#"<c:crossesAt val="3"/>"#), "{xml}");
        assert_eq!(xml.matches(r#"<c:crosses val="max"/>"#).count(), 0, "{xml}");
    }

    #[test]
    fn an_axis_cannot_cross_at_a_value_and_the_maximum() {
        let error = chart(
            Axis::default().crosses_at(1.0).crosses_max(true),
            Axis::default(),
        );
        assert!(
            matches!(error, Err(ChartError::InvalidAxisRange { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn a_non_finite_crossing_is_refused() {
        let error = chart(Axis::default(), Axis::default().crosses_at(f64::NAN));
        assert!(
            matches!(error, Err(ChartError::InvalidAxisRange { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn crossing_a_log_axis_at_zero_or_below_is_refused_but_above_is_fine() {
        let log = || Axis::default().log(10).min(1.0);
        let error = chart(Axis::default().crosses_at(0.0), log());
        assert!(
            matches!(error, Err(ChartError::InvalidAxisRange { .. })),
            "{error:?}"
        );
        let xml = chart(Axis::default().crosses_at(10.0), log()).expect("a chart");
        assert!(xml.contains(r#"<c:crossesAt val="10"/>"#), "{xml}");
    }
}

// --- Display units ---------------------------------------------------------------

mod display_units {
    use super::*;
    use ooxml_chart::DisplayUnit;

    fn chart(kind: ChartKind, category: Axis, value: Axis) -> Result<String, ChartError> {
        ChartSpec::new(kind)
            .category_axis(category)
            .value_axis(value)
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    #[test]
    fn a_built_in_unit_follows_the_tick_units_and_ends_the_axis() {
        let xml = chart(
            ChartKind::ColumnClustered,
            Axis::default(),
            Axis::default()
                .major_unit(5.0)
                .display_units(DisplayUnit::Thousands),
        )
        .expect("a chart");
        assert_well_formed(&xml);
        assert!(
            xml.contains(r#"<c:crossBetween val="between"/><c:majorUnit val="5"/><c:dispUnits><c:builtInUnit val="thousands"/></c:dispUnits></c:valAx>"#),
            "{xml}"
        );
    }

    #[test]
    fn every_built_in_unit_has_its_schema_name() {
        for (unit, name) in [
            (DisplayUnit::Hundreds, "hundreds"),
            (DisplayUnit::TenThousands, "tenThousands"),
            (DisplayUnit::HundredThousands, "hundredThousands"),
            (DisplayUnit::Millions, "millions"),
            (DisplayUnit::TenMillions, "tenMillions"),
            (DisplayUnit::HundredMillions, "hundredMillions"),
            (DisplayUnit::Billions, "billions"),
            (DisplayUnit::Trillions, "trillions"),
        ] {
            let xml = chart(
                ChartKind::Line,
                Axis::default(),
                Axis::default().display_units(unit),
            )
            .expect("a chart");
            assert!(
                xml.contains(&format!(r#"<c:builtInUnit val="{name}"/>"#)),
                "{name}: {xml}"
            );
        }
    }

    #[test]
    fn a_custom_divisor_and_the_caption_are_written() {
        let xml = chart(
            ChartKind::Line,
            Axis::default(),
            Axis::default()
                .display_units(DisplayUnit::Custom(250.0))
                .display_units_label(true),
        )
        .expect("a chart");
        assert!(
            xml.contains(r#"<c:dispUnits><c:custUnit val="250"/><c:dispUnitsLbl/></c:dispUnits>"#),
            "{xml}"
        );
    }

    #[test]
    fn a_scatter_x_axis_takes_display_units() {
        let xml = chart(
            ChartKind::Scatter,
            Axis::default().display_units(DisplayUnit::Millions),
            Axis::default(),
        )
        .expect("a chart");
        assert!(xml.contains(r#"<c:builtInUnit val="millions"/>"#), "{xml}");
    }

    #[test]
    fn a_category_axis_cannot() {
        let error = chart(
            ChartKind::ColumnClustered,
            Axis::default().display_units(DisplayUnit::Thousands),
            Axis::default(),
        );
        assert!(
            matches!(error, Err(ChartError::Unsupported { .. })),
            "{error:?}"
        );
    }

    #[test]
    fn a_bad_divisor_or_an_orphan_caption_is_refused() {
        for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let error = chart(
                ChartKind::Line,
                Axis::default(),
                Axis::default().display_units(DisplayUnit::Custom(bad)),
            );
            assert!(
                matches!(error, Err(ChartError::InvalidAxisRange { .. })),
                "{bad}: {error:?}"
            );
        }
        let error = chart(
            ChartKind::Line,
            Axis::default(),
            Axis::default().display_units_label(true),
        );
        assert!(
            matches!(error, Err(ChartError::InvalidAxisRange { .. })),
            "{error:?}"
        );
    }
}

// --- Title positions ---------------------------------------------------------------

mod title_position {
    use super::*;
    use ooxml_chart::Position;

    const LAYOUT: &str = r#"<c:layout><c:manualLayout><c:xMode val="edge"/><c:yMode val="edge"/><c:x val="0.25"/><c:y val="0.05"/></c:manualLayout></c:layout>"#;

    #[test]
    fn a_chart_title_position_sits_between_the_text_and_overlay() {
        let xml = render(
            ChartSpec::new(ChartKind::Line)
                .title("Sales")
                .title_position(Position::new(0.25, 0.05))
                .series(series()),
        );
        assert_well_formed(&xml);
        assert!(
            xml.contains(&format!("</c:tx>{LAYOUT}<c:overlay val=\"0\"/></c:title>")),
            "{xml}"
        );
    }

    #[test]
    fn an_axis_title_position_is_written_the_same_way() {
        let xml = render(
            ChartSpec::new(ChartKind::Line)
                .value_axis(
                    Axis::default()
                        .title("Units")
                        .title_position(Position::new(0.25, 0.05)),
                )
                .series(series()),
        );
        assert_well_formed(&xml);
        assert!(
            xml.contains(&format!("</c:tx>{LAYOUT}<c:overlay val=\"0\"/></c:title>")),
            "{xml}"
        );
    }

    #[test]
    fn without_a_position_titles_carry_no_layout() {
        let xml = render(
            ChartSpec::new(ChartKind::Line)
                .title("Sales")
                .value_axis(Axis::default().title("Units"))
                .series(series()),
        );
        assert!(!xml.contains("<c:manualLayout>"), "{xml}");
    }

    #[test]
    fn a_position_with_no_title_is_refused() {
        let chart = ChartSpec::new(ChartKind::Line)
            .title_position(Position::new(0.1, 0.1))
            .series(series())
            .render();
        assert!(matches!(chart, Err(ChartError::Unsupported { .. })));
        let axis = ChartSpec::new(ChartKind::Line)
            .category_axis(Axis::default().title_position(Position::new(0.1, 0.1)))
            .series(series())
            .render();
        assert!(matches!(axis, Err(ChartError::Unsupported { .. })));
    }

    #[test]
    fn an_out_of_range_position_is_refused() {
        for (x, y) in [
            (-0.1, 0.5),
            (0.5, 1.1),
            (f64::NAN, 0.5),
            (0.5, f64::INFINITY),
        ] {
            let chart = ChartSpec::new(ChartKind::Line)
                .title("Sales")
                .title_position(Position::new(x, y))
                .series(series())
                .render();
            assert!(
                matches!(chart, Err(ChartError::InvalidLayout { .. })),
                "{x},{y}: {chart:?}"
            );
            let axis = ChartSpec::new(ChartKind::Line)
                .value_axis(
                    Axis::default()
                        .title("U")
                        .title_position(Position::new(x, y)),
                )
                .series(series())
                .render();
            assert!(
                matches!(axis, Err(ChartError::InvalidLayout { .. })),
                "{x},{y}: {axis:?}"
            );
        }
    }
}

// --- Axis line styling ---------------------------------------------------------------

mod axis_line {
    use super::*;

    fn chart(category: Axis, value: Axis) -> Result<String, ChartError> {
        ChartSpec::new(ChartKind::Line)
            .category_axis(category)
            .value_axis(value)
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    #[test]
    fn a_coloured_width_sits_after_the_tick_labels_and_before_the_text() {
        let xml = chart(
            Axis::default()
                .line("8E0DD1")
                .line_width(2.0)
                .label_rotation(45),
            Axis::default(),
        )
        .expect("a chart");
        assert_well_formed(&xml);
        assert!(
            xml.contains(r#"<c:spPr><a:ln w="25400"><a:solidFill><a:srgbClr val="8E0DD1"/></a:solidFill></a:ln></c:spPr><c:txPr>"#),
            "{xml}"
        );
    }

    #[test]
    fn no_line_writes_a_no_fill_outline() {
        let xml = chart(Axis::default(), Axis::default().no_line()).expect("a chart");
        assert!(
            xml.contains(r#"<c:spPr><a:ln><a:noFill/></a:ln></c:spPr><c:crossAx"#),
            "{xml}"
        );
    }

    #[test]
    fn a_width_alone_keeps_excels_colour() {
        let xml = chart(Axis::default().line_width(1.5), Axis::default()).expect("a chart");
        assert!(
            xml.contains(r#"<c:spPr><a:ln w="19050"/></c:spPr>"#),
            "{xml}"
        );
    }

    #[test]
    fn an_unstyled_axis_has_no_shape_properties() {
        let xml = chart(Axis::default(), Axis::default()).expect("a chart");
        assert!(!xml.contains("<c:spPr>"), "{xml}");
    }

    #[test]
    fn a_bad_colour_or_width_is_refused() {
        assert!(matches!(
            chart(Axis::default().line("red"), Axis::default()),
            Err(ChartError::InvalidColor(_))
        ));
        for bad in [-1.0, f64::NAN, 5000.0] {
            assert!(
                chart(Axis::default(), Axis::default().line_width(bad)).is_err(),
                "{bad}"
            );
        }
    }
}

// --- Data table ---------------------------------------------------------------------

mod data_table {
    use super::*;
    use ooxml_chart::DataTable;

    fn chart(kind: ChartKind, table: DataTable) -> Result<String, ChartError> {
        ChartSpec::new(kind)
            .data_table(table)
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
    }

    #[test]
    fn a_default_table_follows_the_axes_and_precedes_the_plot_area_shape() {
        let xml = ChartSpec::new(ChartKind::ColumnClustered)
            .data_table(DataTable::new())
            .plot_area(AreaStyle::new().fill("F8F6F0"))
            .series(series())
            .render()
            .map(|part| String::from_utf8(part.xml).expect("UTF-8"))
            .expect("a chart");
        assert_well_formed(&xml);
        assert!(
            xml.contains(r#"</c:valAx><c:dTable><c:showHorzBorder val="1"/><c:showVertBorder val="1"/><c:showOutline val="1"/><c:showKeys val="1"/></c:dTable><c:spPr>"#),
            "{xml}"
        );
    }

    #[test]
    fn switches_write_zero_and_a_style_writes_text_properties() {
        let xml = chart(
            ChartKind::Line,
            DataTable::new()
                .horizontal_borders(false)
                .vertical_borders(false)
                .outline(false)
                .legend_keys(false)
                .style(TextStyle::new().size(9.0)),
        )
        .expect("a chart");
        assert!(
            xml.contains(r#"<c:showHorzBorder val="0"/><c:showVertBorder val="0"/><c:showOutline val="0"/><c:showKeys val="0"/><c:txPr>"#),
            "{xml}"
        );
    }

    #[test]
    fn column_line_and_area_take_a_table() {
        for kind in [
            ChartKind::ColumnClustered,
            ChartKind::ColumnStacked,
            ChartKind::Line,
            ChartKind::LineMarkers,
            ChartKind::Area,
        ] {
            chart(kind, DataTable::new()).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        }
    }

    #[test]
    fn other_kinds_are_refused() {
        for kind in [
            ChartKind::BarClustered,
            ChartKind::Scatter,
            ChartKind::Pie,
            ChartKind::Doughnut,
        ] {
            assert!(
                matches!(
                    chart(kind, DataTable::new()),
                    Err(ChartError::Unsupported { .. })
                ),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn no_table_writes_no_element() {
        let xml = render(ChartSpec::new(ChartKind::Line).series(series()));
        assert!(!xml.contains("dTable"), "{xml}");
    }

    #[test]
    fn a_bad_font_in_the_table_is_refused() {
        let result = chart(
            ChartKind::Line,
            DataTable::new().style(TextStyle::new().size(f64::NAN)),
        );
        assert!(result.is_err());
    }
}
