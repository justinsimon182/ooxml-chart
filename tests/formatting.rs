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
