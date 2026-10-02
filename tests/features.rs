//! Public-API tests for chart kinds, series and axis formatting, data labels,
//! cached data and anchor types. Every rendered part is also checked to be
//! well-formed, since a malformed chart is refused by Excel without a word.

use ooxml_chart::{
    drawing_part_with, Anchor, Axis, CellAnchor, ChartError, ChartKind, ChartSpec,
    DataLabelPosition, DataLabels, GraphicFrame, MarkerSymbol, Series, SeriesName,
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

/// Tags nest and close in order. Enough to catch a hand-built string going wrong.
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

#[test]
fn every_kind_renders_a_well_formed_part() {
    for kind in [
        ChartKind::BarClustered,
        ChartKind::BarStacked,
        ChartKind::BarPercentStacked,
        ChartKind::ColumnClustered,
        ChartKind::ColumnStacked,
        ChartKind::ColumnPercentStacked,
        ChartKind::Line,
        ChartKind::LineMarkers,
        ChartKind::Area,
        ChartKind::AreaStacked,
        ChartKind::AreaPercentStacked,
        ChartKind::Scatter,
        ChartKind::ScatterLines,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Radar,
    ] {
        render(ChartSpec::new(kind).series(series()));
    }
}

#[test]
fn every_part_turns_rounded_corners_off() {
    let out = render(ChartSpec::new(ChartKind::Pie).series(series()));
    assert!(
        out.contains(r#"<c:roundedCorners val="0"/><c:chart>"#),
        "{out}"
    );
}

#[test]
fn a_horizontal_bar_chart_puts_categories_on_the_left_and_values_along_the_bottom() {
    let out = render(ChartSpec::new(ChartKind::BarClustered).series(series()));
    assert!(out.contains(r#"<c:axPos val="l"/>"#), "{out}");
    let cat = out.find("<c:catAx>").unwrap();
    assert!(out[cat..].contains(r#"<c:axPos val="l"/>"#));
    let val = out.find("<c:valAx>").unwrap();
    assert!(out[val..].contains(r#"<c:axPos val="b"/>"#));
}

#[test]
fn a_column_chart_keeps_categories_on_the_bottom() {
    let out = render(ChartSpec::new(ChartKind::ColumnClustered).series(series()));
    let cat = out.find("<c:catAx>").unwrap();
    assert!(out[cat..].contains(r#"<c:axPos val="b"/>"#), "{out}");
}

#[test]
fn percent_stacked_kinds_group_and_overlap_fully() {
    let out = render(ChartSpec::new(ChartKind::ColumnPercentStacked).series(series()));
    assert!(
        out.contains(r#"<c:grouping val="percentStacked"/>"#),
        "{out}"
    );
    assert!(out.contains(r#"<c:overlap val="100"/>"#), "{out}");
}

#[test]
fn area_kinds_write_an_area_chart_with_their_grouping() {
    let out = render(ChartSpec::new(ChartKind::AreaStacked).series(series()));
    assert!(out.contains("<c:areaChart>"), "{out}");
    assert!(out.contains(r#"<c:grouping val="stacked"/>"#), "{out}");
    assert!(out.contains("<c:catAx>"), "{out}");
}

#[test]
fn a_scatter_has_two_value_axes_and_x_and_y_values() {
    let out = render(ChartSpec::new(ChartKind::Scatter).series(series()));
    assert!(out.contains("<c:scatterChart>"), "{out}");
    assert!(!out.contains("<c:catAx>"), "{out}");
    assert_eq!(out.matches("<c:valAx>").count(), 2, "{out}");
    assert!(
        out.contains("<c:xVal><c:numRef><c:f>&#39;S&#39;!$A$3:$A$5</c:f>"),
        "{out}"
    );
    assert!(out.contains("<c:yVal><c:numRef>"), "{out}");
    // Markers only: the connecting line is off.
    assert!(out.contains("<a:ln><a:noFill/></a:ln>"), "{out}");
}

#[test]
fn a_scatter_with_lines_does_not_switch_the_line_off() {
    let out = render(ChartSpec::new(ChartKind::ScatterLines).series(series()));
    assert!(!out.contains("<a:noFill/>"), "{out}");
}

#[test]
fn a_doughnut_has_a_hole_and_a_start_angle_and_no_axes() {
    let out = render(
        ChartSpec::new(ChartKind::Doughnut)
            .series(series())
            .hole_size(60)
            .first_slice_angle(90),
    );
    assert!(out.contains("<c:doughnutChart>"), "{out}");
    assert!(
        out.contains(r#"<c:firstSliceAng val="90"/><c:holeSize val="60"/>"#),
        "{out}"
    );
    assert!(!out.contains("<c:axId"), "{out}");
}

#[test]
fn a_radar_chart_is_a_radar_chart_with_plain_lines() {
    let out = render(ChartSpec::new(ChartKind::Radar).series(series()));
    assert!(
        out.contains(r#"<c:radarChart><c:radarStyle val="marker"/>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<c:marker><c:symbol val="none"/></c:marker>"#),
        "{out}"
    );
}

#[test]
fn a_bar_series_colour_is_a_solid_fill() {
    let out =
        render(ChartSpec::new(ChartKind::ColumnClustered).series(series().with_color("8e0dd1")));
    assert!(
        out.contains(r#"<c:spPr><a:solidFill><a:srgbClr val="8E0DD1"/></a:solidFill></c:spPr>"#),
        "{out}"
    );
}

#[test]
fn sp_pr_comes_before_the_categories_as_the_schema_requires() {
    let out =
        render(ChartSpec::new(ChartKind::ColumnClustered).series(series().with_color("8E0DD1")));
    assert!(
        out.find("<c:spPr>").unwrap() < out.find("<c:cat>").unwrap(),
        "{out}"
    );
    assert!(
        out.find("<c:tx>").unwrap() < out.find("<c:spPr>").unwrap(),
        "{out}"
    );
}

#[test]
fn a_line_series_colour_and_width_are_an_outline() {
    let out = render(
        ChartSpec::new(ChartKind::Line).series(
            series()
                .with_color("28EAE4")
                .with_line_width(2.25)
                .with_smooth(true),
        ),
    );
    assert!(out.contains(r#"<a:ln w="28575" cap="rnd">"#), "{out}");
    assert!(out.contains(r#"<a:srgbClr val="28EAE4"/>"#), "{out}");
    assert!(out.contains(r#"<c:smooth val="1"/></c:ser>"#), "{out}");
}

#[test]
fn an_explicit_marker_is_written_with_its_shape_size_and_colour() {
    let out = render(
        ChartSpec::new(ChartKind::Line).series(
            series()
                .with_color("104991")
                .with_marker(MarkerSymbol::Diamond, 9),
        ),
    );
    assert!(
        out.contains(r#"<c:marker><c:symbol val="diamond"/><c:size val="9"/><c:spPr>"#),
        "{out}"
    );
    // spPr (the line) precedes marker in the series sequence.
    assert!(out.find("<c:spPr><a:ln").unwrap() < out.find("<c:marker><c:symbol").unwrap());
}

#[test]
fn a_bad_colour_is_refused() {
    for bad in ["#8E0DD1", "8E0D", "GGGGGG", "8E0DD1FF", ""] {
        let error = refused(ChartSpec::new(ChartKind::Line).series(series().with_color(bad)));
        assert!(matches!(error, ChartError::InvalidColor(_)), "{bad}");
    }
}

#[test]
fn a_marker_size_outside_the_schema_range_is_refused() {
    for size in [0, 1, 73] {
        let error = refused(
            ChartSpec::new(ChartKind::Line)
                .series(series().with_marker(MarkerSymbol::Circle, size)),
        );
        assert!(matches!(error, ChartError::OutOfRange { .. }), "{size}");
    }
}

#[test]
fn a_negative_or_non_finite_line_width_is_refused() {
    for width in [-1.0, f64::NAN, f64::INFINITY] {
        let error =
            refused(ChartSpec::new(ChartKind::Line).series(series().with_line_width(width)));
        assert!(matches!(error, ChartError::OutOfRange { .. }));
    }
}

#[test]
fn an_axis_carries_its_scale_format_and_direction_in_schema_order() {
    let out = render(
        ChartSpec::new(ChartKind::ColumnClustered)
            .series(series())
            .value_axis(
                Axis::default()
                    .min(0.0)
                    .max(1.0)
                    .major_unit(0.25)
                    .number_format("0%")
                    .reversed(true),
            ),
    );
    assert!(
        out.contains(r#"<c:scaling><c:orientation val="maxMin"/><c:max val="1"/><c:min val="0"/></c:scaling>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<c:numFmt formatCode="0%" sourceLinked="0"/>"#),
        "{out}"
    );
    assert!(
        out.contains(r#"<c:crossBetween val="between"/><c:majorUnit val="0.25"/></c:valAx>"#),
        "{out}"
    );
    assert!(out.find("<c:numFmt").unwrap() < out.rfind("<c:crossAx").unwrap());
}

#[test]
fn a_category_axis_does_not_carry_bounds() {
    let out = render(
        ChartSpec::new(ChartKind::ColumnClustered)
            .series(series())
            .category_axis(Axis::default().min(0.0).max(5.0)),
    );
    let cat_end = out.find("</c:catAx>").unwrap();
    assert!(!out[..cat_end].contains("<c:max"), "{out}");
}

#[test]
fn an_inverted_or_non_finite_axis_range_is_refused() {
    for axis in [
        Axis::default().min(5.0).max(5.0),
        Axis::default().min(6.0).max(5.0),
        Axis::default().max(f64::NAN),
        Axis::default().major_unit(0.0),
        Axis::default().major_unit(-1.0),
    ] {
        let error = refused(
            ChartSpec::new(ChartKind::Line)
                .series(series())
                .value_axis(axis),
        );
        assert!(matches!(error, ChartError::InvalidAxisRange { .. }));
    }
}

#[test]
fn data_labels_are_written_before_the_gap_width() {
    let out = render(
        ChartSpec::new(ChartKind::ColumnClustered)
            .series(series())
            .data_labels(
                DataLabels::values()
                    .at(DataLabelPosition::OutsideEnd)
                    .number_format("#,##0"),
            ),
    );
    assert!(
        out.contains(r##"<c:dLbls><c:numFmt formatCode="#,##0" sourceLinked="0"/><c:dLblPos val="outEnd"/><c:showLegendKey val="0"/><c:showVal val="1"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/></c:dLbls><c:gapWidth"##),
        "{out}"
    );
}

#[test]
fn pie_labels_can_show_category_and_percent() {
    let out = render(
        ChartSpec::new(ChartKind::Pie).series(series()).data_labels(
            DataLabels::default()
                .with_category()
                .with_percent()
                .at(DataLabelPosition::BestFit),
        ),
    );
    assert!(out.contains(r#"<c:showCatName val="1"/>"#), "{out}");
    assert!(out.contains(r#"<c:showPercent val="1"/>"#), "{out}");
    assert!(out.contains(r#"<c:dLblPos val="bestFit"/>"#), "{out}");
}

#[test]
fn a_label_position_excel_would_reject_is_refused() {
    let cases = [
        (ChartKind::ColumnStacked, DataLabelPosition::OutsideEnd),
        (ChartKind::BarPercentStacked, DataLabelPosition::OutsideEnd),
        (ChartKind::ColumnClustered, DataLabelPosition::Above),
        (ChartKind::Line, DataLabelPosition::OutsideEnd),
        (ChartKind::Pie, DataLabelPosition::Above),
        (ChartKind::Area, DataLabelPosition::Center),
        (ChartKind::Doughnut, DataLabelPosition::Center),
        (ChartKind::Radar, DataLabelPosition::Center),
    ];
    for (kind, position) in cases {
        let error = refused(
            ChartSpec::new(kind)
                .series(series())
                .data_labels(DataLabels::values().at(position)),
        );
        assert!(
            matches!(error, ChartError::InvalidDataLabelPosition { .. }),
            "{kind:?} {position:?}"
        );
    }
}

#[test]
fn labels_without_a_position_are_fine_everywhere() {
    for kind in [
        ChartKind::Area,
        ChartKind::Doughnut,
        ChartKind::Radar,
        ChartKind::ColumnStacked,
    ] {
        render(
            ChartSpec::new(kind)
                .series(series())
                .data_labels(DataLabels::values()),
        );
    }
}

#[test]
fn values_and_categories_can_be_cached_so_previews_draw_the_plot() {
    let out = render(
        ChartSpec::new(ChartKind::ColumnClustered).series(
            series()
                .with_cached_categories(vec!["Q1".into(), "R&D".into(), "Q3".into()])
                .with_cached_values(vec![1.5, f64::NAN, 3.0]),
        ),
    );
    assert!(
        out.contains(r#"<c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>R&amp;D</c:v></c:pt>"#),
        "{out}"
    );
    // The NaN is a blank: counted in ptCount, absent as a pt.
    assert!(
        out.contains(r#"<c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>1.5</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt></c:numCache>"#),
        "{out}"
    );
}

#[test]
fn a_scatter_caches_its_x_values_as_numbers() {
    let out = render(
        ChartSpec::new(ChartKind::Scatter).series(series().with_cached_categories(vec![
            "1".into(),
            "2.5".into(),
            "n/a".into(),
        ])),
    );
    assert!(
        out.contains(r#"<c:xVal><c:numRef><c:f>&#39;S&#39;!$A$3:$A$5</c:f><c:numCache>"#),
        "{out}"
    );
    assert!(out.contains(r#"<c:ptCount val="3"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2.5</c:v></c:pt></c:numCache>"#), "{out}");
}

#[test]
fn bar_geometry_outside_the_schema_range_is_refused() {
    assert!(matches!(
        refused(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(series())
                .gap_width(501)
        ),
        ChartError::OutOfRange { .. }
    ));
    assert!(matches!(
        refused(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(series())
                .overlap(101)
        ),
        ChartError::OutOfRange { .. }
    ));
    // The same values are ignored, not refused, where they mean nothing.
    render(
        ChartSpec::new(ChartKind::Line)
            .series(series())
            .gap_width(999),
    );
}

#[test]
fn doughnut_hole_and_slice_angle_are_range_checked() {
    for spec in [
        ChartSpec::new(ChartKind::Doughnut)
            .series(series())
            .hole_size(9),
        ChartSpec::new(ChartKind::Doughnut)
            .series(series())
            .hole_size(91),
        ChartSpec::new(ChartKind::Pie)
            .series(series())
            .first_slice_angle(361),
    ] {
        assert!(matches!(refused(spec), ChartError::OutOfRange { .. }));
    }
}

#[test]
fn a_pie_with_a_start_angle_writes_it_and_one_without_writes_nothing() {
    let plain = render(ChartSpec::new(ChartKind::Pie).series(series()));
    assert!(!plain.contains("firstSliceAng"), "{plain}");
    let turned = render(
        ChartSpec::new(ChartKind::Pie)
            .series(series())
            .first_slice_angle(45),
    );
    assert!(
        turned.contains(r#"<c:firstSliceAng val="45"/></c:pieChart>"#),
        "{turned}"
    );
}

// --- anchors ---------------------------------------------------------------

fn frame() -> GraphicFrame {
    GraphicFrame::new(2, "Chart 1", "rId1").with_edit_as("oneCell")
}

fn corner() -> CellAnchor {
    CellAnchor::new(3, 4).with_offsets(0, 9525)
}

#[test]
fn a_one_cell_anchor_has_a_corner_and_a_size_and_no_edit_as() {
    let part = drawing_part_with(&[(Anchor::one_cell(corner(), 1_000_000, 500_000), frame())]);
    let out = String::from_utf8(part).unwrap();
    assert_well_formed(&out);
    assert!(
        out.contains("<xdr:oneCellAnchor><xdr:from><xdr:col>3</xdr:col>"),
        "{out}"
    );
    assert!(
        out.contains(r#"<xdr:ext cx="1000000" cy="500000"/><xdr:graphicFrame"#),
        "{out}"
    );
    assert!(!out.contains("editAs"), "{out}");
    assert!(out.ends_with("</xdr:oneCellAnchor></xdr:wsDr>"), "{out}");
}

#[test]
fn an_absolute_anchor_has_a_position_and_a_size() {
    let part = drawing_part_with(&[(Anchor::absolute(10, 20, 30, 40), frame())]);
    let out = String::from_utf8(part).unwrap();
    assert_well_formed(&out);
    assert!(
        out.contains(r#"<xdr:absoluteAnchor><xdr:pos x="10" y="20"/><xdr:ext cx="30" cy="40"/>"#),
        "{out}"
    );
}

#[test]
fn anchor_types_can_share_a_drawing() {
    use ooxml_chart::TwoCellAnchor;
    let two = TwoCellAnchor::new(corner(), CellAnchor::new(9, 20));
    let part = drawing_part_with(&[
        (Anchor::TwoCell(two), frame()),
        (Anchor::absolute(0, 0, 1, 1), frame()),
    ]);
    let out = String::from_utf8(part).unwrap();
    assert_well_formed(&out);
    assert!(
        out.contains(r#"<xdr:twoCellAnchor editAs="oneCell">"#),
        "{out}"
    );
    assert!(out.contains("<xdr:absoluteAnchor>"), "{out}");
}

// --- pictures and text boxes ------------------------------------------------

mod other_objects {
    use super::*;
    use ooxml_chart::{
        drawing_part_objects, drawing_relationships_typed, DrawingObject, Picture, TextBox,
        TwoCellAnchor, CHART_RELATIONSHIP_TYPE, IMAGE_RELATIONSHIP_TYPE,
    };

    fn part(anchor: Anchor, object: DrawingObject) -> String {
        let bytes = drawing_part_objects(&[(anchor, object)]);
        String::from_utf8(bytes).expect("utf-8")
    }

    #[test]
    fn a_picture_points_at_its_image_and_carries_alt_text() {
        let xml = part(
            Anchor::one_cell(corner(), 100, 50),
            DrawingObject::Picture(Picture::new(3, "Logo", "rId7").description("A <logo>")),
        );
        assert!(
            xml.contains(r#"<xdr:oneCellAnchor><xdr:from><xdr:col>3</xdr:col>"#),
            "{xml}"
        );
        assert!(
            xml.contains(r#"<xdr:ext cx="100" cy="50"/><xdr:pic><xdr:nvPicPr><xdr:cNvPr id="3" name="Logo" descr="A &lt;logo&gt;"/>"#),
            "{xml}"
        );
        assert!(
            xml.contains(r#"<a:blip r:embed="rId7"/><a:stretch><a:fillRect/></a:stretch>"#),
            "{xml}"
        );
        assert!(
            xml.ends_with("</xdr:pic><xdr:clientData/></xdr:oneCellAnchor></xdr:wsDr>"),
            "{xml}"
        );
    }

    #[test]
    fn a_text_box_makes_a_paragraph_per_line_and_escapes_text() {
        let xml = part(
            Anchor::absolute(1, 2, 3, 4),
            DrawingObject::TextBox(TextBox::new(4, "Note", "a & b\n\nc")),
        );
        assert!(
            xml.contains(r#"<xdr:absoluteAnchor><xdr:pos x="1" y="2"/><xdr:ext cx="3" cy="4"/><xdr:sp macro="" textlink="">"#),
            "{xml}"
        );
        assert!(xml.contains(r#"<xdr:cNvSpPr txBox="1"/>"#), "{xml}");
        assert!(
            xml.contains("<a:p><a:r><a:t>a &amp; b</a:t></a:r></a:p><a:p/><a:p><a:r><a:t>c</a:t></a:r></a:p></xdr:txBody>"),
            "{xml}"
        );
    }

    #[test]
    fn a_chart_object_is_written_as_before_and_objects_mix() {
        let two = TwoCellAnchor::new(corner(), CellAnchor::new(9, 20));
        let plain = drawing_part_with(&[(Anchor::TwoCell(two), frame())]);
        let objects = drawing_part_objects(&[
            (Anchor::TwoCell(two), DrawingObject::Chart(frame())),
            (
                Anchor::TwoCell(two),
                DrawingObject::Picture(Picture::new(5, "P", "rId2")),
            ),
        ]);
        let objects = String::from_utf8(objects).expect("utf-8");
        let plain = String::from_utf8(plain).expect("utf-8");
        let chart_only = plain.trim_end_matches("</xdr:wsDr>");
        assert!(objects.starts_with(chart_only), "{objects}");
        assert_eq!(objects.matches("<xdr:clientData/>").count(), 2, "{objects}");
    }

    #[test]
    fn typed_relationships_carry_each_kind() {
        let rels = drawing_relationships_typed(&[
            (
                "rId1".into(),
                CHART_RELATIONSHIP_TYPE,
                "../charts/chart1.xml".into(),
            ),
            (
                "rId2".into(),
                IMAGE_RELATIONSHIP_TYPE,
                "../media/image1.png".into(),
            ),
        ]);
        let rels = String::from_utf8(rels).expect("utf-8");
        assert!(
            rels.contains(&format!(
                r#"Type="{IMAGE_RELATIONSHIP_TYPE}" Target="../media/image1.png""#
            )),
            "{rels}"
        );
        assert_eq!(rels.matches("<Relationship ").count(), 2, "{rels}");
    }
}

// --- chartsheets ------------------------------------------------------------------------

mod chartsheets {
    use super::*;
    use ooxml_chart::{
        chartsheet_content_types_override, chartsheet_drawing_part, chartsheet_part,
        workbook_chartsheet_relationship, workbook_sheet_element, CHARTSHEET_CONTENT_TYPE,
        CHARTSHEET_RELATIONSHIP_TYPE,
    };

    #[test]
    fn a_chartsheet_wraps_its_drawing_and_is_well_formed() {
        let xml = String::from_utf8(chartsheet_part("rId1")).expect("utf-8");
        assert!(
            xml.contains(
                r#"<chartsheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#
            ),
            "{xml}"
        );
        assert!(
            xml.ends_with(r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><drawing r:id="rId1"/></chartsheet>"#),
            "{xml}"
        );
    }

    #[test]
    fn the_drawing_anchors_one_chart_absolutely_at_the_origin() {
        let xml = String::from_utf8(chartsheet_drawing_part(&frame())).expect("utf-8");
        assert!(
            xml.contains(
                r#"<xdr:absoluteAnchor><xdr:pos x="0" y="0"/><xdr:ext cx="9293679" cy="6068786"/>"#
            ),
            "{xml}"
        );
    }

    #[test]
    fn the_registration_lines_name_the_right_types() {
        assert_eq!(
            chartsheet_content_types_override("/xl/chartsheets/sheet1.xml"),
            format!(
                r#"<Override PartName="/xl/chartsheets/sheet1.xml" ContentType="{CHARTSHEET_CONTENT_TYPE}"/>"#
            )
        );
        assert_eq!(
            workbook_chartsheet_relationship("rId3", "chartsheets/sheet1.xml"),
            format!(
                r#"<Relationship Id="rId3" Type="{CHARTSHEET_RELATIONSHIP_TYPE}" Target="chartsheets/sheet1.xml"/>"#
            )
        );
        assert_eq!(
            workbook_sheet_element("Q&A", 2, "rId3").expect("a name"),
            r#"<sheet name="Q&amp;A" sheetId="2" r:id="rId3"/>"#
        );
    }

    #[test]
    fn a_name_excel_would_refuse_is_refused() {
        let long = "x".repeat(32);
        for name in ["", "a/b", "a:b", "[x]", "'x", "x'", long.as_str()] {
            assert!(workbook_sheet_element(name, 1, "rId1").is_err(), "{name:?}");
        }
        assert!(workbook_sheet_element(&"x".repeat(31), 1, "rId1").is_ok());
    }
}
