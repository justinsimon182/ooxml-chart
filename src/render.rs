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
use crate::reference;
use crate::spec::{
    AreaStyle, Axis, ChartKind, ChartPart, ChartSpec, DataLabelPosition, DataLabels, DataTable,
    DateUnit, DisplayUnit, ErrorAmount, ErrorAxis, ErrorBarSide, ErrorBars, ErrorValues,
    LabelField, LabelPart, Layout, MarkerSymbol, OfPie, OfPieSplit, Paint, Plot, PointFormat,
    PointLabel, Position, Series, SeriesName, TextStyle, TickLabels, Trendline, TrendlineKind,
    View3D,
};
use crate::xml::escape;

/// The primary axis pair. Arbitrary but stable: what matters is that each axis
/// names the other as its `crossAx`.
const CATEGORY_AXIS_ID: u32 = 111;
const VALUE_AXIS_ID: u32 = 222;
/// The secondary pair, used by plots on the right-hand value axis.
const SECONDARY_CATEGORY_AXIS_ID: u32 = 333;
const SECONDARY_VALUE_AXIS_ID: u32 = 444;
/// The depth (series) axis of a 3-D chart.
const SERIES_AXIS_ID: u32 = 555;

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
    /// High-low-close, or with `open` set, open-high-low-close.
    Stock {
        open: bool,
    },
    /// Pie-of-pie, or with `bar` set, bar-of-pie.
    OfPie {
        bar: bool,
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
        ChartKind::StockHighLowClose => Family::Stock { open: false },
        ChartKind::StockOpenHighLowClose => Family::Stock { open: true },
        ChartKind::PieOfPie => Family::OfPie { bar: false },
        ChartKind::BarOfPie => Family::OfPie { bar: true },
    }
}

/// The families drawn as slices: no axes, a point per slice.
fn is_round(family: Family) -> bool {
    matches!(
        family,
        Family::Pie | Family::Doughnut | Family::OfPie { .. }
    )
}

/// One drawn plot element: the chart's own series, or an added [`Plot`].
struct PlotRef<'a> {
    kind: ChartKind,
    family: Family,
    series: &'a [Series],
    secondary: bool,
    /// Drawn with a `<c:view3D>`. Only ever the chart's own plot.
    three_d: bool,
}

impl<'a> PlotRef<'a> {
    fn of(kind: ChartKind, series: &'a [Series], secondary: bool) -> Self {
        Self {
            kind,
            family: family(kind),
            series,
            secondary,
            three_d: false,
        }
    }

