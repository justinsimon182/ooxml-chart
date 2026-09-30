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
    AreaStyle, Axis, ChartKind, ChartPart, ChartSpec, DataLabelPosition, DataLabels, ErrorAmount,
    ErrorAxis, ErrorBarSide, ErrorBars, ErrorValues, MarkerSymbol, Paint, Plot, PointFormat,
    PointLabel, Series, SeriesName, TextStyle, TickLabels, Trendline, TrendlineKind,
};
use crate::xml::escape;

/// The primary axis pair. Arbitrary but stable: what matters is that each axis
/// names the other as its `crossAx`.
const CATEGORY_AXIS_ID: u32 = 111;
const VALUE_AXIS_ID: u32 = 222;
/// The secondary pair, used by plots on the right-hand value axis.
const SECONDARY_CATEGORY_AXIS_ID: u32 = 333;
const SECONDARY_VALUE_AXIS_ID: u32 = 444;

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
    Bubble,
    Pie,
    Doughnut,
    Radar {
        filled: bool,
    },
}

fn family(kind: ChartKind) -> Family {
    let bar = |horizontal, grouping| Family::Bar {
        horizontal,
        grouping,
    };
    match kind {
        ChartKind::BarClustered => bar(true, "clustered"),
        ChartKind::BarStacked => bar(true, "stacked"),
        ChartKind::BarPercentStacked => bar(true, "percentStacked"),
        ChartKind::ColumnClustered => bar(false, "clustered"),
        ChartKind::ColumnStacked => bar(false, "stacked"),
        ChartKind::ColumnPercentStacked => bar(false, "percentStacked"),
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
        ChartKind::Bubble => Family::Bubble,
        ChartKind::Pie => Family::Pie,
        ChartKind::Doughnut => Family::Doughnut,
        ChartKind::Radar => Family::Radar { filled: false },
        ChartKind::RadarFilled => Family::Radar { filled: true },
    }
}

/// One drawn plot element: the chart's own series, or an added [`Plot`].
struct PlotRef<'a> {
    kind: ChartKind,
    family: Family,
    series: &'a [Series],
    secondary: bool,
}

impl<'a> PlotRef<'a> {
    fn of(kind: ChartKind, series: &'a [Series], secondary: bool) -> Self {
        Self {
            kind,
            family: family(kind),
            series,
            secondary,
        }
    }

    fn axis_ids(&self) -> (u32, u32) {
        if self.secondary {
            (SECONDARY_CATEGORY_AXIS_ID, SECONDARY_VALUE_AXIS_ID)
        } else {
            (CATEGORY_AXIS_ID, VALUE_AXIS_ID)
        }
    }
}

fn plots(spec: &ChartSpec) -> Vec<PlotRef<'_>> {
    let mut all = vec![PlotRef::of(spec.kind, &spec.series, false)];
    all.extend(spec.extra_plots.iter().map(
        |Plot {
             kind,
             series,
             secondary_axis,
         }| PlotRef::of(*kind, series, *secondary_axis),
    ));
    all
}

