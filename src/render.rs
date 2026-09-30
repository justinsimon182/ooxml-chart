//! Turning a [`crate::ChartSpec`] into chart XML.
//!
//! # Element order is not free
//!
//! OOXML's chart schema is a sequence: `<c:barDir>` before `<c:grouping>`
//! before the series before `<c:gapWidth>` before the axis ids. A file with the
//! same elements in a different order is refused by Excel, silently and with no
//! diagnosis. The order below is the order in the reference chart this crate is
//! measured against, and — for the elements the reference chart does not use —
//! the order of the ECMA-376 schema sequences.

use crate::error::ChartError;
use crate::spec::{
    Axis, ChartKind, ChartPart, ChartSpec, DataLabelPosition, DataLabels, MarkerSymbol, Series,
    SeriesName,
};
use crate::xml::escape;

/// The two axis ids every axed chart uses. Arbitrary but stable: what matters
/// is that each axis names the other as its `crossAx`.
const CATEGORY_AXIS_ID: u32 = 111;
const VALUE_AXIS_ID: u32 = 222;

/// EMU in a point, for line widths.
const EMU_PER_POINT: f64 = 12_700.0;
/// The widest line the schema allows, in EMU (1584 pt).
const MAX_LINE_EMU: i64 = 20_116_800;

const HEADER: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<c:chartSpace"#,
    r#" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart""#,
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    // The schema default is *true*: without this, some readers round the
    // chart's border.
    r#"<c:roundedCorners val="0"/>"#,
);

/// The plot-element family a kind renders as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Bar {
        horizontal: bool,
        grouping: &'static str,
    },
    Line,
    Area {
        grouping: &'static str,
    },
    Scatter,
    Pie,
    Doughnut,
    Radar,
}

fn family(kind: ChartKind) -> Family {
    match kind {
        ChartKind::BarClustered => Family::Bar {
            horizontal: true,
            grouping: "clustered",
        },
        ChartKind::BarStacked => Family::Bar {
            horizontal: true,
            grouping: "stacked",
        },
        ChartKind::BarPercentStacked => Family::Bar {
            horizontal: true,
            grouping: "percentStacked",
        },
        ChartKind::ColumnClustered => Family::Bar {
            horizontal: false,
            grouping: "clustered",
        },
        ChartKind::ColumnStacked => Family::Bar {
            horizontal: false,
            grouping: "stacked",
        },
        ChartKind::ColumnPercentStacked => Family::Bar {
            horizontal: false,
            grouping: "percentStacked",
        },
        ChartKind::Line | ChartKind::LineMarkers => Family::Line,
        ChartKind::Area => Family::Area {
            grouping: "standard",
        },
        ChartKind::AreaStacked => Family::Area {
            grouping: "stacked",
        },
        ChartKind::AreaPercentStacked => Family::Area {
            grouping: "percentStacked",
        },
        ChartKind::Scatter | ChartKind::ScatterLines => Family::Scatter,
        ChartKind::Pie => Family::Pie,
        ChartKind::Doughnut => Family::Doughnut,
        ChartKind::Radar => Family::Radar,
    }
}

