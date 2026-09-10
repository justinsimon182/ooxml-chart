//! Describing a chart, rather than drawing one.

use crate::error::ChartError;
use crate::render;

/// What kind of chart to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChartKind {
    /// Horizontal bars, one per category per series, side by side.
    BarClustered,
    /// Horizontal bars stacked into one bar per category.
    BarStacked,
    /// Vertical columns, one per category per series, side by side.
    ColumnClustered,
    /// Vertical columns stacked into one column per category.
    ColumnStacked,
    /// A line per series, no point markers.
    Line,
    /// A line per series with a marker at each point.
    LineMarkers,
    /// A single series as slices of a circle.
    Pie,
}

impl ChartKind {
    /// `true` for the kinds plotted against a category and a value axis.
    ///
    /// Pie is the exception: it has no axes at all, and emitting `<c:axId>` for
    /// it produces a file Excel refuses.
    pub(crate) fn has_axes(self) -> bool {
        !matches!(self, ChartKind::Pie)
    }
}

/// A series name: either text, or a cell holding text.
///
/// Both occur in real workbooks. A literal name carries no reference element at
/// all, which is why this is an enum rather than an `Option<String>` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeriesName {
    /// The name itself.
    Literal(String),
    /// A reference to a cell holding the name, e.g. `"'Sheet1'!$C$1"`.
    Reference(String),
}

/// One plotted series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    /// What to call it in the legend.
    pub name: SeriesName,
    /// The category labels, e.g. `"'Sheet1'!$A$3:$A$38"`. `None` numbers the
    /// categories 1, 2, 3… which is what Excel does with no category reference.
    pub categories: Option<String>,
    /// The plotted values, e.g. `"'Sheet1'!$C$3:$C$38"`.
    pub values: String,
}

/// Where the legend goes, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LegendPosition {
    /// To the right of the plot. Excel's own default.
    Right,
    /// Below the plot.
    Bottom,
    /// Above the plot.
    Top,
    /// To the left of the plot.
    Left,
    /// No legend at all.
    None,
}

impl LegendPosition {
    /// The `val` OOXML stores. `None` has none — the element is omitted.
    pub(crate) fn code(self) -> Option<&'static str> {
        match self {
            LegendPosition::Right => Some("r"),
            LegendPosition::Bottom => Some("b"),
            LegendPosition::Top => Some("t"),
            LegendPosition::Left => Some("l"),
            LegendPosition::None => None,
        }
    }
}

/// One axis of a chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axis {
    /// An axis title, if any.
    pub title: Option<String>,
    /// Whether to draw major gridlines across the plot from this axis.
    pub major_gridlines: bool,
    /// Whether the axis is drawn. A deleted axis still exists and still scales
    /// the plot — it is just not shown.
    pub visible: bool,
}

impl Default for Axis {
    /// Visible, untitled, no gridlines.
    fn default() -> Self {
        Self {
            title: None,
            major_gridlines: false,
            visible: true,
        }
    }
}

/// A chart, described rather than drawn.
///
/// Every method other than [`ChartSpec::new`] is optional: a spec with one
/// series renders a valid chart.
#[derive(Debug, Clone)]
pub struct ChartSpec {
    pub(crate) kind: ChartKind,
    pub(crate) title: Option<String>,
    pub(crate) series: Vec<Series>,
    pub(crate) legend: LegendPosition,
    pub(crate) category_axis: Axis,
    pub(crate) value_axis: Axis,
    pub(crate) gap_width: u16,
    pub(crate) overlap: Option<i16>,
}

