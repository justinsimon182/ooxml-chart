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
    /// Horizontal bars stacked and scaled so each category totals 100%.
    BarPercentStacked,
    /// Vertical columns stacked and scaled so each category totals 100%.
    ColumnPercentStacked,
    /// A filled area per series, drawn over one another.
    Area,
    /// Filled areas stacked into one running total.
    AreaStacked,
    /// Filled areas stacked and scaled so each category totals 100%.
    AreaPercentStacked,
    /// Points placed by a numeric x and y, no connecting lines. A series'
    /// categories reference the x values.
    Scatter,
    /// Points placed by a numeric x and y, joined by lines.
    ScatterLines,
    /// Like [`ChartKind::Pie`] with the middle cut out. Several series draw
    /// as concentric rings.
    Doughnut,
    /// One spoke per category, a line per series joining the spokes.
    Radar,
}

impl ChartKind {
    /// `true` for the kinds plotted against a category and a value axis.
    ///
    /// Pie and doughnut are the exceptions: they have no axes at all, and
    /// emitting `<c:axId>` for them produces a file Excel refuses.
    pub(crate) fn has_axes(self) -> bool {
        !matches!(self, ChartKind::Pie | ChartKind::Doughnut)
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

/// A point marker's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkerSymbol {
    /// A filled circle.
    Circle,
    /// A square.
    Square,
    /// A diamond.
    Diamond,
    /// A triangle.
    Triangle,
    /// An X.
    X,
    /// A star.
    Star,
    /// A short horizontal dash.
    Dash,
    /// A small dot.
    Dot,
    /// A plus sign.
    Plus,
    /// No marker: the line alone.
    None,
}

impl MarkerSymbol {
    pub(crate) fn code(self) -> &'static str {
        match self {
            MarkerSymbol::Circle => "circle",
            MarkerSymbol::Square => "square",
            MarkerSymbol::Diamond => "diamond",
            MarkerSymbol::Triangle => "triangle",
            MarkerSymbol::X => "x",
            MarkerSymbol::Star => "star",
            MarkerSymbol::Dash => "dash",
            MarkerSymbol::Dot => "dot",
            MarkerSymbol::Plus => "plus",
            MarkerSymbol::None => "none",
        }
    }
}

/// One plotted series.
///
/// `name` and `values` are the required core. Everything else is optional
/// formatting, set with the `with_*` methods; build with [`Series::new`] so
/// new options never break the call site.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Series {
    /// What to call it in the legend.
    pub name: SeriesName,
    /// The category labels, e.g. `"'Sheet1'!$A$3:$A$38"`. `None` numbers the
    /// categories 1, 2, 3… which is what Excel does with no category reference.
    /// On a scatter chart these are the x values.
    pub categories: Option<String>,
    /// The plotted values, e.g. `"'Sheet1'!$C$3:$C$38"`.
    pub values: String,
    /// Fill (bars, areas) or line (lines, scatter, radar) colour as `RRGGBB`.
    /// Ignored on pie and doughnut, where one colour would flatten the slices.
    pub color: Option<String>,
    /// Line width in points.
    pub line_width_pt: Option<f64>,
    /// Draw the line as a smooth curve rather than straight segments.
    pub smooth: bool,
    /// Marker shape and size in points (2-72).
    pub marker: Option<(MarkerSymbol, u8)>,
    /// Category labels to cache in the part, so viewers that do not
    /// recalculate can still draw the axis.
    pub categories_cache: Option<Vec<String>>,
    /// Values to cache in the part, so viewers that do not recalculate can
    /// still draw the plot. Non-finite entries are written as blanks.
    pub values_cache: Option<Vec<f64>>,
}

impl Series {
    /// A series with a name and a values reference, unformatted.
    pub fn new(name: SeriesName, values: impl Into<String>) -> Self {
        Self {
            name,
            categories: None,
            values: values.into(),
            color: None,
            line_width_pt: None,
            smooth: false,
            marker: None,
            categories_cache: None,
            values_cache: None,
        }
    }

    /// Sets the category (or scatter x) reference.
    #[must_use]
    pub fn with_categories(mut self, reference: impl Into<String>) -> Self {
        self.categories = Some(reference.into());
        self
    }

    /// Sets the colour, as six hex digits without `#`, e.g. `"8E0DD1"`.
    /// Checked at render time.
    #[must_use]
    pub fn with_color(mut self, rgb: impl Into<String>) -> Self {
        self.color = Some(rgb.into());
        self
    }