pub fn chart_space(spec: &ChartSpec) -> Result<ChartPart, ChartError> {
    if spec.series.is_empty() {
        return Err(ChartError::NoSeries);
    }
    validate(spec)?;

    let family = family(spec.kind);
    let mut out = String::with_capacity(2048);
    out.push_str(HEADER);
    out.push_str("<c:chart>");
    if let Some(title) = &spec.title {
        out.push_str(&title_xml(title));
    }
    out.push_str("<c:plotArea><c:layout/>");
    out.push_str(&plot_xml(spec, family));
    if spec.kind.has_axes() {
        out.push_str(&axes_xml(spec, family));
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

// --- validation -----------------------------------------------------------

/// Refuses what Excel would repair away. A damaged-file prompt is the best
/// case; the worse one is a chart that silently vanishes.
fn validate(spec: &ChartSpec) -> Result<(), ChartError> {
    let family = family(spec.kind);

    for series in &spec.series {
        if let Some(color) = &series.color {
            check_color(color)?;
        }
        if let Some(points) = series.line_width_pt {
            // Non-finite widths become -1, which no range accepts.
            let emu = if points.is_finite() {
                (points * EMU_PER_POINT).round() as i64
            } else {
                -1
            };
            check_range("line width in EMU", emu, 0, MAX_LINE_EMU)?;
        }
        if let Some((_, size)) = series.marker {
            check_range("marker size", i64::from(size), 2, 72)?;
        }
    }

    if spec.kind.has_axes() {
        check_axis(&spec.category_axis)?;
        check_axis(&spec.value_axis)?;
    }

    if let Family::Bar { .. } = family {
        check_range("gap width", i64::from(spec.gap_width), 0, 500)?;
        if let Some(overlap) = spec.overlap {
            check_range("overlap", i64::from(overlap), -100, 100)?;
        }
    }
    if family == Family::Doughnut {
        check_range("hole size", i64::from(spec.hole_size), 10, 90)?;
    }
    if matches!(family, Family::Pie | Family::Doughnut) {
        check_range(
            "first slice angle",
            i64::from(spec.first_slice_angle),
            0,
            360,
        )?;
    }

    if let Some(position) = spec.data_labels.as_ref().and_then(|labels| labels.position) {
        if !label_positions(family).contains(&position) {
            return Err(ChartError::InvalidDataLabelPosition {
                position: position.name(),
            });
        }
    }
    Ok(())
}

fn check_range(field: &'static str, value: i64, min: i64, max: i64) -> Result<(), ChartError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ChartError::OutOfRange {
            field,
            value,
            min,
            max,
        })
    }
}

fn check_color(color: &str) -> Result<(), ChartError> {
    if color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ChartError::InvalidColor(color.to_string()))
    }
}

fn check_axis(axis: &Axis) -> Result<(), ChartError> {
    let reason = |reason| Err(ChartError::InvalidAxisRange { reason });
    for value in [axis.min, axis.max, axis.major_unit].into_iter().flatten() {
        if !value.is_finite() {
            return reason("a value is not finite");
        }
    }
    if let (Some(min), Some(max)) = (axis.min, axis.max) {
        if min >= max {
            return reason("the minimum is not below the maximum");
        }
    }
    if axis.major_unit.is_some_and(|unit| unit <= 0.0) {
        return reason("the major unit is not positive");
    }
    Ok(())
}

/// The label positions Excel accepts for a kind. Anything else — even a
/// position that is fine on a sibling kind — makes it report the file damaged.
fn label_positions(family: Family) -> &'static [DataLabelPosition] {
    use DataLabelPosition::*;
    match family {
        Family::Bar {
            grouping: "clustered",
            ..
        } => &[Center, InsideEnd, InsideBase, OutsideEnd],
        Family::Bar { .. } => &[Center, InsideEnd, InsideBase],
        Family::Line | Family::Scatter => &[Center, Above, Below, Left, Right],
        Family::Pie => &[Center, InsideEnd, OutsideEnd, BestFit],
        // No position element is legal on these.
        Family::Area { .. } | Family::Doughnut | Family::Radar => &[],
    }
}

// --- plot ------------------------------------------------------------------