impl ChartSpec {
    /// A chart of `kind` with defaults chosen to match Excel's own: legend on
    /// the right, both axes visible and untitled, no gridlines, no title.
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            title: None,
            series: Vec::new(),
            legend: LegendPosition::Right,
            category_axis: Axis::default(),
            value_axis: Axis::default(),
            gap_width: 150,
            overlap: None,
        }
    }

    /// Adopts an existing chart part's formatting wholesale, replacing only its
    /// data references and, optionally, its title.
    ///
    /// This is the low-risk way to add one chart to a family of charts that
    /// already exist: every colour, font, axis format and legend setting is
    /// inherited byte-for-byte from a chart that is known to render correctly,
    /// and only what must differ differs.
    ///
    /// # Errors
    ///
    /// [`ChartError::NotAChart`] if `part` holds no `<c:chartSpace>` element
    /// and no unprefixed `<chartSpace>` one. Both are accepted because both
    /// occur: Excel writes the first, and excelize — which binds the chart
    /// namespace as the document default — writes the second.
    ///
    /// [`ChartError::MixedNamespacePrefixes`] if `part` spells its
    /// chart-namespace elements both ways at once. No writer produces such a
    /// document, and reading one would mean guessing which element the chart's
    /// own title is.
    ///
    /// [`ChartError::UnterminatedComment`], [`ChartError::UnterminatedCData`],
    /// or [`ChartError::UnclosedReference`] if `part`'s reference elements
    /// cannot be scanned unambiguously — an unclosed comment or CDATA
    /// section, or a `<c:f>`/`<f>` element with no matching close tag. These
    /// are found while locating `part`'s references, before this function
    /// returns.
    pub fn from_template(part: &[u8]) -> Result<crate::TemplateChart, ChartError> {
        crate::TemplateChart::parse(part)
    }

    /// Sets the chart title.
    #[must_use]
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Appends a series. Series are drawn in the order they are added.
    #[must_use]
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Moves the legend, or removes it with [`LegendPosition::None`].
    #[must_use]
    pub fn legend(mut self, position: LegendPosition) -> Self {
        self.legend = position;
        self
    }

    /// Replaces the category axis.
    #[must_use]
    pub fn category_axis(mut self, axis: Axis) -> Self {
        self.category_axis = axis;
        self
    }

    /// Replaces the value axis.
    #[must_use]
    pub fn value_axis(mut self, axis: Axis) -> Self {
        self.value_axis = axis;
        self
    }

    /// The gap between category groups, as a percentage of bar width.
    /// Bar and column charts only; ignored by the others. Excel's default
    /// is 150.
    #[must_use]
    pub fn gap_width(mut self, percent: u16) -> Self {
        self.gap_width = percent;
        self
    }

    /// How far bars in a category overlap, as a percentage. Stacked charts
    /// default to 100; set this only to override.
    #[must_use]
    pub fn overlap(mut self, percent: i16) -> Self {
        self.overlap = Some(percent);
        self
    }

    /// Emits the chart part.
    ///
    /// # Errors
    ///
    /// [`ChartError::NoSeries`] if no series was added. A chart with no series
    /// opens, shows nothing, and gives no clue why.
    pub fn render(&self) -> Result<ChartPart, ChartError> {
        render::chart_space(self)
    }
}

/// A rendered chart part, ready to be written into a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartPart {
    /// The part's bytes, to be stored at e.g. `xl/charts/chart8.xml`.
    pub xml: Vec<u8>,
    /// The content type to register in `[Content_Types].xml`. Always
    /// [`ChartPart::CONTENT_TYPE`].
    pub content_type: &'static str,
}