    /// Sets the line width in points.
    #[must_use]
    pub fn with_line_width(mut self, points: f64) -> Self {
        self.line_width_pt = Some(points);
        self
    }

    /// Draws the line as a smooth curve.
    #[must_use]
    pub fn with_smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }

    /// Sets the marker. `size` is in points and is checked at render time
    /// (2-72).
    #[must_use]
    pub fn with_marker(mut self, symbol: MarkerSymbol, size: u8) -> Self {
        self.marker = Some((symbol, size));
        self
    }

    /// Caches category labels in the part.
    #[must_use]
    pub fn with_cached_categories(mut self, labels: Vec<String>) -> Self {
        self.categories_cache = Some(labels);
        self
    }

    /// Caches values in the part.
    #[must_use]
    pub fn with_cached_values(mut self, values: Vec<f64>) -> Self {
        self.values_cache = Some(values);
        self
    }
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
///
/// Build with [`Axis::default`] and the builder methods; the struct is
/// `#[non_exhaustive]` so new options never break the call site.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Axis {
    /// An axis title, if any.
    pub title: Option<String>,
    /// Whether to draw major gridlines across the plot from this axis.
    pub major_gridlines: bool,
    /// Whether the axis is drawn. A deleted axis still exists and still scales
    /// the plot — it is just not shown.
    pub visible: bool,
    /// An Excel number format for the tick labels, e.g. `"0.0%"`.
    pub number_format: Option<String>,
    /// Fixed scale minimum. `None` lets Excel choose. Value axes and the x axis
    /// of a scatter only; not written for category axes.
    pub min: Option<f64>,
    /// Fixed scale maximum. `None` lets Excel choose.
    pub max: Option<f64>,
    /// Distance between major ticks. Value axes and the x axis of a scatter
    /// only. `None` lets Excel choose.
    pub major_unit: Option<f64>,
    /// Plot from high to low instead of low to high.
    pub reversed: bool,
}

impl Default for Axis {
    /// Visible, untitled, no gridlines, scaled automatically.
    fn default() -> Self {
        Self {
            title: None,
            major_gridlines: false,
            visible: true,
            number_format: None,
            min: None,
            max: None,
            major_unit: None,
            reversed: false,
        }
    }
}

impl Axis {
    /// Titles the axis.
    #[must_use]
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Turns major gridlines on or off.
    #[must_use]
    pub fn gridlines(mut self, on: bool) -> Self {
        self.major_gridlines = on;
        self
    }

    /// Hides the axis without removing it from the scaling.
    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Sets the tick-label number format.
    #[must_use]
    pub fn number_format(mut self, format: impl Into<String>) -> Self {
        self.number_format = Some(format.into());
        self
    }

    /// Fixes the scale minimum.
    #[must_use]
    pub fn min(mut self, value: f64) -> Self {
        self.min = Some(value);
        self
    }

    /// Fixes the scale maximum.
    #[must_use]
    pub fn max(mut self, value: f64) -> Self {
        self.max = Some(value);
        self
    }

    /// Fixes the distance between major ticks.
    #[must_use]
    pub fn major_unit(mut self, value: f64) -> Self {
        self.major_unit = Some(value);
        self
    }

    /// Plots from high to low.
    #[must_use]
    pub fn reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }
}

/// Where a data label sits relative to its point or bar.
///
/// Which positions are legal depends on the chart kind; an illegal one is
/// refused at render time with [`ChartError::InvalidDataLabelPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLabelPosition {
    /// Centred on the point or bar.
    Center,
    /// Inside the end of a bar or slice.
    InsideEnd,
    /// Inside the base of a bar.
    InsideBase,
    /// Outside the end of a bar or slice. Not allowed on stacked bars.
    OutsideEnd,
    /// Above a point (line, scatter, radar).
    Above,
    /// Below a point (line, scatter, radar).
    Below,
    /// Left of a point (line, scatter, radar).
    Left,
    /// Right of a point (line, scatter, radar).
    Right,
    /// Excel picks the best fit. Pie only.
    BestFit,
}