    /// Whether this 3-D plot has a depth axis, so a third `<c:axId>` names it
    /// rather than the unused `0` Excel writes.
    fn has_depth_axis(&self, view: &View3D) -> bool {
        match self.family {
            Family::Line => true,
            Family::Area { grouping } => grouping == "standard",
            Family::Bar { grouping, .. } => grouping == "clustered" && view.depth_axis,
            _ => false,
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

/// A `<c:layout>`: Excel's own choice when `layout` is `None` (an empty
/// element for the plot area, which always carries one; nothing for a legend),
/// otherwise a manual rectangle. `wMode` and `hMode` are left out, as Excel
/// does, so `w` and `h` are sizes rather than offsets.
fn layout_xml(layout: Option<&Layout>, plot_area: bool) -> String {
    let Some(layout) = layout else {
        return if plot_area {
            "<c:layout/>".to_string()
        } else {
            String::new()
        };
    };
    let target = if plot_area {
        format!(
            r#"<c:layoutTarget val="{}"/>"#,
            if layout.inner { "inner" } else { "outer" }
        )
    } else {
        String::new()
    };
    format!(
        r#"<c:layout><c:manualLayout>{target}<c:xMode val="edge"/><c:yMode val="edge"/><c:x val="{}"/><c:y val="{}"/><c:w val="{}"/><c:h val="{}"/></c:manualLayout></c:layout>"#,
        layout.x, layout.y, layout.width, layout.height,
    )
}

/// A `<c:layout>` for a title: x and y only, since a title sizes itself.
fn title_layout_xml(position: Option<Position>) -> String {
    match position {
        Some(Position { x, y }) => format!(
            r#"<c:layout><c:manualLayout><c:xMode val="edge"/><c:yMode val="edge"/><c:x val="{x}"/><c:y val="{y}"/></c:manualLayout></c:layout>"#
        ),
        None => String::new(),
    }
}

fn check_position(position: &Position) -> Result<(), ChartError> {
    let invalid = |reason| Err(ChartError::InvalidLayout { reason });
    if !position.x.is_finite() || !position.y.is_finite() {
        return invalid("every number must be finite");
    }
    if !(0.0..=1.0).contains(&position.x) || !(0.0..=1.0).contains(&position.y) {
        return invalid("x and y must be between 0 and 1");
    }
    Ok(())
}

fn data_table_xml(table: &DataTable) -> String {
    let flag = |tag: &str, on: bool| format!(r#"<c:{tag} val="{}"/>"#, i32::from(on));
    format!(
        "<c:dTable>{}{}{}{}{}</c:dTable>",
        flag("showHorzBorder", table.horizontal_borders),
        flag("showVertBorder", table.vertical_borders),
        flag("showOutline", table.outline),
        flag("showKeys", table.legend_keys),
        table
            .style
            .as_ref()
            .map(|style| tx_pr_xml(Some(style), None))
            .unwrap_or_default(),
    )
}

fn check_layout(layout: &Layout) -> Result<(), ChartError> {
    let invalid = |reason| Err(ChartError::InvalidLayout { reason });
    let numbers = [layout.x, layout.y, layout.width, layout.height];
    if numbers.iter().any(|n| !n.is_finite()) {
        return invalid("every number must be finite");
    }
    if !(0.0..=1.0).contains(&layout.x) || !(0.0..=1.0).contains(&layout.y) {
        return invalid("x and y must be between 0 and 1");
    }
    if layout.width <= 0.0 || layout.height <= 0.0 {
        return invalid("width and height must be above 0");
    }
    // A sliver of tolerance: 0.1 + 0.9 is not exactly 1.0 in floating point.
    if layout.x + layout.width > 1.0 + 1e-9 || layout.y + layout.height > 1.0 + 1e-9 {
        return invalid("the rectangle must fit inside the chart");
    }
    Ok(())
}

fn plots(spec: &ChartSpec) -> Vec<PlotRef<'_>> {
    let mut first = PlotRef::of(spec.kind, &spec.series, false);
    first.three_d = spec.view_3d.is_some();
    let mut all = vec![first];
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
        out.push_str(&title_xml(
            title,
            spec.title_style.as_ref(),
            spec.title_position,
        ));
    }
    if let Some(view) = &spec.view_3d {
        out.push_str(&view_3d_xml(view, plots[0].family));
    }
    out.push_str("<c:plotArea>");
    out.push_str(&layout_xml(spec.plot_area_layout.as_ref(), true));
    let mut first_index = 0;
    for plot in &plots {
        out.push_str(&plot_xml(spec, plot, first_index));
        first_index += plot.series.len();
    }
    if spec.kind.has_axes() {
        out.push_str(&axes_xml(spec, &plots));
    }
    if let Some(table) = &spec.data_table {
        out.push_str(&data_table_xml(table));
    }
    if let Some(style) = &spec.plot_area {
        out.push_str(&sp_pr_xml(style));
    }
    out.push_str("</c:plotArea>");
    if let Some(code) = spec.legend.code() {
        out.push_str(&format!(
            r#"<c:legend><c:legendPos val="{code}"/>{}{}<c:overlay val="{}"/>{}</c:legend>"#,
            legend_entries_xml(&spec.hidden_legend_entries),
            layout_xml(spec.legend_layout.as_ref(), false),
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
    if let Some(view) = &spec.view_3d {
        check_view_3d(spec, primary, view)?;
    }
    for layout in [&spec.plot_area_layout, &spec.legend_layout]
        .into_iter()
        .flatten()
    {
        check_layout(layout)?;
    }
    if !spec.hidden_legend_entries.is_empty() {
        if spec.legend.code().is_none() {
            return unsupported("a hidden legend entry with no legend".to_string());
        }
        // A pie's entries are its points, whose count lives in the sheet.
        if !is_round(primary.family) {
            let count: usize = plots.iter().map(|plot| plot.series.len()).sum();
            for &index in &spec.hidden_legend_entries {
                check_range(
                    "legend entry",
                    i64::try_from(index).unwrap_or(i64::MAX),
                    0,
                    count as i64 - 1,
                )?;
            }
        }
    }
    let cells = spec.data_labels.as_ref().is_some_and(|l| l.cells);
    for plot in plots {
        for series in plot.series {
            if let Some(paint) = &series.fill {
                let fillable = matches!(
                    plot.family,
                    Family::Bar { .. }
                        | Family::Area { .. }
                        | Family::Bubble
                        | Family::Radar { filled: true }
                );
                if !fillable {
                    return unsupported(format!("a series fill on {:?}", plot.kind));
                }
                check_paint(paint)?;
            }
            for (_, point) in &series.points {
                if let Some(paint) = &point.fill {
                    check_paint(paint)?;
                }
            }
            match (&series.label_range, cells) {
                (Some(reference), _) if reference.trim().is_empty() => {
                    return unsupported("an empty label range".to_string());
                }
                (Some(_), false) => {
                    return unsupported("a label range with no cell labels".to_string());
                }
                (None, true) => {
                    return unsupported("cell labels on a series with no label range".to_string());
                }
                (None, false) if series.label_range_cache.is_some() => {
                    return unsupported("a label range cache with no label range".to_string());
                }
                _ => {}
            }
        }
    }
    for plot in plots {
        for series in plot.series {
            check_series_references(series)?;
        }
    }
    let leader_lines = spec.data_labels.as_ref().is_some_and(|l| l.leader_lines);
    if leader_lines && !is_round(primary.family) {
        return unsupported(format!("leader lines on {:?}", primary.kind));
    }
    if let Some(table) = &spec.data_table {
        // Excel offers a data table for column, line and area charts only.
        let takes_table = matches!(
            primary.family,
            Family::Bar {
                horizontal: false,
                ..
            } | Family::Line
                | Family::Area { .. }
        );
        if !takes_table || plots.iter().any(|plot| plot.family == Family::Scatter) {
            return unsupported(format!("a data table on {:?}", primary.kind));
        }
        if let Some(style) = &table.style {
            check_text_style(style)?;
        }
    }
    if let Some(position) = &spec.title_position {
        if spec.title.is_none() {
            return unsupported("a title position with no title".to_string());
        }
        check_position(position)?;
    }
    if spec.legend_layout.is_some() && spec.legend.code().is_none() {
        return unsupported("a legend layout with no legend".to_string());
    }

    // Combinations.
    if !spec.extra_plots.is_empty() {
        if plots.iter().any(|plot| plot.family == Family::Bubble) {
            return unsupported("combining a bubble chart with other kinds".to_string());
        }
        for (index, plot) in plots.iter().enumerate() {
            let shares_categories = matches!(
                plot.family,
                Family::Bar {
                    horizontal: false,
                    ..
                } | Family::Line
                    | Family::Area { .. }
                    | Family::Stock { .. }
            );
            // A scatter joins column, line and area as an added plot; it
            // cannot be the chart the others are added to.
            let joins_as_scatter = index > 0 && plot.family == Family::Scatter;
            if !(shares_categories || joins_as_scatter) {
                return unsupported(format!(
                    "combining {:?}: column, line, area and stock share a category axis, and a scatter can be added to them",
                    plot.kind
                ));
            }
        }
        // A secondary scatter has two value axes; anything else there has a
        // category axis, and the pair cannot be both.
        let secondary = || plots.iter().skip(1).filter(|plot| plot.secondary);
        if secondary().any(|plot| plot.family == Family::Scatter)
            && secondary().any(|plot| plot.family != Family::Scatter)
        {
            return unsupported(
                "a scatter and another kind on the same secondary axis".to_string(),
            );
        }
    }
    if plots.iter().skip(1).any(|plot| plot.secondary) {
        check_axis(&spec.secondary_value_axis)?;
    }

    // Pie-of-pie and bar-of-pie plot one series and take split settings.
    for plot in plots {
        if matches!(plot.family, Family::OfPie { .. }) && plot.series.len() != 1 {
            return unsupported(format!(
                "{:?} with {} series: it takes exactly 1",
                plot.kind,
                plot.series.len()
            ));
        }
    }
    if let Some(settings) = &spec.of_pie {
        if !matches!(primary.family, Family::OfPie { .. }) {
            return unsupported(format!("of-pie settings on {:?}", primary.kind));
        }
        check_range("second plot size", i64::from(settings.second_size), 5, 200)?;
        match &settings.split {
            OfPieSplit::Auto => {}
            OfPieSplit::LastPoints(0) => {
                return unsupported("a split of zero points".to_string());
            }
            OfPieSplit::LastPoints(_) => {}
            OfPieSplit::ValueBelow(value) if !value.is_finite() => {
                return unsupported("a non-finite split value".to_string());
            }
            OfPieSplit::ValueBelow(_) => {}
            OfPieSplit::PercentBelow(percent) if !(*percent > 0.0 && *percent < 100.0) => {
                return unsupported("a split percentage outside 0-100".to_string());
            }
            OfPieSplit::PercentBelow(_) => {}
            OfPieSplit::Points(points) => {
                let mut sorted = points.clone();
                sorted.sort_unstable();
                sorted.dedup();
                if sorted.is_empty() || sorted.len() != points.len() {
                    return unsupported(
                        "a custom split needs at least one point, each once".to_string(),
                    );
                }
            }
        }
    }

    // Stock charts take a fixed set of series and their own bar colours.
    for plot in plots {
        if let Family::Stock { open } = plot.family {
            let wanted = if open { 4 } else { 3 };
            if plot.series.len() != wanted {
                return unsupported(format!(
                    "{:?} with {} series: it takes exactly {wanted}",
                    plot.kind,
                    plot.series.len()
                ));
            }
        }
    }
    if let Some((up, down)) = &spec.stock_bars {
        if !plots
            .iter()
            .any(|plot| plot.family == Family::Stock { open: true })
        {
            return unsupported(
                "stock bar colours on a chart with no open-high-low-close plot".to_string(),
            );
        }
        check_color(up)?;
        check_color(down)?;
    }

    // Per-series settings.
    for plot in plots {
        for series in plot.series {
            check_series(plot, series)?;
        }
        if let Some(position) = spec.data_labels.as_ref().and_then(|l| l.position) {
            if !label_positions(plot).contains(&position) {
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
            Family::Bar { .. } | Family::Line | Family::Area { .. } | Family::Stock { .. }
        );
        if spec.category_axis.date_unit.is_some() && !dates_ok {
            return unsupported(format!("a date axis on {:?}", primary.kind));
        }
        check_axis(&spec.category_axis)?;
        check_axis(&spec.value_axis)?;
        let has_x_values = matches!(primary.family, Family::Scatter | Family::Bubble);
        if spec.category_axis.display_unit.is_some() && !has_x_values {
            return unsupported(format!(
                "display units on the category axis of {:?}",
                primary.kind
            ));
        }
        // A logarithmic axis has no zero or negative positions to cross at.
        for (crossing, other) in [
            (&spec.category_axis, &spec.value_axis),
            (&spec.value_axis, &spec.category_axis),
        ] {
            if other.log_base.is_some() && crossing.crosses_at.is_some_and(|at| at <= 0.0) {
                return Err(ChartError::InvalidAxisRange {
                    reason: "a crossing value must be positive on a logarithmic axis",
                });
            }
        }
    }

    // Bar geometry and round-chart settings.
    if plots.iter().any(|plot| {
        matches!(
            plot.family,
            Family::Bar { .. } | Family::Stock { open: true } | Family::OfPie { .. }
        )
    }) {
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
        if !label.parts.is_empty() {
            if label.hidden || label.text.is_some() {
                return unsupported(
                    "a data label with parts that is also hidden or has text".to_string(),
                );
            }
            for part in &label.parts {
                match part {
                    LabelPart::Field(LabelField::Percentage) if !is_round(plot.family) => {
                        return unsupported(format!("a percentage field on {:?}", plot.kind));
                    }
                    LabelPart::Field(LabelField::CellRange) if series.label_range.is_none() => {
                        return unsupported("a cell-range field with no label range".to_string());
                    }
                    _ => {}
                }
            }
        }
        if let Some(style) = &label.style {
            check_text_style(style)?;
        }
        if let Some(position) = label.position {
            if !label_positions(plot).contains(&position) {
                return Err(ChartError::InvalidDataLabelPosition {
                    position: position.name(),
                });
            }
        }
    }
    check_error_bars(plot, series)?;
    if !series.trendlines.is_empty() {
        let allowed = !plot.three_d
            && match plot.family {
                Family::Bar { grouping, .. } => grouping == "clustered",
                Family::Area { grouping } => grouping == "standard",
                Family::Line | Family::Scatter | Family::Bubble => true,
                Family::Pie
                | Family::Doughnut
                | Family::OfPie { .. }
                | Family::Radar { .. }
                | Family::Stock { .. } => false,
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
    if plot.three_d {
        return unsupported(format!("error bars on a 3-D {:?}", plot.kind));
    }
    let has_x = matches!(plot.family, Family::Scatter | Family::Bubble);
    if is_round(plot.family) || matches!(plot.family, Family::Radar { .. }) {
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

/// Colours, stop counts and angles of a fill or line.
/// The syntax of every reference a series holds.
fn check_series_references(series: &Series) -> Result<(), ChartError> {
    match (&series.name, &series.name_cache) {
        (SeriesName::Reference(reference), _) => reference::check(reference)?,
        (SeriesName::Literal(_), Some(_)) => {
            return Err(ChartError::Unsupported {
                what: "a name cache on a literal series name".to_string(),
            });
        }
        (SeriesName::Literal(_), None) => {}
    }
    let optional = [
        &series.categories,
        &series.bubble_sizes,
        &series.label_range,
    ];
    for reference in optional.into_iter().flatten() {
        reference::check(reference)?;
    }
    reference::check(&series.values)?;
    for bars in &series.error_bars {
        if let ErrorAmount::Custom { plus, minus } = &bars.amount {
            for values in [plus, minus].into_iter().flatten() {
                if let ErrorValues::Reference(reference) = values {
                    reference::check(reference)?;
                }
            }
        }
    }
    Ok(())
}

fn check_paint(paint: &Paint) -> Result<(), ChartError> {
    match paint {
        Paint::None => {}
        Paint::Color(color) => check_color(color)?,
        Paint::Gradient(gradient) => {
            let invalid = |reason| Err(ChartError::InvalidFill { reason });
            if !(2..=10).contains(&gradient.stops.len()) {
                return invalid("a gradient needs two to ten stops");
            }
            if gradient.angle > 359 {
                return invalid("a gradient angle is from 0 to 359 degrees");
            }
            if gradient.stops.iter().any(|stop| stop.position > 100) {
                return invalid("a gradient stop position is from 0 to 100 percent");
            }
            if gradient
                .stops
                .windows(2)
                .any(|pair| pair[0].position > pair[1].position)
            {
                return invalid("gradient stop positions must not decrease");
            }
            for stop in &gradient.stops {
                check_color(&stop.color)?;
            }
        }
        Paint::Pattern(pattern) => {
            check_color(&pattern.foreground)?;
            check_color(&pattern.background)?;
        }
    }
    Ok(())
}

fn check_area_style(style: &AreaStyle) -> Result<(), ChartError> {
    for paint in [&style.fill, &style.border].into_iter().flatten() {
        check_paint(paint)?;
    }
    if let Some(points) = style.border_width_pt {
        check_line_width(points)?;
    }
    Ok(())
}

/// Date-axis tick spacing: a count of time units, never finer than the axis's
/// own unit, which is how Excel's own dialog limits it.
/// `<c:majorUnit>` and `<c:majorTimeUnit>` (or the minor pair), in that order.
fn date_tick_xml(which: &str, count: Option<f64>, unit: Option<DateUnit>) -> String {
    let count = count
        .map(|count| format!(r#"<c:{which}Unit val="{count}"/>"#))
        .unwrap_or_default();
    let unit = unit
        .map(|unit| format!(r#"<c:{which}TimeUnit val="{}"/>"#, unit.code()))
        .unwrap_or_default();
    format!("{count}{unit}")
}

fn check_date_ticks(axis: &Axis) -> Result<(), ChartError> {
    let reason = |reason| Err(ChartError::InvalidAxisRange { reason });
    let Some(base) = axis.date_unit else {
        if axis.major_time_unit.is_some() || axis.minor_time_unit.is_some() {
            return reason("a tick time unit is set on an axis that is not a date axis");
        }
        return Ok(());
    };
    for (count, unit) in [
        (axis.major_unit, axis.major_time_unit),
        (axis.minor_unit, axis.minor_time_unit),
    ] {
        if unit.is_some() && count.is_none() {
            return reason("a tick time unit has no tick unit to count");
        }
        if count.is_some_and(|count| count.fract() != 0.0) {
            return reason("a date axis tick unit is not a whole number");
        }
        if unit.is_some_and(|unit| unit < base) {
            return reason("a date axis tick unit is finer than the axis unit");
        }
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
    check_date_ticks(axis)?;
    if let Some(DisplayUnit::Custom(divisor)) = axis.display_unit {
        if !divisor.is_finite() || divisor <= 0.0 {
            return reason("a custom display unit is not a positive number");
        }
    }
    if axis.display_unit_label && axis.display_unit.is_none() {
        return reason("a display unit label needs a display unit");
    }
    if let Some(at) = axis.crosses_at {
        if !at.is_finite() {
            return reason("a crossing value is not finite");
        }
        if axis.crosses_max {
            return reason("an axis cannot cross at both a value and the maximum");
        }
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
    if let Some(position) = &axis.title_position {
        if axis.title.is_none() {
            return Err(ChartError::Unsupported {
                what: "an axis title position with no axis title".to_string(),
            });
        }
        check_position(position)?;
    }
    if let Some(paint) = &axis.line {
        check_paint(paint)?;
    }
    if let Some(points) = axis.line_width_pt {
        check_line_width(points)?;
    }
    for style in [&axis.title_style, &axis.label_style].into_iter().flatten() {
        check_text_style(style)?;
    }
    Ok(())
}

/// The label positions Excel accepts for a kind. Anything else — even a
/// position that is fine on a sibling kind — makes it report the file damaged.
fn label_positions(plot: &PlotRef<'_>) -> &'static [DataLabelPosition] {
    use DataLabelPosition::*;
    // A 3-D pie keeps its positions; every other 3-D chart has none.
    if plot.three_d && plot.family != Family::Pie {
        return &[];
    }
    let family = plot.family;
    match family {
        Family::Bar {
            grouping: "clustered",
            ..
        } => &[Center, InsideEnd, InsideBase, OutsideEnd],
        Family::Bar { .. } => &[Center, InsideEnd, InsideBase],
        Family::Line | Family::Scatter | Family::Bubble | Family::Stock { .. } => {
            &[Center, Above, Below, Left, Right]
        }
        Family::Pie | Family::OfPie { .. } => &[Center, InsideEnd, OutsideEnd, BestFit],
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
    if let (true, Some(view)) = (plot.three_d, &spec.view_3d) {
        // A 3-D chart names a third axis: the depth axis, or the unused `0`
        // Excel writes when there is none.
        let third = if plot.has_depth_axis(view) {
            SERIES_AXIS_ID
        } else {
            0
        };
        let axes = format!(r#"{axes}<c:axId val="{third}"/>"#);
        return plot_3d_xml(spec, plot, view, &series, &labels, &axes);
    }

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
        Family::OfPie { bar } => {
            let kind = if bar { "bar" } else { "pie" };
            format!(
                r#"<c:ofPieChart><c:ofPieType val="{kind}"/><c:varyColors val="1"/>{series}{labels}<c:gapWidth val="{gap}"/>{}</c:ofPieChart>"#,
                of_pie_xml(spec.of_pie.as_ref()),
                gap = spec.gap_width,
            )
        }
        Family::Stock { open } => {
            let bars = if open {
                stock_bars_xml(spec)
            } else {
                String::new()
            };
            format!(r#"<c:stockChart>{series}{labels}<c:hiLowLines/>{bars}{axes}</c:stockChart>"#)
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

/// The split, second-plot size and joining lines that end an `<c:ofPieChart>`.
fn of_pie_xml(settings: Option<&OfPie>) -> String {
    let default = OfPie::default();
    let settings = settings.unwrap_or(&default);
    let split = match &settings.split {
        OfPieSplit::Auto => r#"<c:splitType val="auto"/>"#.to_string(),
        OfPieSplit::LastPoints(count) => {
            format!(r#"<c:splitType val="pos"/><c:splitPos val="{count}"/>"#)
        }
        OfPieSplit::ValueBelow(value) => {
            format!(r#"<c:splitType val="val"/><c:splitPos val="{value}"/>"#)
        }
        OfPieSplit::PercentBelow(percent) => {
            format!(r#"<c:splitType val="percent"/><c:splitPos val="{percent}"/>"#)
        }
        OfPieSplit::Points(points) => {
            let mut sorted = points.clone();
            sorted.sort_unstable();
            let items: String = sorted
                .iter()
                .map(|point| format!(r#"<c:secondPiePt val="{point}"/>"#))
                .collect();
            format!(r#"<c:splitType val="cust"/><c:custSplit>{items}</c:custSplit>"#)
        }
    };
    let lines = if settings.series_lines {
        "<c:serLines/>"
    } else {
        ""
    };
    format!(
        r#"{split}<c:secondPieSize val="{}"/>{lines}"#,
        settings.second_size
    )
}

/// `<c:upDownBars>`: the bars between each category's open and close.
fn stock_bars_xml(spec: &ChartSpec) -> String {
    let bar = |tag: &str, color: Option<&String>| match color {
        Some(rgb) => format!("<c:{tag}><c:spPr>{}</c:spPr></c:{tag}>", solid(rgb)),
        None => format!("<c:{tag}/>"),
    };
    let (up, down) = match &spec.stock_bars {
        Some((up, down)) => (Some(up), Some(down)),
        None => (None, None),
    };
    format!(
        r#"<c:upDownBars><c:gapWidth val="{}"/>{}{}</c:upDownBars>"#,
        spec.gap_width,
        bar("upBars", up),
        bar("downBars", down)
    )
}

/// The 3-D counterpart of a plot element.
fn plot_3d_xml(
    spec: &ChartSpec,
    plot: &PlotRef<'_>,
    view: &View3D,
    series: &str,
    labels: &str,
    axes: &str,
) -> String {
    let gap_depth = view.gap_depth.unwrap_or(150);
    match plot.family {
        Family::Bar {
            horizontal,
            grouping,
        } => {
            let direction = if horizontal { "bar" } else { "col" };
            let grouping = if view.depth_axis {
                "standard"
            } else {
                grouping
            };
            let shape = view
                .bar_shape
                .map(|shape| format!(r#"<c:shape val="{}"/>"#, shape.code()))
                .unwrap_or_default();
            format!(
                r#"<c:bar3DChart><c:barDir val="{direction}"/><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}{labels}<c:gapWidth val="{gap}"/><c:gapDepth val="{gap_depth}"/>{shape}{axes}</c:bar3DChart>"#,
                gap = spec.gap_width,
            )
        }
        Family::Line => format!(
            r#"<c:line3DChart><c:grouping val="standard"/><c:varyColors val="0"/>{series}{labels}<c:gapDepth val="{gap_depth}"/>{axes}</c:line3DChart>"#
        ),
        Family::Area { grouping } => format!(
            r#"<c:area3DChart><c:grouping val="{grouping}"/><c:varyColors val="0"/>{series}{labels}<c:gapDepth val="{gap_depth}"/>{axes}</c:area3DChart>"#
        ),
        _ => format!(r#"<c:pie3DChart><c:varyColors val="1"/>{series}{labels}</c:pie3DChart>"#),
    }
}

/// `<c:view3D>` and the floor and walls that follow it.
fn view_3d_xml(view: &View3D, family: Family) -> String {
    let pie = family == Family::Pie;
    let right_angle = view.right_angle_axes.unwrap_or(!pie);
    let rot_x = view.rotation_x.unwrap_or(if pie { 30 } else { 15 });
    let rot_y = view.rotation_y.unwrap_or(if pie { 0 } else { 20 });
    let height = view
        .height_percent
        .map(|percent| format!(r#"<c:hPercent val="{percent}"/>"#))
        .unwrap_or_default();
    let depth = view
        .depth_percent
        .map(|percent| format!(r#"<c:depthPercent val="{percent}"/>"#))
        .unwrap_or_default();
    let perspective = if right_angle {
        String::new()
    } else {
        format!(
            r#"<c:perspective val="{}"/>"#,
            view.perspective.unwrap_or(30)
        )
    };
    let surface = |tag: &str, style: &Option<AreaStyle>| {
        style
            .as_ref()
            .map(|style| {
                format!(
                    r#"<c:{tag}><c:thickness val="0"/>{}</c:{tag}>"#,
                    sp_pr_xml(style)
                )
            })
            .unwrap_or_default()
    };
    format!(
        r#"<c:view3D><c:rotX val="{rot_x}"/>{height}<c:rotY val="{rot_y}"/>{depth}<c:rAngAx val="{}"/>{perspective}</c:view3D>{}{}{}"#,
        i32::from(right_angle),
        surface("floor", &view.floor),
        surface("sideWall", &view.side_wall),
        surface("backWall", &view.back_wall),
    )
}

/// The depth axis of a 3-D chart, as Excel writes it.
fn series_axis_xml() -> String {
    format!(
        r#"<c:serAx><c:axId val="{SERIES_AXIS_ID}"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/><c:crossAx val="{VALUE_AXIS_ID}"/><c:crosses val="autoZero"/></c:serAx>"#
    )
}

fn check_view_3d(spec: &ChartSpec, primary: &PlotRef<'_>, view: &View3D) -> Result<(), ChartError> {
    let pie = primary.family == Family::Pie;
    let bar = matches!(primary.family, Family::Bar { .. });
    if !(bar || pie || matches!(primary.family, Family::Line | Family::Area { .. })) {
        return unsupported(format!("a 3-D view on {:?}", primary.kind));
    }
    if !spec.extra_plots.is_empty() {
        return unsupported("a 3-D view on a combination chart".to_string());
    }
    let ranges: [(&str, Option<i64>, i64, i64); 6] = [
        ("3-D rotation x", view.rotation_x.map(i64::from), -90, 90),
        ("3-D rotation y", view.rotation_y.map(i64::from), 0, 360),
        ("3-D perspective", view.perspective.map(i64::from), 0, 240),
        ("3-D height", view.height_percent.map(i64::from), 5, 500),
        ("3-D depth", view.depth_percent.map(i64::from), 20, 2000),
        ("3-D gap depth", view.gap_depth.map(i64::from), 0, 500),
    ];
    for (what, value, min, max) in ranges {
        if let Some(value) = value {
            check_range(what, value, min, max)?;
        }
    }
    if view.right_angle_axes == Some(true) && view.perspective.is_some() {
        return unsupported("a 3-D perspective with right-angle axes".to_string());
    }
    if pie
        && (view.right_angle_axes == Some(true)
            || view.gap_depth.is_some()
            || view.bar_shape.is_some()
            || view.depth_axis
            || view.floor.is_some()
            || view.side_wall.is_some()
            || view.back_wall.is_some())
    {
        return unsupported(
            "right-angle axes, depth, shapes, floor or walls on a 3-D pie".to_string(),
        );
    }
    if view.bar_shape.is_some() && !bar {
        return unsupported(format!("a bar shape on {:?}", primary.kind));
    }
    if view.depth_axis
        && !matches!(
            primary.family,
            Family::Bar {
                grouping: "clustered",
                ..
            }
        )
    {
        return unsupported(format!("a depth axis on {:?}", primary.kind));
    }
    for style in [&view.floor, &view.side_wall, &view.back_wall]
        .into_iter()
        .flatten()
    {
        check_area_style(style)?;
    }
    Ok(())
}

fn data_labels_xml(labels: &DataLabels) -> String {
    format!(
        "<c:dLbls>{}{}{}</c:dLbls>",
        label_group_xml(labels),
        leader_lines_xml(labels),
        cells_ext_xml(labels)
    )
}

const C15: &str = "http://schemas.microsoft.com/office/drawing/2012/chart";

/// The Office 2013 extension that turns on "Value From Cells". It ends a
/// `<c:dLbls>` or a `<c:dLbl>`.
fn cells_ext_xml(labels: &DataLabels) -> String {
    if labels.cells {
        format!(
            r#"<c:extLst><c:ext uri="{{CE6537A1-D6FC-4f65-9D91-7224C49458BB}}" xmlns:c15="{C15}"><c15:showDataLabelsRange val="1"/></c:ext></c:extLst>"#
        )
    } else {
        String::new()
    }
}

/// The extension a label with fields needs: an (empty) field table, then the
/// "Value From Cells" switch.
fn fields_ext_xml(cells: bool) -> String {
    format!(
        r#"<c:extLst><c:ext uri="{{CE6537A1-D6FC-4f65-9D91-7224C49458BB}}" xmlns:c15="{C15}"><c15:dlblFieldTable/><c15:showDataLabelsRange val="{}"/></c:ext></c:extLst>"#,
        i32::from(cells)
    )
}

/// A series' label range, the last thing in a `<c:ser>`.
fn label_range_xml(series: &Series) -> String {
    let Some(reference) = &series.label_range else {
        return String::new();
    };
    let cache = series
        .label_range_cache
        .as_deref()
        .map(|labels| str_cache(labels).replace("c:strCache", "c15:dlblRangeCache"))
        .unwrap_or_default();
    format!(
        r#"<c:extLst><c:ext uri="{{02D57815-91ED-43cb-92C2-25804820EDAC}}" xmlns:c15="{C15}"><c15:datalabelsRange><c15:f>{}</c15:f>{cache}</c15:datalabelsRange></c:ext></c:extLst>"#,
        escape(reference)
    )
}

/// `<c:showLeaderLines>`, which follows the flags in a `<c:dLbls>` and has no
/// place in a single `<c:dLbl>`.
fn leader_lines_xml(labels: &DataLabels) -> &'static str {
    if labels.leader_lines {
        r#"<c:showLeaderLines val="1"/>"#
    } else {
        ""
    }
}

/// One `<c:legendEntry>` per hidden entry, in index order.
fn legend_entries_xml(hidden: &[usize]) -> String {
    let mut indices = hidden.to_vec();
    indices.sort_unstable();
    indices.dedup();
    indices
        .into_iter()
        .map(|index| {
            format!(r#"<c:legendEntry><c:idx val="{index}"/><c:delete val="1"/></c:legendEntry>"#)
        })
        .collect()
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
fn series_labels_xml(
    series_index: usize,
    series: &Series,
    chart_wide: Option<&DataLabels>,
) -> String {
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
        .map(|(index, label)| point_label_xml(series_index, *index, label, &inherited))
        .collect();
    let group = chart_wide
        .map(|labels| {
            format!(
                "{}{}{}",
                label_group_xml(labels),
                leader_lines_xml(labels),
                cells_ext_xml(labels)
            )
        })
        .unwrap_or_else(|| label_flags_xml(&DataLabels::default()));
    format!("<c:dLbls>{entries}{group}</c:dLbls>")
}

/// A GUID for a field, unique within the chart and stable between renders.
fn field_id(series: usize, point: usize, part: usize) -> String {
    format!("{{{series:08X}-{point:04X}-4000-8000-{part:012X}}}")
}

fn point_label_xml(
    series_index: usize,
    index: usize,
    label: &PointLabel,
    inherited: &DataLabels,
) -> String {
    if label.hidden {
        return format!(r#"<c:dLbl><c:idx val="{index}"/><c:delete val="1"/></c:dLbl>"#);
    }
    let style = label.style.as_ref().or(inherited.style.as_ref());
    let custom = label.text.is_some() || !label.parts.is_empty();
    let has_fields = label
        .parts
        .iter()
        .any(|part| matches!(part, LabelPart::Field(_)));
    let mixed = if label.parts.is_empty() {
        String::new()
    } else {
        let props = style
            .map(|style| run_props("rPr", r#" lang="en-US""#, style))
            .unwrap_or_else(|| r#"<a:rPr lang="en-US"/>"#.to_string());
        let runs: String = label
            .parts
            .iter()
            .enumerate()
            .map(|(part, item)| match item {
                LabelPart::Text(text) => format!("<a:r>{props}<a:t>{}</a:t></a:r>", escape(text)),
                LabelPart::Field(field) => {
                    let (code, placeholder) = field.code();
                    format!(
                        r#"<a:fld id="{}" type="{code}">{props}<a:pPr/><a:t>{placeholder}</a:t></a:fld>"#,
                        field_id(series_index, index, part)
                    )
                }
            })
            .collect();
        format!("<c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p>{runs}</a:p></c:rich></c:tx>")
    };
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
        .unwrap_or(mixed);
    // Custom text carries its font on the run; otherwise it is the label's
    // default text properties.
    let body = DataLabels {
        position: label.position.or(inherited.position),
        style: style.filter(|_| !custom).cloned(),
        ..inherited.clone()
    };
    let flags_and_format = {
        let group = label_group_xml(&body);
        // Custom text is shown whatever the flags say, but Excel writes at
        // least the value flag alongside it.
        if custom && !(body.value || body.category || body.series || body.percent) {
            label_group_xml(&DataLabels {
                value: true,
                ..body
            })
        } else {
            group
        }
    };
    format!(
        r#"<c:dLbl><c:idx val="{index}"/>{text}{flags_and_format}{}</c:dLbl>"#,
        if has_fields {
            fields_ext_xml(inherited.cells)
        } else {
            cells_ext_xml(inherited)
        }
    )
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
        SeriesName::Reference(reference) => {
            let cache = series
                .name_cache
                .as_ref()
                .map(|text| str_cache(std::slice::from_ref(text)))
                .unwrap_or_default();
            format!(
                "<c:tx><c:strRef><c:f>{}</c:f>{cache}</c:strRef></c:tx>",
                escape(reference)
            )
        }
    };
    let shape = shape_xml(plot, series);
    let points = format!(
        "{}{}",
        points_xml(plot, series),
        series_labels_xml(index, series, chart_wide_labels)
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
            "{head}{shape}{points}{trendlines}{error_bars}{x}<c:yVal>{values}</c:yVal>{tail}{}</c:ser>",
            label_range_xml(series)
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
    format!("{head}{shape}{points}{trendlines}{error_bars}{categories}<c:val>{values}</c:val>{smooth}{}</c:ser>", label_range_xml(series))
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
        | Family::Radar { filled: true } => series
            .fill
            .as_ref()
            .map(fill_xml)
            .or_else(|| color.map(solid))
            .map(|fill| format!("<c:spPr>{fill}</c:spPr>"))
            .unwrap_or_default(),
        Family::Pie | Family::Doughnut | Family::OfPie { .. } => String::new(),
        Family::Stock { open } => {
            // The high-low line (and bars) draw the chart; the series' own
            // lines are switched off. A high-low-close chart marks the close
            // with a tick, and every other series carries no marker.
            let is_close = plot
                .series
                .last()
                .is_some_and(|last| std::ptr::eq(last, series));
            let marker = if series.marker.is_none() && !open && is_close {
                let mut ticked = series.clone();
                ticked.marker = Some((MarkerSymbol::Dash, 7));
                marker_xml(plot.kind, &ticked, color)
            } else if series.marker.is_none() {
                format!(
                    r#"<c:marker><c:symbol val="{}"/></c:marker>"#,
                    MarkerSymbol::None.code()
                )
            } else {
                marker_xml(plot.kind, series, color)
            };
            format!(
                r#"<c:spPr><a:ln w="19050" cap="rnd"><a:noFill/><a:round/></a:ln></c:spPr>{marker}"#
            )
        }
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
    let round = is_round(plot.family);
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
                .fill
                .as_ref()
                .map(fill_xml)
                .or_else(|| point.color.as_deref().map(solid))
                .map(|fill| format!("<c:spPr>{fill}</c:spPr>"))
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

/// The fill element for a paint: `<a:noFill/>`, `<a:solidFill>`,
/// `<a:gradFill>` or `<a:pattFill>`.
fn fill_xml(paint: &Paint) -> String {
    let color = |rgb: &str| format!(r#"<a:srgbClr val="{}"/>"#, rgb.to_ascii_uppercase());
    match paint {
        Paint::None => "<a:noFill/>".to_string(),
        Paint::Color(rgb) => solid(rgb),
        Paint::Gradient(gradient) => {
            let stops: String = gradient
                .stops
                .iter()
                .map(|stop| {
                    format!(
                        r#"<a:gs pos="{}">{}</a:gs>"#,
                        u32::from(stop.position) * 1000,
                        color(&stop.color)
                    )
                })
                .collect();
            format!(
                r#"<a:gradFill rotWithShape="1"><a:gsLst>{stops}</a:gsLst><a:lin ang="{}" scaled="0"/></a:gradFill>"#,
                u32::from(gradient.angle) * 60_000
            )
        }
        Paint::Pattern(pattern) => format!(
            r#"<a:pattFill prst="{}"><a:fgClr>{}</a:fgClr><a:bgClr>{}</a:bgClr></a:pattFill>"#,
            pattern.kind.code(),
            color(&pattern.foreground),
            color(&pattern.background)
        ),
    }
}

fn sp_pr_xml(style: &AreaStyle) -> String {
    let fill = style.fill.as_ref().map(fill_xml).unwrap_or_default();
    let width = line_width_attr(style.border_width_pt);
    let line = match &style.border {
        Some(paint) => format!("<a:ln{width}>{}</a:ln>", fill_xml(paint)),
        None if !width.is_empty() => format!("<a:ln{width}/>"),
        None => String::new(),
    };
    format!("<c:spPr>{fill}{line}</c:spPr>")
}

fn rich_title(text: &str, style: Option<&TextStyle>, position: Option<Position>) -> String {
    let run_props = style
        .map(|style| run_props("rPr", r#" lang="en-US""#, style))
        .unwrap_or_default();
    format!(
        "<c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r>{run_props}<a:t>{}</a:t></a:r></a:p></c:rich></c:tx>{}<c:overlay val=\"0\"/>",
        escape(text),
        title_layout_xml(position)
    )
}

fn title_xml(text: &str, style: Option<&TextStyle>, position: Option<Position>) -> String {
    format!(
        r#"<c:title>{}</c:title><c:autoTitleDeleted val="0"/>"#,
        rich_title(text, style, position)
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
        // A secondary scatter takes two value axes, x along the bottom.
        let (secondary_first, secondary_between) = if plots
            .iter()
            .any(|plot| plot.secondary && plot.family == Family::Scatter)
        {
            (AxisKind::Value, "midCat")
        } else {
            (first, cross_between)
        };
        // The right-hand value axis crosses the (hidden) secondary category
        // axis at its far end, which is what puts it on the right.
        let mut value_axis = spec.secondary_value_axis.clone();
        value_axis.crosses_max = value_axis.crosses_at.is_none();
        out.push_str(&axis_xml(
            &AxisPlacement {
                kind: AxisKind::Value,
                id: SECONDARY_VALUE_AXIS_ID,
                cross_id: SECONDARY_CATEGORY_AXIS_ID,
                position: "r",
                cross_between: secondary_between,
            },
            &value_axis,
        ));
        out.push_str(&axis_xml(
            &AxisPlacement {
                kind: secondary_first,
                id: SECONDARY_CATEGORY_AXIS_ID,
                cross_id: SECONDARY_VALUE_AXIS_ID,
                position: category_pos,
                cross_between: secondary_between,
            },
            &Axis {
                date_unit: spec.category_axis.date_unit,
                ..Axis::default().hidden()
            },
        ));
    }
    if let (true, Some(view)) = (plots[0].three_d, &spec.view_3d) {
        if plots[0].has_depth_axis(view) {
            out.push_str(&series_axis_xml());
        }
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
                rich_title(text, axis.title_style.as_ref(), axis.title_position)
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
    let line = if axis.line.is_some() || axis.line_width_pt.is_some() {
        sp_pr_xml(&AreaStyle {
            border: axis.line.clone(),
            border_width_pt: axis.line_width_pt,
            ..AreaStyle::default()
        })
    } else {
        String::new()
    };
    let tx_pr = if axis.label_style.is_some() || axis.label_rotation.is_some() {
        tx_pr_xml(axis.label_style.as_ref(), axis.label_rotation)
    } else {
        String::new()
    };
    let crosses = match axis.crosses_at {
        Some(value) => format!(r#"<c:crossesAt val="{value}"/>"#),
        None if axis.crosses_max => r#"<c:crosses val="max"/>"#.to_string(),
        None => String::new(),
    };
    let tail = match kind {
        AxisKind::Category => String::new(),
        AxisKind::Date => format!(
            r#"<c:auto val="0"/><c:lblOffset val="100"/><c:baseTimeUnit val="{}"/>{}{}"#,
            axis.date_unit.map_or("days", |unit| unit.code()),
            date_tick_xml("major", axis.major_unit, axis.major_time_unit),
            date_tick_xml("minor", axis.minor_unit, axis.minor_time_unit),
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
            let display = axis
                .display_unit
                .map(|unit| {
                    let label = if axis.display_unit_label {
                        "<c:dispUnitsLbl/>"
                    } else {
                        ""
                    };
                    format!("<c:dispUnits>{}{label}</c:dispUnits>", unit.element())
                })
                .unwrap_or_default();
            format!(r#"<c:crossBetween val="{cross_between}"/>{major}{minor}{display}"#)
        }
    };
    format!(
        r#"<c:{tag}><c:axId val="{id}"/><c:scaling>{log}<c:orientation val="{orientation}"/>{max}{min}</c:scaling><c:delete val="{deleted}"/><c:axPos val="{position}"/>{gridlines}{minor_gridlines}{title}{number_format}{ticks}{tick_labels}{line}{tx_pr}<c:crossAx val="{cross_id}"/>{crosses}{tail}</c:{tag}>"#,
        deleted = i32::from(!axis.visible),
    )
}