pub fn chart_space(spec: &ChartSpec) -> Result<ChartPart, ChartError> {
    let plots = plots(spec);
    if plots.iter().any(|plot| plot.series.is_empty()) {
        return Err(ChartError::NoSeries);
    }
    validate(spec, &plots)?;

    let mut out = String::with_capacity(4096);
    out.push_str(HEADER);
    out.push_str("<c:chart>");
    if let Some(title) = &spec.title {
        out.push_str(&title_xml(title, spec.title_style.as_ref()));
    }
    out.push_str("<c:plotArea><c:layout/>");
    let mut first_index = 0;
    for plot in &plots {
        out.push_str(&plot_xml(spec, plot, first_index));
        first_index += plot.series.len();
    }
    if spec.kind.has_axes() {
        out.push_str(&axes_xml(spec, &plots));
    }
    if let Some(style) = &spec.plot_area {
        out.push_str(&sp_pr_xml(style));
    }
    out.push_str("</c:plotArea>");
    if let Some(code) = spec.legend.code() {
        out.push_str(&format!(
            r#"<c:legend><c:legendPos val="{code}"/><c:overlay val="{}"/>{}</c:legend>"#,
            i32::from(spec.legend_overlay),
            spec.legend_style
                .as_ref()
                .map(|style| tx_pr_xml(Some(style), None))
                .unwrap_or_default(),
        ));
    }
    out.push_str(r#"<c:plotVisOnly val="1"/>"#);
    out.push_str("</c:chart>");
    if let Some(style) = &spec.chart_area {
        out.push_str(&sp_pr_xml(style));
    }
    if let Some(style) = &spec.text_style {
        out.push_str(&tx_pr_xml(Some(style), None));
    }
    out.push_str("</c:chartSpace>");

    Ok(ChartPart {
        xml: out.into_bytes(),
        content_type: ChartPart::CONTENT_TYPE,
    })
}

// --- validation -----------------------------------------------------------

fn unsupported(what: String) -> Result<(), ChartError> {
    Err(ChartError::Unsupported { what })
}

/// Refuses what Excel would repair away. A damaged-file prompt is the best
/// case; the worse one is a chart that silently vanishes.
fn validate(spec: &ChartSpec, plots: &[PlotRef<'_>]) -> Result<(), ChartError> {
    let primary = &plots[0];

    // Text and area styling.
    for style in [&spec.title_style, &spec.text_style, &spec.legend_style]
        .into_iter()
        .flatten()
    {
        check_text_style(style)?;
    }
    if let Some(style) = spec.data_labels.as_ref().and_then(|l| l.style.as_ref()) {
        check_text_style(style)?;
    }
    for style in [&spec.chart_area, &spec.plot_area].into_iter().flatten() {
        check_area_style(style)?;
    }

    // Combinations.
    if !spec.extra_plots.is_empty() {
        for plot in plots {
            if !matches!(
                plot.family,
                Family::Bar {
                    horizontal: false,
                    ..
                } | Family::Line
                    | Family::Area { .. }
            ) {
                return unsupported(format!(
                    "combining {:?}: only column, line and area kinds share a category axis",
                    plot.kind
                ));
            }
        }
    }
    if plots.iter().skip(1).any(|plot| plot.secondary) {
        check_axis(&spec.secondary_value_axis)?;
    }

    // Per-series settings.
    for plot in plots {
        for series in plot.series {
            check_series(plot, series)?;
        }
        if let Some(position) = spec.data_labels.as_ref().and_then(|l| l.position) {
            if !label_positions(plot.family).contains(&position) {
                return Err(ChartError::InvalidDataLabelPosition {
                    position: position.name(),
                });
            }
        }
    }

    // Axes.
    if primary.kind.has_axes() {
        let dates_ok = matches!(
            primary.family,
            Family::Bar { .. } | Family::Line | Family::Area { .. }
        );
        if spec.category_axis.date_unit.is_some() && !dates_ok {
            return unsupported(format!("a date axis on {:?}", primary.kind));
        }
        check_axis(&spec.category_axis)?;
        check_axis(&spec.value_axis)?;
    }

    // Bar geometry and round-chart settings.
    if plots
        .iter()
        .any(|plot| matches!(plot.family, Family::Bar { .. }))
    {
        check_range("gap width", i64::from(spec.gap_width), 0, 500)?;
        if let Some(overlap) = spec.overlap {
            check_range("overlap", i64::from(overlap), -100, 100)?;
        }
    }
    if primary.family == Family::Doughnut {
        check_range("hole size", i64::from(spec.hole_size), 10, 90)?;
    }
    if matches!(primary.family, Family::Pie | Family::Doughnut) {
        check_range(
            "first slice angle",
            i64::from(spec.first_slice_angle),
            0,
            360,
        )?;
    }
    if primary.family == Family::Bubble {
        check_range("bubble scale", i64::from(spec.bubble_scale), 0, 300)?;
    }
    Ok(())
}

fn check_series(plot: &PlotRef<'_>, series: &Series) -> Result<(), ChartError> {
    if let Some(color) = &series.color {
        check_color(color)?;
    }
    if let Some(points) = series.line_width_pt {
        check_line_width(points)?;
    }
    if let Some((_, size)) = series.marker {
        check_range("marker size", i64::from(size), 2, 72)?;
    }
    if plot.family == Family::Bubble && series.bubble_sizes.is_none() {
        return Err(ChartError::MissingBubbleSizes);
    }
    for (_, point) in &series.points {
        if let Some(color) = &point.color {
            check_color(color)?;
        }
        if let Some(explosion) = point.explosion {
            check_range("explosion", i64::from(explosion), 0, 400)?;
        }
        if let Some((_, size)) = point.marker {
            check_range("marker size", i64::from(size), 2, 72)?;
        }
    }
    for (_, label) in &series.point_labels {
        if label.hidden && label.text.is_some() {
            return unsupported("a data label that is both hidden and has text".to_string());
        }
        if let Some(style) = &label.style {
            check_text_style(style)?;
        }
        if let Some(position) = label.position {
            if !label_positions(plot.family).contains(&position) {
                return Err(ChartError::InvalidDataLabelPosition {
                    position: position.name(),
                });
            }
        }
    }
    check_error_bars(plot, series)?;
    if !series.trendlines.is_empty() {
        let allowed = match plot.family {
            Family::Bar { grouping, .. } => grouping == "clustered",
            Family::Area { grouping } => grouping == "standard",
            Family::Line | Family::Scatter | Family::Bubble => true,
            Family::Pie | Family::Doughnut | Family::Radar { .. } => false,
        };
        if !allowed {
            return unsupported(format!("a trendline on {:?}", plot.kind));
        }
        for trendline in &series.trendlines {
            check_trendline(trendline)?;
        }
    }
    Ok(())
}

fn check_error_bars(plot: &PlotRef<'_>, series: &Series) -> Result<(), ChartError> {
    if series.error_bars.is_empty() {
        return Ok(());
    }
    let has_x = matches!(plot.family, Family::Scatter | Family::Bubble);
    if matches!(
        plot.family,
        Family::Pie | Family::Doughnut | Family::Radar { .. }
    ) {
        return unsupported(format!("error bars on {:?}", plot.kind));
    }
    let mut seen = [false; 2];
    for bars in &series.error_bars {
        if bars.axis == ErrorAxis::X && !has_x {
            return unsupported(format!("x error bars on {:?}", plot.kind));
        }
        let slot = &mut seen[usize::from(bars.axis == ErrorAxis::X)];
        if std::mem::replace(slot, true) {
            return unsupported("two error bars in the same direction".to_string());
        }
        check_error_bar(bars)?;
    }
    Ok(())
}

fn check_error_bar(bars: &ErrorBars) -> Result<(), ChartError> {
    let bad_amount = |field: &'static str| ChartError::OutOfRange {
        field,
        value: -1,
        min: 0,
        max: i64::MAX,
    };
    match &bars.amount {
        ErrorAmount::Fixed(value) if !value.is_finite() || *value < 0.0 => {
            return Err(bad_amount("error bar amount"));
        }
        ErrorAmount::Percentage(value) if !value.is_finite() || *value < 0.0 => {
            return Err(bad_amount("error bar percentage"));
        }
        ErrorAmount::StdDev(value) if !value.is_finite() || *value <= 0.0 => {
            return Err(bad_amount("error bar standard deviations"));
        }
        ErrorAmount::Custom { plus, minus } => {
            let wanted = |side: ErrorBarSide| bars.side == ErrorBarSide::Both || bars.side == side;
            for (present, needed, which) in [
                (plus, wanted(ErrorBarSide::Plus), "plus"),
                (minus, wanted(ErrorBarSide::Minus), "minus"),
            ] {
                match present {
                    None if needed => {
                        return unsupported(format!(
                            "custom error bars drawn on the {which} side with no {which} amounts"
                        ));
                    }
                    Some(ErrorValues::Reference(reference)) if reference.trim().is_empty() => {
                        return unsupported("an empty custom error bar reference".to_string());
                    }
                    Some(ErrorValues::Literal(values))
                        if values.is_empty()
                            || values.iter().any(|v| !v.is_finite() || *v < 0.0) =>
                    {
                        return Err(bad_amount("custom error bar amount"));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    if let Some(color) = &bars.color {
        check_color(color)?;
    }
    if let Some(points) = bars.width_pt {
        check_line_width(points)?;
    }
    Ok(())
}

fn check_trendline(trendline: &Trendline) -> Result<(), ChartError> {
    match trendline.kind {
        TrendlineKind::Polynomial(order) => {
            check_range("polynomial order", i64::from(order), 2, 6)?
        }
        TrendlineKind::MovingAverage(period) => {
            check_range("moving average period", i64::from(period), 2, 255)?
        }
        _ => {}
    }
    if let Some(color) = &trendline.color {
        check_color(color)?;
    }
    if let Some(points) = trendline.width_pt {
        check_line_width(points)?;
    }
    for value in [trendline.forward, trendline.backward]
        .into_iter()
        .flatten()
    {
        if !value.is_finite() {
            return Err(ChartError::OutOfRange {
                field: "trendline extension",
                value: -1,
                min: 0,
                max: i64::MAX,
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

fn line_emu(points: f64) -> i64 {
    // Non-finite widths become -1, which no range accepts.
    if points.is_finite() {
        (points * EMU_PER_POINT).round() as i64
    } else {
        -1
    }
}

fn check_line_width(points: f64) -> Result<(), ChartError> {
    check_range("line width in EMU", line_emu(points), 0, MAX_LINE_EMU)
}

fn check_color(color: &str) -> Result<(), ChartError> {
    if color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ChartError::InvalidColor(color.to_string()))
    }
}

fn check_text_style(style: &TextStyle) -> Result<(), ChartError> {
    if let Some(size) = style.size_pt {
        // Hundredths of a point in the file; a NaN size becomes -1.
        let hundredths = if size.is_finite() {
            (size * 100.0).round() as i64
        } else {
            -1
        };
        check_range(
            "font size in hundredths of a point",
            hundredths,
            100,
            400_000,
        )?;
    }
    if let Some(color) = &style.color {
        check_color(color)?;
    }
    Ok(())
}

fn check_area_style(style: &AreaStyle) -> Result<(), ChartError> {
    for paint in [&style.fill, &style.border].into_iter().flatten() {
        if let Paint::Color(color) = paint {
            check_color(color)?;
        }
    }
    if let Some(points) = style.border_width_pt {
        check_line_width(points)?;
    }
    Ok(())
}

fn check_axis(axis: &Axis) -> Result<(), ChartError> {
    let reason = |reason| Err(ChartError::InvalidAxisRange { reason });
    for value in [axis.min, axis.max, axis.major_unit, axis.minor_unit]
        .into_iter()
        .flatten()
    {
        if !value.is_finite() {
            return reason("a value is not finite");
        }
    }
    if let (Some(min), Some(max)) = (axis.min, axis.max) {
        if min >= max {
            return reason("the minimum is not below the maximum");
        }
    }
    if [axis.major_unit, axis.minor_unit]
        .into_iter()
        .flatten()
        .any(|unit| unit <= 0.0)
    {
        return reason("a tick unit is not positive");
    }
    if let Some(base) = axis.log_base {
        check_range("log base", i64::from(base), 2, 1000)?;
        if axis.min.is_some_and(|min| min <= 0.0) {
            return reason("a logarithmic axis needs a positive minimum");
        }
    }
    if let Some(degrees) = axis.label_rotation {
        check_range("label rotation", i64::from(degrees), -90, 90)?;
    }
    for style in [&axis.title_style, &axis.label_style].into_iter().flatten() {
        check_text_style(style)?;
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
        Family::Line | Family::Scatter | Family::Bubble => &[Center, Above, Below, Left, Right],
        Family::Pie => &[Center, InsideEnd, OutsideEnd, BestFit],
        // No position element is legal on these.
        Family::Area { .. } | Family::Doughnut | Family::Radar { .. } => &[],
    }
}

// --- plot ------------------------------------------------------------------

fn plot_xml(spec: &ChartSpec, plot: &PlotRef<'_>, first_index: usize) -> String {
    let series: String = plot
        .series
        .iter()
        .enumerate()
        .map(|(offset, series)| {
            series_xml(
                plot,
                first_index + offset,
                series,
                spec.data_labels.as_ref(),
            )
        })
        .collect();
    let labels = spec
        .data_labels
        .as_ref()
        .map(data_labels_xml)
        .unwrap_or_default();
    let (category_id, value_id) = plot.axis_ids();
    let axes = format!(r#"<c:axId val="{category_id}"/><c:axId val="{value_id}"/>"#);

    match plot.family {
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
            )
        }
        Family::Line => {
            let marker = i32::from(plot.kind == ChartKind::LineMarkers);
            format!(
                r#"<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>{series}{labels}<c:marker val="{marker}"/>{axes}</c:lineChart>"#,
            )
        }
        Family::Area { grouping } => format!(
            r#"<c:areaChart><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}{labels}{axes}</c:areaChart>"#,
        ),
        Family::Scatter => format!(
            r#"<c:scatterChart><c:scatterStyle val="lineMarker"/><c:varyColors val="0"/>{series}{labels}{axes}</c:scatterChart>"#,
        ),
        Family::Bubble => format!(
            r#"<c:bubbleChart><c:varyColors val="0"/>{series}{labels}<c:bubbleScale val="{scale}"/><c:showNegBubbles val="0"/>{axes}</c:bubbleChart>"#,
            scale = spec.bubble_scale,
        ),
        Family::Radar { filled } => {
            let style = if filled { "filled" } else { "marker" };
            format!(
                r#"<c:radarChart><c:radarStyle val="{style}"/><c:varyColors val="0"/>{series}{labels}{axes}</c:radarChart>"#,
            )
        }
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

fn data_labels_xml(labels: &DataLabels) -> String {
    format!("<c:dLbls>{}</c:dLbls>", label_group_xml(labels))
}

/// The settings common to a `<c:dLbls>` and a `<c:dLbl>`, in schema order.
fn label_group_xml(labels: &DataLabels) -> String {
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
    let tx_pr = labels
        .style
        .as_ref()
        .map(|style| tx_pr_xml(Some(style), None))
        .unwrap_or_default();
    let position = labels
        .position
        .map(|position| format!(r#"<c:dLblPos val="{}"/>"#, position.code()))
        .unwrap_or_default();
    format!(
        r#"{number_format}{tx_pr}{position}{}"#,
        label_flags_xml(labels)
    )
}

fn label_flags_xml(labels: &DataLabels) -> String {
    format!(
        r#"<c:showLegendKey val="0"/><c:showVal val="{}"/><c:showCatName val="{}"/><c:showSerName val="{}"/><c:showPercent val="{}"/><c:showBubbleSize val="0"/>"#,
        i32::from(labels.value),
        i32::from(labels.category),
        i32::from(labels.series),
        i32::from(labels.percent),
    )
}

/// A series' own `<c:dLbls>`, written only when it has point overrides.
///
/// A series-level `<c:dLbls>` replaces the chart-wide one for that series
/// outright, so the group after the `<c:dLbl>` entries repeats the chart-wide
/// settings — or, with none, switches labels off — for the points that are not
/// overridden.
fn series_labels_xml(series: &Series, chart_wide: Option<&DataLabels>) -> String {
    if series.point_labels.is_empty() {
        return String::new();
    }
    let mut overrides: Vec<_> = series.point_labels.iter().collect();
    overrides.sort_by_key(|(index, _)| *index);
    // A point with nothing said about what to show falls back to the
    // chart-wide choice, or to its value when the chart has no labels.
    let inherited = chart_wide.cloned().unwrap_or_else(DataLabels::values);
    let entries: String = overrides
        .into_iter()
        .map(|(index, label)| point_label_xml(*index, label, &inherited))
        .collect();
    let group = chart_wide
        .map(label_group_xml)
        .unwrap_or_else(|| label_flags_xml(&DataLabels::default()));
    format!("<c:dLbls>{entries}{group}</c:dLbls>")
}

fn point_label_xml(index: usize, label: &PointLabel, inherited: &DataLabels) -> String {
    if label.hidden {
        return format!(r#"<c:dLbl><c:idx val="{index}"/><c:delete val="1"/></c:dLbl>"#);
    }
    let style = label.style.as_ref().or(inherited.style.as_ref());
    let text = label
        .text
        .as_ref()
        .map(|text| {
            let run_props = style
                .map(|style| run_props("rPr", r#" lang="en-US""#, style))
                .unwrap_or_default();
            format!(
                "<c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r>{run_props}<a:t>{}</a:t></a:r></a:p></c:rich></c:tx>",
                escape(text)
            )
        })
        .unwrap_or_default();
    // Custom text carries its font on the run; otherwise it is the label's
    // default text properties.
    let body = DataLabels {
        position: label.position.or(inherited.position),
        style: style.filter(|_| label.text.is_none()).cloned(),
        ..inherited.clone()
    };
    let flags_and_format = {
        let group = label_group_xml(&body);
        // Custom text is shown whatever the flags say, but Excel writes at
        // least the value flag alongside it.
        if label.text.is_some() && !(body.value || body.category || body.series || body.percent) {
            label_group_xml(&DataLabels {
                value: true,
                ..body
            })
        } else {
            group
        }
    };
    format!(r#"<c:dLbl><c:idx val="{index}"/>{text}{flags_and_format}</c:dLbl>"#)
}

// --- series ----------------------------------------------------------------

fn solid(rgb: &str) -> String {
    format!(
        r#"<a:solidFill><a:srgbClr val="{}"/></a:solidFill>"#,
        rgb.to_ascii_uppercase()
    )
}

fn series_xml(
    plot: &PlotRef<'_>,
    index: usize,
    series: &Series,
    chart_wide_labels: Option<&DataLabels>,
) -> String {
    let name = match &series.name {
        SeriesName::Literal(text) => format!("<c:tx><c:v>{}</c:v></c:tx>", escape(text)),
        SeriesName::Reference(reference) => format!(
            "<c:tx><c:strRef><c:f>{}</c:f></c:strRef></c:tx>",
            escape(reference)
        ),
    };
    let shape = shape_xml(plot, series);
    let points = format!(
        "{}{}",
        points_xml(plot, series),
        series_labels_xml(series, chart_wide_labels)
    );
    let trendlines: String = series.trendlines.iter().map(trendline_xml).collect();
    let error_bars = error_bars_xml(plot, series);
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

    if matches!(plot.family, Family::Scatter | Family::Bubble) {
        let x = series
            .categories
            .as_ref()
            .map(|reference| {
                // The x values are numbers, so a cache is numeric too.
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
        let tail = if plot.family == Family::Bubble {
            let sizes = series.bubble_sizes.as_deref().unwrap_or_default();
            format!(
                r#"<c:bubbleSize><c:numRef><c:f>{}</c:f></c:numRef></c:bubbleSize><c:bubble3D val="0"/>"#,
                escape(sizes)
            )
        } else {
            smooth_xml(series)
        };
        return format!(
            "{head}{shape}{points}{trendlines}{error_bars}{x}<c:yVal>{values}</c:yVal>{tail}</c:ser>"
        );
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
    let smooth = if plot.family == Family::Line {
        smooth_xml(series)
    } else {
        String::new()
    };
    format!("{head}{shape}{points}{trendlines}{error_bars}{categories}<c:val>{values}</c:val>{smooth}</c:ser>")
}

fn smooth_xml(series: &Series) -> String {
    if series.smooth {
        r#"<c:smooth val="1"/>"#.to_string()
    } else {
        String::new()
    }
}

fn line_width_attr(points: Option<f64>) -> String {
    points
        .map(|points| format!(r#" w="{}""#, line_emu(points)))
        .unwrap_or_default()
}

/// `<c:spPr>` and, for the kinds that have one, `<c:marker>` — in that order,
/// which is the order of every series sequence in the schema.
fn shape_xml(plot: &PlotRef<'_>, series: &Series) -> String {
    let color = series.color.as_deref();
    let width = line_width_attr(series.line_width_pt);

    match plot.family {
        Family::Bar { .. }
        | Family::Area { .. }
        | Family::Bubble
        | Family::Radar { filled: true } => color
            .map(|rgb| format!("<c:spPr>{}</c:spPr>", solid(rgb)))
            .unwrap_or_default(),
        Family::Pie | Family::Doughnut => String::new(),
        Family::Line | Family::Radar { filled: false } | Family::Scatter => {
            let line = if plot.kind == ChartKind::Scatter {
                // Markers only: the connecting line is switched off.
                Some(format!("<a:ln{width}><a:noFill/></a:ln>"))
            } else if color.is_some() || !width.is_empty() {
                let fill = color.map(solid).unwrap_or_default();
                Some(format!(r#"<a:ln{width} cap="rnd">{fill}<a:round/></a:ln>"#))
            } else {
                None
            };
            let sp_pr = line
                .map(|line| format!("<c:spPr>{line}</c:spPr>"))
                .unwrap_or_default();
            format!("{sp_pr}{}", marker_xml(plot.kind, series, color))
        }
    }
}

fn marker_xml(kind: ChartKind, series: &Series, color: Option<&str>) -> String {
    let marker_fill = color
        .map(|rgb| {
            let rgb = rgb.to_ascii_uppercase();
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

/// `<c:dPt>` elements, in index order, for the kinds where a point has a shape
/// of its own: a slice or bar, or on line-like kinds its marker.
fn points_xml(plot: &PlotRef<'_>, series: &Series) -> String {
    let round = matches!(plot.family, Family::Pie | Family::Doughnut);
    let shaped = round || matches!(plot.family, Family::Bar { .. } | Family::Bubble);
    let marked = matches!(
        plot.family,
        Family::Line | Family::Scatter | Family::Radar { filled: false }
    );
    if !shaped && !marked {
        return String::new();
    }
    let mut points: Vec<_> = series.points.iter().collect();
    points.sort_by_key(|(index, _)| *index);
    points
        .into_iter()
        .filter_map(|(index, point)| {
            if marked {
                let marker = point_marker_xml(point)?;
                return Some(format!(r#"<c:dPt><c:idx val="{index}"/>{marker}</c:dPt>"#));
            }
            let explosion = point
                .explosion
                .filter(|_| round)
                .map(|percent| format!(r#"<c:explosion val="{percent}"/>"#))
                .unwrap_or_default();
            let fill = point
                .color
                .as_deref()
                .map(|rgb| format!("<c:spPr>{}</c:spPr>", solid(rgb)))
                .unwrap_or_default();
            // A point that changes nothing here (e.g. it only sets a marker)
            // would be an empty `<c:dPt>`.
            if explosion.is_empty() && fill.is_empty() {
                return None;
            }
            Some(format!(
                r#"<c:dPt><c:idx val="{index}"/>{explosion}{fill}</c:dPt>"#
            ))
        })
        .collect()
}

/// The `<c:marker>` for one point of a line-like series, or `None` if the point
/// says nothing about its marker.
fn point_marker_xml(point: &PointFormat) -> Option<String> {
    if point.marker.is_none() && point.color.is_none() {
        return None;
    }
    let shape = point
        .marker
        .map(|(symbol, size)| {
            format!(
                r#"<c:symbol val="{}"/><c:size val="{size}"/>"#,
                symbol.code()
            )
        })
        .unwrap_or_default();
    let fill = point
        .color
        .as_deref()
        .map(|rgb| {
            let rgb = rgb.to_ascii_uppercase();
            format!(
                r#"<c:spPr><a:solidFill><a:srgbClr val="{rgb}"/></a:solidFill><a:ln w="9525"><a:solidFill><a:srgbClr val="{rgb}"/></a:solidFill></a:ln></c:spPr>"#
            )
        })
        .unwrap_or_default();
    Some(format!("<c:marker>{shape}{fill}</c:marker>"))
}

/// `<c:errBars>` elements, x before y.
///
/// `errDir` is written for every kind but bar and column, which measure along
/// the value axis only and leave it out, as Excel does.
fn error_bars_xml(plot: &PlotRef<'_>, series: &Series) -> String {
    let mut bars: Vec<_> = series.error_bars.iter().collect();
    bars.sort_by_key(|bars| bars.axis == ErrorAxis::Y);
    bars.into_iter()
        .map(|bars| {
            let direction = match (plot.family, bars.axis) {
                (Family::Bar { .. }, _) => "",
                (_, ErrorAxis::X) => r#"<c:errDir val="x"/>"#,
                (_, ErrorAxis::Y) => r#"<c:errDir val="y"/>"#,
            };
            let side = match bars.side {
                ErrorBarSide::Both => "both",
                ErrorBarSide::Plus => "plus",
                ErrorBarSide::Minus => "minus",
            };
            let (kind, amounts) = match &bars.amount {
                ErrorAmount::Fixed(value) => ("fixedVal", format!(r#"<c:val val="{value}"/>"#)),
                ErrorAmount::Percentage(value) => {
                    ("percentage", format!(r#"<c:val val="{value}"/>"#))
                }
                ErrorAmount::StdDev(value) => ("stdDev", format!(r#"<c:val val="{value}"/>"#)),
                ErrorAmount::StdErr => ("stdErr", String::new()),
                ErrorAmount::Custom { plus, minus } => {
                    let part = |tag: &str, values: &Option<ErrorValues>, used: bool| {
                        values
                            .as_ref()
                            .filter(|_| used)
                            .map(|values| format!("<c:{tag}>{}</c:{tag}>", error_values_xml(values)))
                            .unwrap_or_default()
                    };
                    let both = bars.side == ErrorBarSide::Both;
                    let plus = part("plus", plus, both || bars.side == ErrorBarSide::Plus);
                    let minus = part("minus", minus, both || bars.side == ErrorBarSide::Minus);
                    ("cust", format!("{plus}{minus}"))
                }
            };
            let cap = i32::from(!bars.end_cap);
            let width = line_width_attr(bars.width_pt);
            let sp_pr = if bars.color.is_some() || !width.is_empty() {
                let fill = bars.color.as_deref().map(solid).unwrap_or_default();
                format!(r#"<c:spPr><a:ln{width}>{fill}</a:ln></c:spPr>"#)
            } else {
                String::new()
            };
            format!(
                r#"<c:errBars>{direction}<c:errBarType val="{side}"/><c:errValType val="{kind}"/><c:noEndCap val="{cap}"/>{amounts}{sp_pr}</c:errBars>"#
            )
        })
        .collect()
}

fn error_values_xml(values: &ErrorValues) -> String {
    match values {
        ErrorValues::Reference(reference) => {
            format!("<c:numRef><c:f>{}</c:f></c:numRef>", escape(reference))
        }
        ErrorValues::Literal(values) => {
            let points: String = values
                .iter()
                .enumerate()
                .map(|(index, value)| format!(r#"<c:pt idx="{index}"><c:v>{value}</c:v></c:pt>"#))
                .collect();
            format!(
                r#"<c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="{}"/>{points}</c:numLit>"#,
                values.len()
            )
        }
    }
}

fn trendline_xml(trendline: &Trendline) -> String {
    let name = trendline
        .name
        .as_ref()
        .map(|text| format!("<c:name>{}</c:name>", escape(text)))
        .unwrap_or_default();
    let width = line_width_attr(trendline.width_pt);
    let sp_pr = if trendline.color.is_some() || !width.is_empty() {
        let fill = trendline.color.as_deref().map(solid).unwrap_or_default();
        format!(r#"<c:spPr><a:ln{width} cap="rnd">{fill}</a:ln></c:spPr>"#)
    } else {
        String::new()
    };
    let (kind, order, period) = match trendline.kind {
        TrendlineKind::Linear => ("linear", String::new(), String::new()),
        TrendlineKind::Exponential => ("exp", String::new(), String::new()),
        TrendlineKind::Logarithmic => ("log", String::new(), String::new()),
        TrendlineKind::Power => ("power", String::new(), String::new()),
        TrendlineKind::Polynomial(order) => (
            "poly",
            format!(r#"<c:order val="{order}"/>"#),
            String::new(),
        ),
        TrendlineKind::MovingAverage(period) => (
            "movingAvg",
            String::new(),
            format!(r#"<c:period val="{period}"/>"#),
        ),
    };
    let forward = trendline
        .forward
        .map(|value| format!(r#"<c:forward val="{value}"/>"#))
        .unwrap_or_default();
    let backward = trendline
        .backward
        .map(|value| format!(r#"<c:backward val="{value}"/>"#))
        .unwrap_or_default();
    format!(
        r#"<c:trendline>{name}{sp_pr}<c:trendlineType val="{kind}"/>{order}{period}{forward}{backward}<c:dispRSqr val="{}"/><c:dispEq val="{}"/></c:trendline>"#,
        i32::from(trendline.show_r_squared),
        i32::from(trendline.show_equation),
    )
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

// --- text and area formatting -------------------------------------------------

/// The attributes and children of a run-properties element for `style`.
/// Children come in schema order: fill, then typeface.
fn run_props(tag: &str, extra_attrs: &str, style: &TextStyle) -> String {
    let mut attrs = String::from(extra_attrs);
    if let Some(size) = style.size_pt {
        attrs.push_str(&format!(r#" sz="{}""#, (size * 100.0).round() as i64));
    }
    if let Some(bold) = style.bold {
        attrs.push_str(&format!(r#" b="{}""#, i32::from(bold)));
    }
    if let Some(italic) = style.italic {
        attrs.push_str(&format!(r#" i="{}""#, i32::from(italic)));
    }
    let fill = style.color.as_deref().map(solid).unwrap_or_default();
    let font = style
        .font
        .as_ref()
        .map(|name| format!(r#"<a:latin typeface="{}"/>"#, escape(name)))
        .unwrap_or_default();
    if fill.is_empty() && font.is_empty() {
        format!("<a:{tag}{attrs}/>")
    } else {
        format!("<a:{tag}{attrs}>{fill}{font}</a:{tag}>")
    }
}

/// A `<c:txPr>`: default text properties for a whole element, optionally with
/// its labels rotated.
fn tx_pr_xml(style: Option<&TextStyle>, rotation: Option<i16>) -> String {
    let body = match rotation {
        Some(degrees) => format!(
            r#"<a:bodyPr rot="{}" vert="horz"/>"#,
            i32::from(degrees) * 60_000
        ),
        None => "<a:bodyPr/>".to_string(),
    };
    let default_props = match style {
        Some(style) => run_props("defRPr", "", style),
        None => "<a:defRPr/>".to_string(),
    };
    format!(
        r#"<c:txPr>{body}<a:lstStyle/><a:p><a:pPr>{default_props}</a:pPr><a:endParaRPr lang="en-US"/></a:p></c:txPr>"#
    )
}

fn sp_pr_xml(style: &AreaStyle) -> String {
    let fill = match &style.fill {
        Some(Paint::Color(rgb)) => solid(rgb),
        Some(Paint::None) => "<a:noFill/>".to_string(),
        None => String::new(),
    };
    let width = line_width_attr(style.border_width_pt);
    let line = match &style.border {
        Some(Paint::Color(rgb)) => format!("<a:ln{width}>{}</a:ln>", solid(rgb)),
        Some(Paint::None) => format!("<a:ln{width}><a:noFill/></a:ln>"),
        None if !width.is_empty() => format!("<a:ln{width}/>"),
        None => String::new(),
    };
    format!("<c:spPr>{fill}{line}</c:spPr>")
}

fn rich_title(text: &str, style: Option<&TextStyle>) -> String {
    let run_props = style
        .map(|style| run_props("rPr", r#" lang="en-US""#, style))
        .unwrap_or_default();
    format!(
        "<c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r>{run_props}<a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/>",
        escape(text)
    )
}

fn title_xml(text: &str, style: Option<&TextStyle>) -> String {
    format!(
        r#"<c:title>{}</c:title><c:autoTitleDeleted val="0"/>"#,
        rich_title(text, style)
    )
}

// --- axes ----------------------------------------------------------------------

/// What sort of axis element an axis is written as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AxisKind {
    Category,
    Date,
    Value,
}

struct AxisPlacement<'a> {
    kind: AxisKind,
    id: u32,
    cross_id: u32,
    position: &'a str,
    cross_between: &'a str,
}

fn axes_xml(spec: &ChartSpec, plots: &[PlotRef<'_>]) -> String {
    let family = plots[0].family;
    let (category_pos, value_pos) = match family {
        // A horizontal bar chart reads sideways: categories run up the left,
        // values along the bottom.
        Family::Bar {
            horizontal: true, ..
        } => ("l", "b"),
        _ => ("b", "l"),
    };
    let cross_between = match family {
        Family::Area { .. } | Family::Scatter | Family::Bubble => "midCat",
        _ => "between",
    };
    // A scatter or bubble chart has two value axes; the "category" axis holds
    // the x values.
    let first = if matches!(family, Family::Scatter | Family::Bubble) {
        AxisKind::Value
    } else if spec.category_axis.date_unit.is_some() {
        AxisKind::Date
    } else {
        AxisKind::Category
    };

    let mut out = axis_xml(
        &AxisPlacement {
            kind: first,
            id: CATEGORY_AXIS_ID,
            cross_id: VALUE_AXIS_ID,
            position: category_pos,
            cross_between,
        },
        &spec.category_axis,
    );
    out.push_str(&axis_xml(
        &AxisPlacement {
            kind: AxisKind::Value,
            id: VALUE_AXIS_ID,
            cross_id: CATEGORY_AXIS_ID,
            position: value_pos,
            cross_between,
        },
        &spec.value_axis,
    ));

    if plots.iter().any(|plot| plot.secondary) {
        // The right-hand value axis crosses the (hidden) secondary category
        // axis at its far end, which is what puts it on the right.
        let mut value_axis = spec.secondary_value_axis.clone();
        value_axis.crosses_max = true;
        out.push_str(&axis_xml(
            &AxisPlacement {
                kind: AxisKind::Value,
                id: SECONDARY_VALUE_AXIS_ID,
                cross_id: SECONDARY_CATEGORY_AXIS_ID,
                position: "r",
                cross_between,
            },
            &value_axis,
        ));
        out.push_str(&axis_xml(
            &AxisPlacement {
                kind: first,
                id: SECONDARY_CATEGORY_AXIS_ID,
                cross_id: SECONDARY_VALUE_AXIS_ID,
                position: category_pos,
                cross_between,
            },
            &Axis {
                date_unit: spec.category_axis.date_unit,
                ..Axis::default().hidden()
            },
        ));
    }
    out
}

fn axis_xml(place: &AxisPlacement<'_>, axis: &Axis) -> String {
    let AxisPlacement {
        kind,
        id,
        cross_id,
        position,
        cross_between,
    } = *place;
    let tag = match kind {
        AxisKind::Category => "catAx",
        AxisKind::Date => "dateAx",
        AxisKind::Value => "valAx",
    };
    let numeric = kind == AxisKind::Value;
    // Bounds, log scale and units mean nothing on a category or date axis;
    // Excel ignores them there, so they are not written.
    let (log, max, min) = if numeric {
        (
            axis.log_base
                .map(|base| format!(r#"<c:logBase val="{base}"/>"#))
                .unwrap_or_default(),
            axis.max
                .map(|v| format!(r#"<c:max val="{v}"/>"#))
                .unwrap_or_default(),
            axis.min
                .map(|v| format!(r#"<c:min val="{v}"/>"#))
                .unwrap_or_default(),
        )
    } else {
        Default::default()
    };
    let orientation = if axis.reversed { "maxMin" } else { "minMax" };
    let gridlines = if axis.major_gridlines {
        "<c:majorGridlines/>"
    } else {
        ""
    };
    let minor_gridlines = if axis.minor_gridlines {
        "<c:minorGridlines/>"
    } else {
        ""
    };
    let title = axis
        .title
        .as_ref()
        .map(|text| {
            format!(
                "<c:title>{}</c:title>",
                rich_title(text, axis.title_style.as_ref())
            )
        })
        .unwrap_or_default();
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
    let ticks: String = [
        ("majorTickMark", axis.major_tick),
        ("minorTickMark", axis.minor_tick),
    ]
    .into_iter()
    .filter_map(|(tag, mark)| mark.map(|mark| format!(r#"<c:{tag} val="{}"/>"#, mark.code())))
    .collect();
    let tick_labels = if axis.tick_labels == TickLabels::NextTo {
        String::new()
    } else {
        format!(r#"<c:tickLblPos val="{}"/>"#, axis.tick_labels.code())
    };
    let tx_pr = if axis.label_style.is_some() || axis.label_rotation.is_some() {
        tx_pr_xml(axis.label_style.as_ref(), axis.label_rotation)
    } else {
        String::new()
    };
    let crosses = if axis.crosses_max {
        r#"<c:crosses val="max"/>"#
    } else {
        ""
    };
    let tail = match kind {
        AxisKind::Category => String::new(),
        AxisKind::Date => format!(
            r#"<c:auto val="0"/><c:lblOffset val="100"/><c:baseTimeUnit val="{}"/>"#,
            axis.date_unit.map_or("days", |unit| unit.code())
        ),
        AxisKind::Value => {
            let major = axis
                .major_unit
                .map(|v| format!(r#"<c:majorUnit val="{v}"/>"#))
                .unwrap_or_default();
            let minor = axis
                .minor_unit
                .map(|v| format!(r#"<c:minorUnit val="{v}"/>"#))
                .unwrap_or_default();
            format!(r#"<c:crossBetween val="{cross_between}"/>{major}{minor}"#)
        }
    };
    format!(
        r#"<c:{tag}><c:axId val="{id}"/><c:scaling>{log}<c:orientation val="{orientation}"/>{max}{min}</c:scaling><c:delete val="{deleted}"/><c:axPos val="{position}"/>{gridlines}{minor_gridlines}{title}{number_format}{ticks}{tick_labels}{tx_pr}<c:crossAx val="{cross_id}"/>{crosses}{tail}</c:{tag}>"#,
        deleted = i32::from(!axis.visible),
    )
}