impl DataLabelPosition {
    pub(crate) fn code(self) -> &'static str {
        match self {
            DataLabelPosition::Center => "ctr",
            DataLabelPosition::InsideEnd => "inEnd",
            DataLabelPosition::InsideBase => "inBase",
            DataLabelPosition::OutsideEnd => "outEnd",
            DataLabelPosition::Above => "t",
            DataLabelPosition::Below => "b",
            DataLabelPosition::Left => "l",
            DataLabelPosition::Right => "r",
            DataLabelPosition::BestFit => "bestFit",
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            DataLabelPosition::Center => "center",
            DataLabelPosition::InsideEnd => "inside end",
            DataLabelPosition::InsideBase => "inside base",
            DataLabelPosition::OutsideEnd => "outside end",
            DataLabelPosition::Above => "above",
            DataLabelPosition::Below => "below",
            DataLabelPosition::Left => "left",
            DataLabelPosition::Right => "right",
            DataLabelPosition::BestFit => "best fit",
        }
    }
}

/// Labels drawn on the plotted points of every series.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct DataLabels {
    /// Show the value.
    pub value: bool,
    /// Show the category name.
    pub category: bool,
    /// Show the series name.
    pub series: bool,
    /// Show the percentage. Pie and doughnut only.
    pub percent: bool,
    /// Where to put them. `None` leaves it to Excel.
    pub position: Option<DataLabelPosition>,
    /// Number format for the labels, e.g. `"#,##0"`.
    pub number_format: Option<String>,
}

impl DataLabels {
    /// Labels showing each point's value.
    pub fn values() -> Self {
        Self {
            value: true,
            ..Self::default()
        }
    }

    /// Also show the category name.
    #[must_use]
    pub fn with_category(mut self) -> Self {
        self.category = true;
        self
    }

    /// Also show the series name.
    #[must_use]
    pub fn with_series(mut self) -> Self {
        self.series = true;
        self
    }

    /// Also show the percentage (pie and doughnut).
    #[must_use]
    pub fn with_percent(mut self) -> Self {
        self.percent = true;
        self
    }

    /// Places the labels.
    #[must_use]
    pub fn at(mut self, position: DataLabelPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the label number format.
    #[must_use]
    pub fn number_format(mut self, format: impl Into<String>) -> Self {
        self.number_format = Some(format.into());
        self
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
    pub(crate) data_labels: Option<DataLabels>,
    pub(crate) hole_size: u8,
    pub(crate) first_slice_angle: u16,
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
            data_labels: None,
            hole_size: 50,
            first_slice_angle: 0,
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

    /// Draws data labels on every series.
    #[must_use]
    pub fn data_labels(mut self, labels: DataLabels) -> Self {
        self.data_labels = Some(labels);
        self
    }

    /// The doughnut's hole as a percentage of its diameter, 10-90. Doughnut
    /// only; Excel's default is 50.
    #[must_use]
    pub fn hole_size(mut self, percent: u8) -> Self {
        self.hole_size = percent;
        self
    }

    /// Where the first pie or doughnut slice starts, in degrees clockwise from
    /// twelve o'clock, 0-360.
    #[must_use]
    pub fn first_slice_angle(mut self, degrees: u16) -> Self {
        self.first_slice_angle = degrees;
        self
    }

    /// Emits the chart part.
    ///
    /// # Errors
    ///
    /// [`ChartError::NoSeries`] if no series was added. A chart with no series
    /// opens, shows nothing, and gives no clue why.
    ///
    /// [`ChartError::InvalidColor`], [`ChartError::InvalidAxisRange`],
    /// [`ChartError::InvalidDataLabelPosition`] and [`ChartError::OutOfRange`]
    /// for settings Excel would repair away rather than honour.
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
        Series::new(
            SeriesName::Literal("Open".to_string()),
            "'Sheet1'!$C$3:$C$5",
        )
        .with_categories("'Sheet1'!$A$3:$A$5")
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
        let out = rendered(
            ChartSpec::new(ChartKind::ColumnClustered).series(Series::new(
                SeriesName::Reference("'Sheet1'!$C$1".to_string()),
                "'Sheet1'!$C$3:$C$5",
            )),
        );
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
                .value_axis(Axis::default().title("Revenue").gridlines(true)),
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
        let out = rendered(ChartSpec::new(ChartKind::Line).series(Series::new(
            SeriesName::Literal("Open".to_string()),
            "'Sheet1'!$C$3:$C$5",
        )));
        assert!(!out.contains("<c:cat>"), "{out}");
        assert!(out.contains("<c:val>"), "{out}");
    }
}
