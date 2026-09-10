//! Turning a [`crate::ChartSpec`] into chart XML.
//!
//! # Element order is not free
//!
//! OOXML's chart schema is a sequence: `<c:barDir>` before `<c:grouping>`
//! before the series before `<c:gapWidth>` before the axis ids. A file with the
//! same elements in a different order is refused by Excel, silently and with no
//! diagnosis. The order below is the order in the reference chart this crate is
//! measured against.

use crate::error::ChartError;
use crate::spec::{Axis, ChartKind, ChartPart, ChartSpec, Series, SeriesName};
use crate::xml::escape;

/// The two axis ids every axed chart uses. Arbitrary but stable: what matters
/// is that each axis names the other as its `crossAx`.
const CATEGORY_AXIS_ID: u32 = 111;
const VALUE_AXIS_ID: u32 = 222;

const HEADER: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<c:chartSpace"#,
    r#" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart""#,
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
);

pub fn chart_space(spec: &ChartSpec) -> Result<ChartPart, ChartError> {
    if spec.series.is_empty() {
        return Err(ChartError::NoSeries);
    }

    let mut out = String::with_capacity(2048);
    out.push_str(HEADER);
    out.push_str("<c:chart>");
    if let Some(title) = &spec.title {
        out.push_str(&title_xml(title));
    }
    out.push_str("<c:plotArea><c:layout/>");
    out.push_str(&plot_xml(spec));
    if spec.kind.has_axes() {
        out.push_str(&category_axis_xml(&spec.category_axis));
        out.push_str(&value_axis_xml(&spec.value_axis));
    }
    out.push_str("</c:plotArea>");
    if let Some(code) = spec.legend.code() {
        out.push_str(&format!(
            r#"<c:legend><c:legendPos val="{code}"/><c:overlay val="0"/></c:legend>"#
        ));
    }
    out.push_str(r#"<c:plotVisOnly val="1"/>"#);
    out.push_str("</c:chart></c:chartSpace>");

    Ok(ChartPart {
        xml: out.into_bytes(),
        content_type: ChartPart::CONTENT_TYPE,
    })
}

fn plot_xml(spec: &ChartSpec) -> String {
    let series: String = spec
        .series
        .iter()
        .enumerate()
        .map(|(index, series)| series_xml(index, series))
        .collect();

    match spec.kind {
        ChartKind::BarClustered
        | ChartKind::BarStacked
        | ChartKind::ColumnClustered
        | ChartKind::ColumnStacked => {
            let direction = match spec.kind {
                ChartKind::BarClustered | ChartKind::BarStacked => "bar",
                _ => "col",
            };
            let (grouping, default_overlap) = match spec.kind {
                ChartKind::BarStacked | ChartKind::ColumnStacked => ("stacked", Some(100)),
                _ => ("clustered", None),
            };
            let overlap = spec.overlap.or(default_overlap);
            let overlap_xml = overlap
                .map(|value| format!(r#"<c:overlap val="{value}"/>"#))
                .unwrap_or_default();
            format!(
                r#"<c:barChart><c:barDir val="{direction}"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}<c:gapWidth val="{gap}"/>{overlap_xml}{axes}</c:barChart>"#,
                gap = spec.gap_width,
                axes = axis_ids(),
            )
        }
        ChartKind::Line | ChartKind::LineMarkers => {
            let marker = i32::from(spec.kind == ChartKind::LineMarkers);
            format!(
                r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>{series}<c:marker val="{marker}"/>{axes}</c:lineChart>"#,
                axes = axis_ids(),
            )
        }
        ChartKind::Pie => {
            format!(r#"<c:pieChart><c:varyColors val="1"/>{series}</c:pieChart>"#)
        }
    }
}

fn axis_ids() -> String {
    format!(r#"<c:axId val="{CATEGORY_AXIS_ID}"/><c:axId val="{VALUE_AXIS_ID}"/>"#)
}

fn series_xml(index: usize, series: &Series) -> String {
    let name = match &series.name {
        SeriesName::Literal(text) => format!("<c:tx><c:v>{}</c:v></c:tx>", escape(text)),
        SeriesName::Reference(reference) => format!(
            "<c:tx><c:strRef><c:f>{}</c:f></c:strRef></c:tx>",
            escape(reference)
        ),
    };
    let categories = series
        .categories
        .as_ref()
        .map(|reference| {
            format!(
                "<c:cat><c:strRef><c:f>{}</c:f></c:strRef></c:cat>",
                escape(reference)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>{name}{categories}<c:val><c:numRef><c:f>{values}</c:f></c:numRef></c:val></c:ser>"#,
        values = escape(&series.values),
    )
}

fn title_xml(text: &str) -> String {
    format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title><c:autoTitleDeleted val="0"/>"#,
        escape(text)
    )
}

fn category_axis_xml(axis: &Axis) -> String {
    format!(
        r#"<c:catAx><c:axId val="{CATEGORY_AXIS_ID}"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="{deleted}"/><c:axPos val="b"/>{gridlines}{title}<c:crossAx val="{VALUE_AXIS_ID}"/></c:catAx>"#,
        deleted = i32::from(!axis.visible),
        gridlines = gridlines_xml(axis),
        title = axis_title_xml(axis),
    )
}

fn value_axis_xml(axis: &Axis) -> String {
    format!(
        r#"<c:valAx><c:axId val="{VALUE_AXIS_ID}"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="{deleted}"/><c:axPos val="l"/>{gridlines}{title}<c:crossAx val="{CATEGORY_AXIS_ID}"/></c:valAx>"#,
        deleted = i32::from(!axis.visible),
        gridlines = gridlines_xml(axis),
        title = axis_title_xml(axis),
    )
}

fn gridlines_xml(axis: &Axis) -> &'static str {
    if axis.major_gridlines {
        "<c:majorGridlines/>"
    } else {
        ""
    }
}

fn axis_title_xml(axis: &Axis) -> String {
    axis.title
        .as_ref()
        .map(|text| {
            format!(
                r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>"#,
                escape(text)
            )
        })
        .unwrap_or_default()
}
