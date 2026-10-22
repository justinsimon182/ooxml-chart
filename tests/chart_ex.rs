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