impl ChartPart {
    /// The content type every chart part declares.
    pub const CONTENT_TYPE: &'static str =
        "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_series() -> Series {
        Series {
            name: SeriesName::Literal("Open".to_string()),
            categories: Some("'Sheet1'!$A$3:$A$5".to_string()),
            values: "'Sheet1'!$C$3:$C$5".to_string(),
        }
    }

    fn rendered(spec: ChartSpec) -> String {
        String::from_utf8(spec.render().expect("a chart").xml).expect("UTF-8")
    }

    #[test]
    fn a_column_chart_draws_columns_and_a_bar_chart_draws_bars() {
        let column = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(one_series()));
        assert!(column.contains(r#"<c:barDir val="col"/>"#), "{column}");

        let bar = rendered(ChartSpec::new(ChartKind::BarClustered).series(one_series()));
        assert!(bar.contains(r#"<c:barDir val="bar"/>"#), "{bar}");
    }

    #[test]
    fn a_stacked_chart_is_grouped_stacked_and_fully_overlapped() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnStacked).series(one_series()));
        assert!(out.contains(r#"<c:grouping val="stacked"/>"#), "{out}");
        assert!(out.contains(r#"<c:overlap val="100"/>"#), "{out}");
    }

    #[test]
    fn the_references_are_written_with_excel_escaping() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(one_series()));
        assert!(
            out.contains("<c:f>&#39;Sheet1&#39;!$C$3:$C$5</c:f>"),
            "{out}"
        );
    }

    #[test]
    fn a_literal_series_name_has_no_reference_element() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(one_series()));
        assert!(out.contains("<c:tx><c:v>Open</c:v></c:tx>"), "{out}");
    }

    #[test]
    fn a_referenced_series_name_is_a_string_reference() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(Series {
            name: SeriesName::Reference("'Sheet1'!$C$1".to_string()),
            categories: None,
            values: "'Sheet1'!$C$3:$C$5".to_string(),
        }));
        assert!(
            out.contains("<c:tx><c:strRef><c:f>&#39;Sheet1&#39;!$C$1</c:f></c:strRef></c:tx>"),
            "{out}"
        );
    }

    #[test]
    fn series_are_numbered_in_the_order_they_were_added() {
        let out = rendered(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(one_series())
                .series(one_series()),
        );
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
    fn a_title_is_rich_text_and_is_escaped() {
        let out = rendered(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(one_series())
                .title("R&D by region"),
        );
        assert!(out.contains("<a:t>R&amp;D by region</a:t>"), "{out}");
    }

    #[test]
    fn no_title_means_no_title_element() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(one_series()));
        assert!(!out.contains("<c:title>"), "{out}");
    }

    #[test]
    fn the_legend_position_is_a_single_letter_code() {
        let out = rendered(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(one_series())
                .legend(LegendPosition::Bottom),
        );
        assert!(out.contains(r#"<c:legendPos val="b"/>"#), "{out}");
    }

    #[test]
    fn legend_none_omits_the_legend_entirely() {
        let out = rendered(
            ChartSpec::new(ChartKind::ColumnClustered)
                .series(one_series())
                .legend(LegendPosition::None),
        );
        assert!(!out.contains("<c:legend>"), "{out}");
    }

    #[test]
    fn a_chart_with_no_series_is_refused() {
        let error = ChartSpec::new(ChartKind::ColumnClustered).render();
        assert!(matches!(error, Err(ChartError::NoSeries)));
    }

    #[test]
    fn the_two_axes_cross_each_other() {
        let out = rendered(ChartSpec::new(ChartKind::ColumnClustered).series(one_series()));
        // The category axis is 111 and crosses 222; the value axis is the reverse.
        assert!(out.contains(r#"<c:catAx><c:axId val="111"/>"#), "{out}");
        assert!(out.contains(r#"<c:crossAx val="111"/>"#), "{out}");
        assert!(out.contains(r#"<c:crossAx val="222"/>"#), "{out}");
    }

    #[test]
    fn a_line_chart_is_grouped_standard_with_markers_off() {
        let out = rendered(ChartSpec::new(ChartKind::Line).series(one_series()));
        assert!(out.contains("<c:lineChart>"), "{out}");
        assert!(out.contains(r#"<c:grouping val="standard"/>"#), "{out}");
        assert!(out.contains(r#"<c:marker val="0"/>"#), "{out}");
    }

    #[test]
    fn line_with_markers_turns_markers_on() {
        let out = rendered(ChartSpec::new(ChartKind::LineMarkers).series(one_series()));
        assert!(out.contains(r#"<c:marker val="1"/>"#), "{out}");
    }

    #[test]
    fn a_pie_chart_has_no_axes_at_all() {
        let out = rendered(ChartSpec::new(ChartKind::Pie).series(one_series()));
        assert!(out.contains("<c:pieChart>"), "{out}");
        // A pie with axis ids or axis elements is refused by Excel.
        assert!(!out.contains("<c:axId"), "{out}");
        assert!(!out.contains("<c:catAx>"), "{out}");
        assert!(!out.contains("<c:valAx>"), "{out}");
    }

    #[test]
    fn a_pie_chart_varies_its_slice_colours() {
        let out = rendered(ChartSpec::new(ChartKind::Pie).series(one_series()));
        assert!(out.contains(r#"<c:varyColors val="1"/>"#), "{out}");
    }

    #[test]
    fn an_axis_can_be_titled_and_given_gridlines() {
        let out = rendered(
            ChartSpec::new(ChartKind::Line)
                .series(one_series())
                .value_axis(Axis {
                    title: Some("Revenue".to_string()),
                    major_gridlines: true,
                    visible: true,
                }),
        );
        assert!(out.contains("<c:majorGridlines/>"), "{out}");
        assert!(out.contains("<a:t>Revenue</a:t>"), "{out}");
    }

    #[test]
    fn an_invisible_axis_is_marked_deleted_rather_than_omitted() {
        let out = rendered(
            ChartSpec::new(ChartKind::Line)
                .series(one_series())
                .category_axis(Axis {
                    visible: false,
                    ..Axis::default()
                }),
        );
        assert!(out.contains("<c:catAx>"), "{out}");
        assert!(out.contains(r#"<c:delete val="1"/>"#), "{out}");
    }

    #[test]
    fn a_series_without_categories_omits_the_category_element() {
        let out = rendered(ChartSpec::new(ChartKind::Line).series(Series {
            name: SeriesName::Literal("Open".to_string()),
            categories: None,
            values: "'Sheet1'!$C$3:$C$5".to_string(),
        }));
        assert!(!out.contains("<c:cat>"), "{out}");
        assert!(out.contains("<c:val>"), "{out}");
    }
}
