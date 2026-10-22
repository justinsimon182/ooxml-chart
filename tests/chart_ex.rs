//! Public-API tests for the Office 2016 chart kinds (`chartEx` parts).

use ooxml_chart::{
    chart_ex_content_types_override, Anchor, Bins, ChartError, ChartEx, ChartExKind, GraphicFrame,
    SeriesName,
};

const VALUES: &str = "'S'!$C$2:$C$6";
const CATS: &str = "'S'!$A$2:$A$6";

fn render(chart: ChartEx) -> String {
    String::from_utf8(chart.render().expect("a chartEx part")).expect("UTF-8")
}

#[test]
fn a_waterfall_matches_what_excel_writes() {
    let xml = render(ChartEx::new(ChartExKind::Waterfall, VALUES).categories(CATS));
    assert!(xml.contains(concat!(
        r#"<cx:chartData><cx:data id="0"><cx:strDim type="cat"><cx:f>&#39;S&#39;!$A$2:$A$6</cx:f></cx:strDim>"#,
        r#"<cx:numDim type="val"><cx:f>&#39;S&#39;!$C$2:$C$6</cx:f></cx:numDim></cx:data></cx:chartData>"#,
    )));
    assert!(xml.contains(r#"<cx:series layoutId="waterfall" "#), "{xml}");
    assert!(xml.contains(r#"<cx:dataLabels pos="outEnd">"#), "{xml}");
    assert!(xml.contains("<cx:layoutPr><cx:subtotals></cx:subtotals></cx:layoutPr>"));
    assert!(xml.contains(r#"<cx:catScaling gapWidth="0.5"/>"#), "{xml}");
    assert!(
        xml.ends_with(r#"<cx:legend pos="t" align="ctr" overlay="0"/></cx:chart></cx:chartSpace>"#)
    );
}

#[test]
fn subtotals_title_name_and_legend_are_written() {
    let xml = render(
        ChartEx::new(ChartExKind::Waterfall, VALUES)
            .categories(CATS)
            .title("Cash & flow")
            .name(SeriesName::Reference("'S'!$C$1".into()))
            .legend(false)
            .subtotals([0, 4]),
    );
    assert!(xml.contains(
        "<cx:title pos=\"t\" align=\"ctr\" overlay=\"0\"><cx:tx><cx:txData><cx:v>Cash &amp; flow</cx:v>"
    ));
    assert!(xml.contains("<cx:tx><cx:txData><cx:f>&#39;S&#39;!$C$1</cx:f></cx:txData></cx:tx>"));
    assert!(xml.contains(r#"<cx:subtotals><cx:idx val="0"/><cx:idx val="4"/></cx:subtotals>"#));
    assert!(!xml.contains("<cx:legend"), "{xml}");
}

#[test]
fn each_kind_gets_its_layout_and_hierarchical_kinds_size_by_value() {
    for (kind, layout, dim) in [
        (ChartExKind::Funnel, "funnel", "val"),
        (ChartExKind::Histogram, "clusteredColumn", "val"),
        (ChartExKind::Pareto, "clusteredColumn", "val"),
        (ChartExKind::BoxWhisker, "boxWhisker", "val"),
        (ChartExKind::Treemap, "treemap", "size"),
        (ChartExKind::Sunburst, "sunburst", "size"),
    ] {
        let xml = render(ChartEx::new(kind, VALUES).categories(CATS));
        assert!(xml.contains(&format!(r#"layoutId="{layout}""#)), "{kind:?}");
        assert!(
            xml.contains(&format!(r#"<cx:numDim type="{dim}">"#)),
            "{kind:?}"
        );
    }
}

#[test]
fn a_pareto_adds_its_line_on_a_percentage_axis() {
    let xml = render(ChartEx::new(ChartExKind::Pareto, VALUES).categories(CATS));
    assert!(xml.contains(r#"<cx:series layoutId="paretoLine" ownerIdx="0" "#));
    assert!(xml.contains(
        r#"<cx:axis id="2"><cx:valScaling max="1" min="0"/><cx:units unit="percentage"/>"#
    ));
}

#[test]
fn bins_are_written_for_a_histogram() {
    let count = render(ChartEx::new(ChartExKind::Histogram, VALUES).bins(Bins::Count(5)));
    assert!(count.contains(r#"<cx:binning intervalClosed="r"><cx:binCount val="5"/></cx:binning>"#));
    let width = render(ChartEx::new(ChartExKind::Histogram, VALUES).bins(Bins::Width(2.5)));
    assert!(width.contains(r#"<cx:binSize val="2.5"/>"#), "{width}");
    assert!(!width.contains("<cx:strDim"), "no categories were given");
}

#[test]
fn options_a_kind_does_not_have_are_refused() {
    let refused = |chart: ChartEx| {
        assert!(
            matches!(chart.render(), Err(ChartError::Unsupported { .. })),
            "{chart:?}"
        )
    };
    refused(ChartEx::new(ChartExKind::Waterfall, VALUES));
    refused(
        ChartEx::new(ChartExKind::Funnel, VALUES)
            .categories(CATS)
            .subtotals([1]),
    );
    refused(
        ChartEx::new(ChartExKind::Treemap, VALUES)
            .categories(CATS)
            .bins(Bins::Count(3)),
    );
    refused(ChartEx::new(ChartExKind::Histogram, VALUES).bins(Bins::Count(0)));
    refused(ChartEx::new(ChartExKind::Histogram, VALUES).bins(Bins::Width(f64::NAN)));
    assert!(matches!(
        ChartEx::new(ChartExKind::Histogram, "Sheet1!A1:").render(),
        Err(ChartError::InvalidReference { .. })
    ));
}

#[test]
fn a_frame_for_a_chart_ex_names_the_chartex_namespace() {
    let anchor = Anchor::absolute(0, 0, 4_572_000, 2_743_200);
    let frame = GraphicFrame::new(2, "Chart 1", "rId1").for_chart_ex();
    let xml = ooxml_chart::anchor_xml_for(&anchor, &frame);
    assert!(xml.contains(concat!(
        r#"<a:graphicData uri="http://schemas.microsoft.com/office/drawing/2014/chartex">"#,
        r#"<cx:chart xmlns:cx="http://schemas.microsoft.com/office/drawing/2014/chartex""#,
    )));
    assert_eq!(
        chart_ex_content_types_override("/xl/charts/chartEx1.xml"),
        r#"<Override PartName="/xl/charts/chartEx1.xml" ContentType="application/vnd.ms-office.chartex+xml"/>"#
    );
}

#[test]
fn a_box_and_whisker_takes_several_series_each_with_its_own_data() {
    use ooxml_chart::{BoxWhiskerStyle, ChartExSeries, QuartileMethod};
    let xml = render(
        ChartEx::new(ChartExKind::BoxWhisker, VALUES)
            .categories(CATS)
            .add_series(ChartExSeries::new("'S'!$D$2:$D$6").name(SeriesName::Literal("Two".into())))
            .box_whisker(
                BoxWhiskerStyle::new()
                    .mean_line(true)
                    .inner_points(true)
                    .quartile_method(QuartileMethod::Inclusive),
            ),
    );
    assert!(xml.contains(concat!(
        r#"<cx:data id="1"><cx:strDim type="cat"><cx:f>&#39;S&#39;!$A$2:$A$6</cx:f></cx:strDim>"#,
        r#"<cx:numDim type="val"><cx:f>&#39;S&#39;!$D$2:$D$6</cx:f></cx:numDim></cx:data>"#,
    )));
    assert!(xml.contains(r#"<cx:dataId val="1"/>"#), "{xml}");
    assert!(xml.contains("<cx:v>Two</cx:v>"), "{xml}");
    assert_eq!(
        xml.matches(r#"<cx:visibility meanLine="1" meanMarker="1" nonoutliers="1" outliers="1"/><cx:statistics quartileMethod="inclusive"/>"#)
            .count(),
        2
    );
}

#[test]
fn treemap_parent_labels_and_data_labels_can_be_changed() {
    use ooxml_chart::ParentLabels;
    let xml = render(
        ChartEx::new(ChartExKind::Treemap, VALUES)
            .categories(CATS)
            .parent_labels(ParentLabels::Banner)
            .data_labels(false),
    );
    assert!(
        xml.contains(r#"<cx:parentLabelLayout val="banner"/>"#),
        "{xml}"
    );
    assert!(!xml.contains("<cx:dataLabels"), "{xml}");
    let labelled = render(ChartEx::new(ChartExKind::Histogram, VALUES).data_labels(true));
    assert!(labelled
        .contains(r#"<cx:dataLabels><cx:visibility seriesName="0" categoryName="0" value="1"/>"#));
}

#[test]
fn options_for_other_kinds_are_refused() {
    use ooxml_chart::{BoxWhiskerStyle, ChartExSeries, ParentLabels};
    for chart in [
        ChartEx::new(ChartExKind::Funnel, VALUES)
            .categories(CATS)
            .add_series(ChartExSeries::new(VALUES)),
        ChartEx::new(ChartExKind::Histogram, VALUES).box_whisker(BoxWhiskerStyle::new()),
        ChartEx::new(ChartExKind::Sunburst, VALUES)
            .categories(CATS)
            .parent_labels(ParentLabels::None),
    ] {
        assert!(
            matches!(chart.render(), Err(ChartError::Unsupported { .. })),
            "{chart:?}"
        );
    }
    let bad = ChartEx::new(ChartExKind::BoxWhisker, VALUES).add_series(ChartExSeries::new("A1:"));
    assert!(matches!(
        bad.render(),
        Err(ChartError::InvalidReference { .. })
    ));
}

#[test]
fn histogram_overflow_underflow_and_closed_left_bins_are_written() {
    let xml = render(
        ChartEx::new(ChartExKind::Histogram, VALUES)
            .bins(Bins::Count(4))
            .underflow(2.0)
            .overflow(9.5)
            .bins_closed_left(),
    );
    assert!(xml.contains(r#"<cx:binning intervalClosed="l" underflow="2" overflow="9.5"><cx:binCount val="4"/></cx:binning>"#), "{xml}");
    let pareto = render(
        ChartEx::new(ChartExKind::Pareto, VALUES)
            .categories(CATS)
            .overflow(5.0),
    );
    assert!(
        pareto.contains(r#"<cx:binning intervalClosed="r" overflow="5"></cx:binning>"#),
        "{pareto}"
    );
}

#[test]
fn connector_lines_and_axis_titles_are_written() {
    let xml = render(
        ChartEx::new(ChartExKind::Waterfall, VALUES)
            .categories(CATS)
            .connector_lines(false)
            .category_axis_title("Month")
            .value_axis_title("Cash <USD>"),
    );
    assert!(xml.contains(r#"<cx:layoutPr><cx:visibility connectorLines="0"/><cx:subtotals>"#));
    assert!(xml.contains(
        r#"<cx:catScaling gapWidth="0.5"/><cx:title><cx:tx><cx:txData><cx:v>Month</cx:v>"#
    ));
    assert!(xml.contains(r#"<cx:valScaling/><cx:title><cx:tx><cx:txData><cx:v>Cash &lt;USD&gt;</cx:v></cx:txData></cx:tx></cx:title><cx:majorGridlines/>"#));
}

#[test]
fn binning_connector_and_axis_options_on_other_kinds_are_refused() {
    for chart in [
        ChartEx::new(ChartExKind::Funnel, VALUES)
            .categories(CATS)
            .overflow(1.0),
        ChartEx::new(ChartExKind::Histogram, VALUES)
            .underflow(5.0)
            .overflow(5.0),
        ChartEx::new(ChartExKind::Histogram, VALUES).overflow(f64::INFINITY),
        ChartEx::new(ChartExKind::Pareto, VALUES)
            .categories(CATS)
            .connector_lines(true),
        ChartEx::new(ChartExKind::Treemap, VALUES)
            .categories(CATS)
            .category_axis_title("x"),
        ChartEx::new(ChartExKind::Funnel, VALUES)
            .categories(CATS)
            .value_axis_title("y"),
    ] {
        assert!(
            matches!(chart.render(), Err(ChartError::Unsupported { .. })),
            "{chart:?}"
        );
    }
}

#[test]
fn series_points_chart_area_and_text_are_styled() {
    use ooxml_chart::{AreaStyle, ChartExSeries, TextStyle};
    let xml = render(
        ChartEx::new(ChartExKind::BoxWhisker, VALUES)
            .style(AreaStyle::new().fill("8E0DD1"))
            .add_series(ChartExSeries::new(VALUES).style(AreaStyle::new().fill("0090b2")))
            .chart_area(AreaStyle::new().fill("F8F6F0"))
            .text_style(TextStyle::new().size(9.0)),
    );
    assert!(xml.contains(r#"<cx:spPr><a:solidFill><a:srgbClr val="8E0DD1"/></a:solidFill></cx:spPr><cx:dataId val="0"/>"#), "{xml}");
    assert!(xml.contains(r#"<cx:spPr><a:solidFill><a:srgbClr val="0090B2"/></a:solidFill></cx:spPr><cx:dataId val="1"/>"#));
    assert!(xml.contains(r#"</cx:chart><cx:spPr><a:solidFill><a:srgbClr val="F8F6F0"/></a:solidFill></cx:spPr><cx:txPr>"#));
    assert!(xml.ends_with("</cx:txPr></cx:chartSpace>"), "{xml}");
    assert!(!xml.contains("<c:"), "{xml}");

    let treemap = render(
        ChartEx::new(ChartExKind::Treemap, VALUES)
            .categories(CATS)
            .point(2, AreaStyle::new().fill("28EAE4")),
    );
    assert!(treemap.contains(r#"<cx:dataPt idx="2"><cx:spPr><a:solidFill><a:srgbClr val="28EAE4"/></a:solidFill></cx:spPr></cx:dataPt><cx:dataLabels"#), "{treemap}");
}

#[test]
fn bad_styles_on_a_chart_ex_are_refused() {
    use ooxml_chart::{AreaStyle, PictureFill};
    assert!(matches!(
        ChartEx::new(ChartExKind::Histogram, VALUES)
            .style(AreaStyle::new().fill("purple"))
            .render(),
        Err(ChartError::InvalidColor(_))
    ));
    let picture =
        AreaStyle::new().fill_paint(ooxml_chart::Paint::Picture(PictureFill::new("rId9")));
    assert!(matches!(
        ChartEx::new(ChartExKind::Histogram, VALUES)
            .chart_area(picture)
            .render(),
        Err(ChartError::Unsupported { .. })
    ));
}

#[test]
fn a_region_map_colours_by_value_and_leaves_geography_to_excel() {
    let xml = render(ChartEx::new(ChartExKind::RegionMap, VALUES).categories(CATS));
    assert!(xml.contains(r#"<cx:numDim type="colorVal">"#), "{xml}");
    assert!(xml.contains(r#"<cx:series layoutId="regionMap" "#));
    // A geography without Bing's cache stops Excel looking the regions up.
    assert!(
        !xml.contains("<cx:layoutPr") && !xml.contains("geography"),
        "{xml}"
    );
    assert!(xml.contains(r#"<cx:legend pos="r" align="min" overlay="0"/>"#));
    assert!(!xml.contains("<cx:axis"));
    assert!(matches!(
        ChartEx::new(ChartExKind::RegionMap, VALUES).render(),
        Err(ChartError::Unsupported { .. })
    ));
    assert!(matches!(
        ChartEx::new(ChartExKind::RegionMap, VALUES)
            .categories(CATS)
            .value_axis_title("v")
            .render(),
        Err(ChartError::Unsupported { .. })
    ));
}