fn plot_xml(spec: &ChartSpec, family: Family) -> String {
    let series: String = spec
        .series
        .iter()
        .enumerate()
        .map(|(index, series)| series_xml(spec.kind, family, index, series))
        .collect();
    let labels = spec
        .data_labels
        .as_ref()
        .map(data_labels_xml)
        .unwrap_or_default();

    match family {
        Family::Bar {
            horizontal,
            grouping,
        } => {
            let direction = if horizontal { "bar" } else { "col" };
            let default_overlap = (grouping != "clustered").then_some(100);
            let overlap_xml = spec
                .overlap
                .or(default_overlap)
                .map(|value| format!(r#"<c:overlap val="{value}"/>"#))
                .unwrap_or_default();
            format!(
                r#"<c:barChart><c:barDir val="{direction}"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}{labels}<c:gapWidth val="{gap}"/>{overlap_xml}{axes}</c:barChart>"#,
                gap = spec.gap_width,
                axes = axis_ids(),
            )
        }
        Family::Line => {
            let marker = i32::from(spec.kind == ChartKind::LineMarkers);
            format!(
                r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>{series}{labels}<c:marker val="{marker}"/>{axes}</c:lineChart>"#,
                axes = axis_ids(),
            )
        }
        Family::Area { grouping } => format!(
            r#"<c:areaChart><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}{labels}{axes}</c:areaChart>"#,
            axes = axis_ids(),
        ),
        Family::Scatter => format!(
            r#"<c:scatterChart><c:scatterStyle val="lineMarker"/><c:varyColors val="0"/>{series}{labels}{axes}</c:scatterChart>"#,
            axes = axis_ids(),
        ),
        Family::Radar => format!(
            r#"<c:radarChart><c:radarStyle val="marker"/><c:varyColors val="0"/>{series}{labels}{axes}</c:radarChart>"#,
            axes = axis_ids(),
        ),
        Family::Pie => {
            let angle = match spec.first_slice_angle {
                0 => String::new(),
                degrees => format!(r#"<c:firstSliceAng val="{degrees}"/>"#),
            };
            format!(r#"<c:pieChart><c:varyColors val="1"/>{series}{labels}{angle}</c:pieChart>"#)
        }
        Family::Doughnut => format!(
            r#"<c:doughnutChart><c:varyColors val="1"/>{series}{labels}<c:firstSliceAng val="{angle}"/><c:holeSize val="{hole}"/></c:doughnutChart>"#,
            angle = spec.first_slice_angle,
            hole = spec.hole_size,
        ),
    }
}

fn axis_ids() -> String {
    format!(r#"<c:axId val="{CATEGORY_AXIS_ID}"/><c:axId val="{VALUE_AXIS_ID}"/>"#)
}

fn data_labels_xml(labels: &DataLabels) -> String {
    let number_format = labels
        .number_format
        .as_ref()
        .map(|format| {
            format!(
                r#"<c:numFmt formatCode="{}" sourceLinked="0"/>"#,
                escape(format)
            )
        })
        .unwrap_or_default();
    let position = labels
        .position
        .map(|position| format!(r#"<c:dLblPos val="{}"/>"#, position.code()))
        .unwrap_or_default();
    format!(
        r#"<c:dLbls>{number_format}{position}<c:showLegendKey val="0"/><c:showVal val="{}"/><c:showCatName val="{}"/><c:showSerName val="{}"/><c:showPercent val="{}"/><c:showBubbleSize val="0"/></c:dLbls>"#,
        i32::from(labels.value),
        i32::from(labels.category),
        i32::from(labels.series),
        i32::from(labels.percent),
    )
}

// --- series ----------------------------------------------------------------

fn series_xml(kind: ChartKind, family: Family, index: usize, series: &Series) -> String {
    let name = match &series.name {
        SeriesName::Literal(text) => format!("<c:tx><c:v>{}</c:v></c:tx>", escape(text)),
        SeriesName::Reference(reference) => format!(
            "<c:tx><c:strRef><c:f>{}</c:f></c:strRef></c:tx>",
            escape(reference)
        ),
    };
    let shape = shape_xml(kind, family, series);
    let values = format!(
        "<c:numRef><c:f>{}</c:f>{}</c:numRef>",
        escape(&series.values),
        series
            .values_cache
            .as_deref()
            .map(num_cache)
            .unwrap_or_default(),
    );

    let head = format!(r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/>{name}"#);

    if family == Family::Scatter {
        let x = series
            .categories
            .as_ref()
            .map(|reference| {
                // A scatter's x values are numbers, so a cache is numeric too.
                let cache = series
                    .categories_cache
                    .as_ref()
                    .map(|labels| {
                        let numbers: Vec<f64> = labels
                            .iter()
                            .map(|label| label.parse().unwrap_or(f64::NAN))
                            .collect();
                        num_cache(&numbers)
                    })
                    .unwrap_or_default();
                format!(
                    "<c:xVal><c:numRef><c:f>{}</c:f>{cache}</c:numRef></c:xVal>",
                    escape(reference)
                )
            })
            .unwrap_or_default();
        let smooth = smooth_xml(series);
        return format!("{head}{shape}{x}<c:yVal>{values}</c:yVal>{smooth}</c:ser>");
    }

    let categories = series
        .categories
        .as_ref()
        .map(|reference| {
            let cache = series
                .categories_cache
                .as_deref()
                .map(str_cache)
                .unwrap_or_default();
            format!(
                "<c:cat><c:strRef><c:f>{}</c:f>{cache}</c:strRef></c:cat>",
                escape(reference)
            )
        })
        .unwrap_or_default();
    let smooth = if family == Family::Line {
        smooth_xml(series)
    } else {
        String::new()
    };
    format!("{head}{shape}{categories}<c:val>{values}</c:val>{smooth}</c:ser>")
}

fn smooth_xml(series: &Series) -> String {
    if series.smooth {
        r#"<c:smooth val="1"/>"#.to_string()
    } else {
        String::new()
    }
}

/// `<c:spPr>` and, for the kinds that have one, `<c:marker>` — in that order,
/// which is the order of every series sequence in the schema.
fn shape_xml(kind: ChartKind, family: Family, series: &Series) -> String {
    let color = series.color.as_deref().map(str::to_ascii_uppercase);
    let solid = |rgb: &str| format!(r#"<a:solidFill><a:srgbClr val="{rgb}"/></a:solidFill>"#);
    let width = series
        .line_width_pt
        .map(|points| format!(r#" w="{}""#, (points * EMU_PER_POINT).round() as i64))
        .unwrap_or_default();

    match family {
        Family::Bar { .. } | Family::Area { .. } => color
            .map(|rgb| format!("<c:spPr>{}</c:spPr>", solid(&rgb)))
            .unwrap_or_default(),
        Family::Pie | Family::Doughnut => String::new(),
        Family::Line | Family::Radar | Family::Scatter => {
            let line = if kind == ChartKind::Scatter {
                // Markers only: the connecting line is switched off.
                Some(format!("<a:ln{width}><a:noFill/></a:ln>"))
            } else if color.is_some() || !width.is_empty() {
                let fill = color.as_deref().map(solid).unwrap_or_default();
                Some(format!(r#"<a:ln{width} cap="rnd">{fill}<a:round/></a:ln>"#))
            } else {
                None
            };
            let sp_pr = line
                .map(|line| format!("<c:spPr>{line}</c:spPr>"))
                .unwrap_or_default();
            format!("{sp_pr}{}", marker_xml(kind, series, color.as_deref()))
        }
    }
}

fn marker_xml(kind: ChartKind, series: &Series, color: Option<&str>) -> String {
    let marker_fill = color
        .map(|rgb| {
            format!(
                r#"<c:spPr><a:solidFill><a:srgbClr val="{rgb}"/></a:solidFill><a:ln w="9525"><a:solidFill><a:srgbClr val="{rgb}"/></a:solidFill></a:ln></c:spPr>"#
            )
        })
        .unwrap_or_default();

    match series.marker {
        Some((symbol, size)) => format!(
            r#"<c:marker><c:symbol val="{}"/><c:size val="{size}"/>{marker_fill}</c:marker>"#,
            symbol.code()
        ),
        // Plain `Line` and `Radar` mean "no markers"; Excel writes that on
        // every series rather than relying on the chart-level flag.
        None if matches!(kind, ChartKind::Line | ChartKind::Radar) => format!(
            r#"<c:marker><c:symbol val="{}"/></c:marker>"#,
            MarkerSymbol::None.code()
        ),
        None if !marker_fill.is_empty() => format!("<c:marker>{marker_fill}</c:marker>"),
        None => String::new(),
    }
}

fn str_cache(labels: &[String]) -> String {
    let points: String = labels
        .iter()
        .enumerate()
        .map(|(index, label)| format!(r#"<c:pt idx="{index}"><c:v>{}</c:v></c:pt>"#, escape(label)))
        .collect();
    format!(
        r#"<c:strCache><c:ptCount val="{}"/>{points}</c:strCache>"#,
        labels.len()
    )
}

/// Non-finite values are left out, which Excel reads as a blank cell.
fn num_cache(values: &[f64]) -> String {
    let points: String = values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .map(|(index, value)| format!(r#"<c:pt idx="{index}"><c:v>{value}</c:v></c:pt>"#))
        .collect();
    format!(
        r#"<c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="{}"/>{points}</c:numCache>"#,
        values.len()
    )
}

// --- titles and axes ---------------------------------------------------------

fn title_xml(text: &str) -> String {
    format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title><c:autoTitleDeleted val="0"/>"#,
        escape(text)
    )
}

/// What sort of `<c:...Ax>` element an axis is written as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AxisKind {
    Category,
    Value,
}

fn axes_xml(spec: &ChartSpec, family: Family) -> String {
    let (category_pos, value_pos) = match family {
        // A horizontal bar chart reads sideways: categories run up the left,
        // values along the bottom.
        Family::Bar {
            horizontal: true, ..
        } => ("l", "b"),
        _ => ("b", "l"),
    };
    let cross_between = match family {
        Family::Area { .. } | Family::Scatter => "midCat",
        _ => "between",
    };
    // A scatter has two value axes; the "category" axis holds the x values.
    let first = if family == Family::Scatter {
        AxisKind::Value
    } else {
        AxisKind::Category
    };

    let mut out = axis_xml(
        first,
        CATEGORY_AXIS_ID,
        VALUE_AXIS_ID,
        category_pos,
        &spec.category_axis,
        cross_between,
    );
    out.push_str(&axis_xml(
        AxisKind::Value,
        VALUE_AXIS_ID,
        CATEGORY_AXIS_ID,
        value_pos,
        &spec.value_axis,
        cross_between,
    ));
    out
}

fn axis_xml(
    kind: AxisKind,
    id: u32,
    cross_id: u32,
    position: &str,
    axis: &Axis,
    cross_between: &str,
) -> String {
    let tag = if kind == AxisKind::Category {
        "catAx"
    } else {
        "valAx"
    };
    // Bounds and unit mean nothing on a category axis; Excel ignores them
    // there, so they are not written.
    let (max, min) = if kind == AxisKind::Value {
        (
            axis.max
                .map(|v| format!(r#"<c:max val="{v}"/>"#))
                .unwrap_or_default(),
            axis.min
                .map(|v| format!(r#"<c:min val="{v}"/>"#))
                .unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    };
    let orientation = if axis.reversed { "maxMin" } else { "minMax" };
    let gridlines = if axis.major_gridlines {
        "<c:majorGridlines/>"
    } else {
        ""
    };
    let number_format = axis
        .number_format
        .as_ref()
        .map(|format| {
            format!(
                r#"<c:numFmt formatCode="{}" sourceLinked="0"/>"#,
                escape(format)
            )
        })
        .unwrap_or_default();
    let tail = if kind == AxisKind::Value {
        let unit = axis
            .major_unit
            .map(|v| format!(r#"<c:majorUnit val="{v}"/>"#))
            .unwrap_or_default();
        format!(r#"<c:crossBetween val="{cross_between}"/>{unit}"#)
    } else {
        String::new()
    };
    format!(
        r#"<c:{tag}><c:axId val="{id}"/><c:scaling><c:orientation val="{orientation}"/>{max}{min}</c:scaling><c:delete val="{deleted}"/><c:axPos val="{position}"/>{gridlines}{title}{number_format}<c:crossAx val="{cross_id}"/>{tail}</c:{tag}>"#,
        deleted = i32::from(!axis.visible),
        title = axis_title_xml(axis),
    )
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
