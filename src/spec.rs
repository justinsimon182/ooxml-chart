//! Describing a chart, rather than drawing one.

use crate::error::ChartError;
use crate::render;

/// What kind of chart to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
    /// Like [`ChartKind::Radar`] with each series' outline filled.
    RadarFilled,
    /// Circles placed by a numeric x and y, sized by a third value. A
    /// series' categories reference the x values and
    /// [`Series::with_bubble_sizes`] the sizes.
    Bubble,
    /// A stock chart of high, low and close: a vertical line from each
    /// category's low to its high with a tick at the close. Takes exactly three
    /// series, in that order.
    StockHighLowClose,
    /// A stock chart of open, high, low and close: the high-low line plus up
    /// and down bars between open and close. Takes exactly four series, in that
    /// order. Colour the bars with [`ChartSpec::stock_bars`].
    StockOpenHighLowClose,
    /// A pie whose smaller slices are gathered into a second, smaller pie
    /// beside it. One series. Choose what moves with [`ChartSpec::of_pie`].
    PieOfPie,
    /// Like [`ChartKind::PieOfPie`], but the second plot is a stacked bar.
    BarOfPie,
    /// Values as a surface stretched over a grid of categories (across) and
    /// series (into the depth), coloured in bands by value. Set the band
    /// colours with [`ChartSpec::surface_bands`] and the view with
    /// [`ChartSpec::view_3d`].
    Surface,
    /// Like [`ChartKind::Surface`] drawn as a wire mesh, without the coloured
    /// bands.
    SurfaceWireframe,
    /// A surface seen from straight above: coloured bands of equal value, like
    /// a contour map.
    Contour,
    /// Like [`ChartKind::Contour`] drawn as lines only.
    ContourWireframe,
    /// Lines stacked into one running total, no point markers.
    LineStacked,
    /// Lines stacked and scaled so each category totals 100%, no markers.
    LinePercentStacked,
    /// Stacked lines with a marker at each point.
    LineMarkersStacked,
    /// Percent-stacked lines with a marker at each point.
    LineMarkersPercentStacked,
}

impl ChartKind {
    /// `true` for the kinds plotted against a category and a value axis.
    ///
    /// Pie and doughnut are the exceptions: they have no axes at all, and
    /// emitting `<c:axId>` for them produces a file Excel refuses.
    pub(crate) fn has_axes(self) -> bool {
        !matches!(
            self,
            ChartKind::Pie | ChartKind::Doughnut | ChartKind::PieOfPie | ChartKind::BarOfPie
        )
    }
}

/// A series name: either text, or a cell holding text.
///
/// Both occur in real workbooks. A literal name carries no reference element at
/// all, which is why this is an enum rather than an `Option<String>` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum SeriesName {
    /// The name itself.
    Literal(String),
    /// A reference to a cell holding the name, e.g. `"'Sheet1'!$C$1"`.
    Reference(String),
}

/// A point marker's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

/// Formatting for one point of a series.
///
/// Honoured on bar, column, pie, doughnut and bubble series; ignored on kinds
/// where a single point has no shape of its own to colour.
///
/// On line, scatter and (unfilled) radar series the point's shape is its
/// marker: `marker` restyles it and `color` recolours it, fill and border.
/// `marker` and `explosion` are ignored on the kinds that have no such
/// thing.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct PointFormat {
    /// Fill colour as `RRGGBB`.
    pub color: Option<String>,
    /// A gradient, pattern or other fill; it wins over `color`.
    pub fill: Option<Paint>,
    /// How far a pie slice is pulled out from the centre, as a percentage of
    /// the radius (0-400). Pie and doughnut only.
    pub explosion: Option<u16>,
    /// Marker shape and size in points (2-72) for this point alone. Line,
    /// scatter and unfilled radar series only.
    pub marker: Option<(MarkerSymbol, u8)>,
    /// Shadow, glow and soft edge for this point alone. Bar, column, pie,
    /// doughnut and bubble series only; other kinds refuse it.
    pub effects: Option<Effects>,
}

impl PointFormat {
    /// Gives this point its own shadow, glow or soft edge.
    #[must_use]
    pub fn effects(mut self, effects: Effects) -> Self {
        self.effects = Some(effects);
        self
    }

    /// Gives this point its own marker, `size` in points (checked at render
    /// time, 2-72).
    #[must_use]
    pub fn marker(mut self, symbol: MarkerSymbol, size: u8) -> Self {
        self.marker = Some((symbol, size));
        self
    }

    /// An unformatted point; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fill colour, six hex digits without `#`.
    #[must_use]
    pub fn color(mut self, rgb: impl Into<String>) -> Self {
        self.color = Some(rgb.into());
        self
    }

    /// Fills the point with a gradient, a pattern or any other [`Paint`].
    #[must_use]
    pub fn fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }

    /// Pulls a pie slice out from the centre, 0-400 percent of the radius.
    #[must_use]
    pub fn explosion(mut self, percent: u16) -> Self {
        self.explosion = Some(percent);
        self
    }
}

/// The curve a trendline fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum TrendlineKind {
    /// A straight line of best fit.
    Linear,
    /// `y = c * e^(b*x)`. Needs positive values.
    Exponential,
    /// `y = c * ln(x) + b`. Needs positive x.
    Logarithmic,
    /// A polynomial of the given order, 2-6.
    Polynomial(u8),
    /// `y = c * x^b`. Needs positive values.
    Power,
    /// A moving average over the given number of points, at least 2.
    MovingAverage(u16),
}

/// A fitted line drawn over a series.
///
/// Excel accepts trendlines only on unstacked bar, column, line, area,
/// scatter and bubble series; anything else is refused at render time.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Trendline {
    /// The curve to fit.
    pub kind: TrendlineKind,
    /// Legend entry. `None` lets Excel name it after the fit.
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: Option<String>,
    /// Line colour as `RRGGBB`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub color: Option<String>,
    /// Line width in points.
    #[cfg_attr(feature = "serde", serde(default))]
    pub width_pt: Option<f64>,
    /// Dash pattern of the line.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dash: Option<Dash>,
    /// Periods to extend forward past the last point.
    #[cfg_attr(feature = "serde", serde(default))]
    pub forward: Option<f64>,
    /// Periods to extend backward before the first point.
    #[cfg_attr(feature = "serde", serde(default))]
    pub backward: Option<f64>,
    /// Print the equation on the chart.
    #[cfg_attr(feature = "serde", serde(default))]
    pub show_equation: bool,
    /// Print R-squared on the chart.
    #[cfg_attr(feature = "serde", serde(default))]
    pub show_r_squared: bool,
}

impl Trendline {
    /// A trendline fitting `kind`, otherwise default.
    pub fn new(kind: TrendlineKind) -> Self {
        Self {
            kind,
            name: None,
            color: None,
            width_pt: None,
            dash: None,
            forward: None,
            backward: None,
            show_equation: false,
            show_r_squared: false,
        }
    }

    /// Names the legend entry.
    #[must_use]
    pub fn name(mut self, text: impl Into<String>) -> Self {
        self.name = Some(text.into());
        self
    }

    /// Sets the line colour, six hex digits without `#`.
    #[must_use]
    pub fn color(mut self, rgb: impl Into<String>) -> Self {
        self.color = Some(rgb.into());
        self
    }

    /// Sets the line width in points.
    #[must_use]
    pub fn width(mut self, points: f64) -> Self {
        self.width_pt = Some(points);
        self
    }

    /// Draws the line dashed or dotted.
    #[must_use]
    pub fn dash(mut self, dash: Dash) -> Self {
        self.dash = Some(dash);
        self
    }

    /// Extends the line forward by `periods`.
    #[must_use]
    pub fn forward(mut self, periods: f64) -> Self {
        self.forward = Some(periods);
        self
    }

    /// Extends the line backward by `periods`.
    #[must_use]
    pub fn backward(mut self, periods: f64) -> Self {
        self.backward = Some(periods);
        self
    }

    /// Prints the equation on the chart.
    #[must_use]
    pub fn show_equation(mut self) -> Self {
        self.show_equation = true;
        self
    }

    /// Prints R-squared on the chart.
    #[must_use]
    pub fn show_r_squared(mut self) -> Self {
        self.show_r_squared = true;
        self
    }
}

/// One plotted series.
///
/// `name` and `values` are the required core. Everything else is optional
/// formatting, set with the `with_*` methods; build with [`Series::new`] so
/// new options never break the call site.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Series {
    /// What to call it in the legend.
    pub name: SeriesName,
    /// The category labels, e.g. `"'Sheet1'!$A$3:$A$38"`. `None` numbers the
    /// categories 1, 2, 3… which is what Excel does with no category reference.
    /// On a scatter or bubble chart these are the x values.
    #[cfg_attr(feature = "serde", serde(default))]
    pub categories: Option<String>,
    /// The plotted values, e.g. `"'Sheet1'!$C$3:$C$38"`.
    pub values: String,
    /// Fill (bars, areas, bubbles, filled radar) or line (lines, scatter,
    /// radar) colour as `RRGGBB`. Ignored on pie and doughnut, where one
    /// colour would flatten the slices; use [`Series::with_point`] there.
    #[cfg_attr(feature = "serde", serde(default))]
    pub color: Option<String>,
    /// A gradient, pattern or other fill for bars, areas, bubbles and filled
    /// radar; it wins over `color` there. Refused on line-like kinds and on
    /// pie and doughnut, which colour points: see [`PointFormat::fill`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub fill: Option<Paint>,
    /// Shadow, glow and soft edge. Refused on stock charts and on a scatter
    /// chart without lines, where the series draws no line to carry them.
    #[cfg_attr(feature = "serde", serde(default))]
    pub effects: Option<Effects>,
    /// Line width in points.
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_width_pt: Option<f64>,
    /// Dash pattern of the series line. Only series drawn as a line take it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub dash: Option<Dash>,
    /// Strokes of the series line. Only series drawn as a line take it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub compound: Option<Compound>,
    /// Arrowheads at the start and end of the series line. Only series
    /// drawn as a line take them.
    #[cfg_attr(feature = "serde", serde(default))]
    pub arrows: (Option<Arrow>, Option<Arrow>),
    /// Line ends and dash ends; Excel's default is round. Only series drawn
    /// as a line take it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cap: Option<LineCap>,
    /// Draw the line as a smooth curve rather than straight segments.
    #[cfg_attr(feature = "serde", serde(default))]
    pub smooth: bool,
    /// Marker shape and size in points (2-72).
    #[cfg_attr(feature = "serde", serde(default))]
    pub marker: Option<(MarkerSymbol, u8)>,
    /// Read `categories` as several columns (or rows) of nested labels, such as
    /// a year column beside a month column. Set with
    /// [`Series::with_multi_level_categories`]. Excel reads the labels from the
    /// sheet, so no cache is written and `categories_cache` is refused.
    #[cfg_attr(feature = "serde", serde(default))]
    pub multi_level_categories: bool,
    /// Category labels to cache in the part, so viewers that do not
    /// recalculate can still draw the axis.
    #[cfg_attr(feature = "serde", serde(default))]
    pub categories_cache: Option<Vec<String>>,
    /// Pie, doughnut, pie-of-pie and bar-of-pie only: pull every slice out
    /// from the centre by this percentage of the radius (0-400).
    #[cfg_attr(feature = "serde", serde(default))]
    pub explosion: Option<u16>,
    /// Values to cache in the part, so viewers that do not recalculate can
    /// still draw the plot. Non-finite entries are written as blanks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub values_cache: Option<Vec<f64>>,
    /// The text of the referenced name cell, cached in the part. Needs a
    /// [`SeriesName::Reference`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub name_cache: Option<String>,
    /// Reference to the bubble sizes. Required on a bubble chart, ignored
    /// elsewhere.
    #[cfg_attr(feature = "serde", serde(default))]
    pub bubble_sizes: Option<String>,
    /// Per-point formatting, by zero-based point index.
    #[cfg_attr(feature = "serde", serde(default))]
    pub points: Vec<(usize, PointFormat)>,
    /// Per-point data-label overrides, by zero-based point index.
    #[cfg_attr(feature = "serde", serde(default))]
    pub point_labels: Vec<(usize, PointLabel)>,
    /// Error bars, at most one per direction.
    #[cfg_attr(feature = "serde", serde(default))]
    pub error_bars: Vec<ErrorBars>,
    /// Fitted lines drawn over the series.
    #[cfg_attr(feature = "serde", serde(default))]
    pub trendlines: Vec<Trendline>,
    /// Cells whose text labels the points ("Value From Cells"), e.g.
    /// `"'Sheet1'!$D$2:$D$5"`. Shown when the chart's [`DataLabels`] ask for
    /// cell text.
    #[cfg_attr(feature = "serde", serde(default))]
    pub label_range: Option<String>,
    /// The text of `label_range`, cached in the part.
    #[cfg_attr(feature = "serde", serde(default))]
    pub label_range_cache: Option<Vec<String>>,
}

impl Series {
    /// A series with a name and a values reference, unformatted.
    pub fn new(name: SeriesName, values: impl Into<String>) -> Self {
        Self {
            name,
            categories: None,
            values: values.into(),
            color: None,
            fill: None,
            effects: None,
            line_width_pt: None,
            dash: None,
            compound: None,
            arrows: (None, None),
            cap: None,
            smooth: false,
            marker: None,
            multi_level_categories: false,
            categories_cache: None,
            explosion: None,
            values_cache: None,
            name_cache: None,
            bubble_sizes: None,
            points: Vec::new(),
            point_labels: Vec::new(),
            error_bars: Vec::new(),
            trendlines: Vec::new(),
            label_range: None,
            label_range_cache: None,
        }
    }

    /// Sets the category (or scatter x) reference.
    #[must_use]
    pub fn with_categories(mut self, reference: impl Into<String>) -> Self {
        self.categories = Some(reference.into());
        self
    }

    /// Sets the categories to a range of two or more columns (or rows) read as
    /// nested labels, e.g. `"'Sheet1'!$A$2:$B$13"`. Not for scatter or bubble
    /// charts, and no category cache can accompany it.
    #[must_use]
    pub fn with_multi_level_categories(mut self, reference: impl Into<String>) -> Self {
        self.categories = Some(reference.into());
        self.multi_level_categories = true;
        self
    }

    /// Fills the series with a gradient, a pattern or any other [`Paint`].
    #[must_use]
    pub fn with_fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }

    /// Adds shadow, glow or soft-edge effects to the series.
    #[must_use]
    pub fn with_effects(mut self, effects: Effects) -> Self {
        self.effects = Some(effects);
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

    /// Draws the series line dashed or dotted: line, radar and scatter
    /// series with lines only.
    #[must_use]
    pub fn with_dash(mut self, dash: Dash) -> Self {
        self.dash = Some(dash);
        self
    }

    /// Draws the series line as several strokes, like [`Self::with_dash`].
    #[must_use]
    pub fn with_compound(mut self, compound: Compound) -> Self {
        self.compound = Some(compound);
        self
    }

    /// Puts arrowheads on the series line's start and end, like
    /// [`Self::with_dash`].
    #[must_use]
    pub fn with_arrows(mut self, start: Option<Arrow>, end: Option<Arrow>) -> Self {
        self.arrows = (start, end);
        self
    }

    /// Sets how the series line's ends and dashes are capped, like
    /// [`Self::with_dash`].
    #[must_use]
    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = Some(cap);
        self
    }

    /// Pulls every slice out from the centre by `percent` of the radius
    /// (0-400). Pie, doughnut, pie-of-pie and bar-of-pie only.
    #[must_use]
    pub fn with_explosion(mut self, percent: u16) -> Self {
        self.explosion = Some(percent);
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

    /// Labels the points with the text of these cells. The chart's
    /// [`DataLabels`] must ask for it with [`DataLabels::with_cells`].
    #[must_use]
    pub fn with_label_range(mut self, reference: impl Into<String>) -> Self {
        self.label_range = Some(reference.into());
        self
    }

    /// Caches the label range's text in the part. Needs `with_label_range`.
    #[must_use]
    pub fn with_cached_label_range(mut self, labels: Vec<String>) -> Self {
        self.label_range_cache = Some(labels);
        self
    }

    /// Caches category labels in the part.
    #[must_use]
    pub fn with_cached_categories(mut self, labels: Vec<String>) -> Self {
        self.categories_cache = Some(labels);
        self
    }

    /// Caches the text of a referenced series name in the part, so a viewer
    /// that does not recalculate still shows it. Needs [`SeriesName::Reference`]:
    /// anything else is refused at render time.
    #[must_use]
    pub fn with_cached_name(mut self, text: impl Into<String>) -> Self {
        self.name_cache = Some(text.into());
        self
    }

    /// Caches values in the part.
    #[must_use]
    pub fn with_cached_values(mut self, values: Vec<f64>) -> Self {
        self.values_cache = Some(values);
        self
    }

    /// Sets the bubble sizes reference. Bubble charts only.
    #[must_use]
    pub fn with_bubble_sizes(mut self, reference: impl Into<String>) -> Self {
        self.bubble_sizes = Some(reference.into());
        self
    }

    /// Formats one point, by zero-based index. A later call for the same index
    /// replaces the earlier one.
    #[must_use]
    pub fn with_point(mut self, index: usize, format: PointFormat) -> Self {
        self.points.retain(|(existing, _)| *existing != index);
        self.points.push((index, format));
        self
    }

    /// Overrides the data label of one point, by zero-based index: hide it,
    /// give it custom text, move it or restyle it. The rest inherit the
    /// chart-wide [`DataLabels`]. A later call for the same index replaces
    /// the earlier one.
    #[must_use]
    pub fn with_point_label(mut self, index: usize, label: PointLabel) -> Self {
        self.point_labels.retain(|(existing, _)| *existing != index);
        self.point_labels.push((index, label));
        self
    }

    /// Draws error bars on the series. A scatter or bubble series takes one
    /// per direction (see [`ErrorBars::along_x`]); any other at most one.
    #[must_use]
    pub fn with_error_bars(mut self, bars: ErrorBars) -> Self {
        self.error_bars.push(bars);
        self
    }

    /// Draws a trendline over the series. Several may be added.
    #[must_use]
    pub fn with_trendline(mut self, trendline: Trendline) -> Self {
        self.trendlines.push(trendline);
        self
    }
}

/// Where the custom error amounts of [`ErrorAmount::Custom`] come from.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ErrorValues {
    /// A worksheet range, one amount per point, e.g. `"'Sheet1'!$D$2:$D$5"`.
    Reference(String),
    /// Amounts written into the chart itself, one per point.
    Literal(Vec<f64>),
}

/// How far an error bar reaches.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ErrorAmount {
    /// The same amount, in value units, for every point.
    Fixed(f64),
    /// A percentage of each point's value.
    Percentage(f64),
    /// This many standard deviations of the series.
    StdDev(f64),
    /// The standard error of the series.
    StdErr,
    /// An amount per point, for each side in use. A side not in use (see
    /// [`ErrorBarSide`]) may be left out; one that is in use must be given.
    Custom {
        /// Amounts above (or to the right of) each point.
        #[cfg_attr(feature = "serde", serde(default))]
        plus: Option<ErrorValues>,
        /// Amounts below (or to the left of) each point.
        #[cfg_attr(feature = "serde", serde(default))]
        minus: Option<ErrorValues>,
    },
}

/// Which side of a point an error bar extends to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ErrorBarSide {
    /// Above and below.
    #[default]
    Both,
    /// Above (or to the right) only.
    Plus,
    /// Below (or to the left) only.
    Minus,
}

/// The direction an error bar points in. Only scatter and bubble series have
/// an x direction; every other kind measures error along the value axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ErrorAxis {
    /// Along the value axis.
    #[default]
    Y,
    /// Along the x axis. Scatter and bubble only.
    X,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

/// Error bars on a series, set with [`Series::with_error_bars`].
///
/// Excel draws them on bar, column, line, area, scatter and bubble series and
/// nowhere else; anything else is refused at render time, as is an x-direction
/// bar on a kind with no x values to be uncertain about. A scatter or bubble
/// series may carry one bar per direction; every other series at most one.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct ErrorBars {
    /// How far the bars reach.
    pub amount: ErrorAmount,
    /// Which sides are drawn. Both by default.
    #[cfg_attr(feature = "serde", serde(default))]
    pub side: ErrorBarSide,
    /// The direction. Y by default.
    #[cfg_attr(feature = "serde", serde(default))]
    pub axis: ErrorAxis,
    /// Draw the small cap at the end of each bar. On by default.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub end_cap: bool,
    /// Line colour as `RRGGBB`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub color: Option<String>,
    /// Line width in points.
    #[cfg_attr(feature = "serde", serde(default))]
    pub width_pt: Option<f64>,
}

impl ErrorBars {
    /// Error bars of `amount`, drawn both ways along y with end caps.
    pub fn new(amount: ErrorAmount) -> Self {
        Self {
            amount,
            side: ErrorBarSide::Both,
            axis: ErrorAxis::Y,
            end_cap: true,
            color: None,
            width_pt: None,
        }
    }

    /// Draws only the given side.
    #[must_use]
    pub fn side(mut self, side: ErrorBarSide) -> Self {
        self.side = side;
        self
    }

    /// Points the bars along x. Scatter and bubble only.
    #[must_use]
    pub fn along_x(mut self) -> Self {
        self.axis = ErrorAxis::X;
        self
    }

    /// Turns the end caps on or off.
    #[must_use]
    pub fn end_cap(mut self, on: bool) -> Self {
        self.end_cap = on;
        self
    }

    /// Sets the line colour, six hex digits without `#`.
    #[must_use]
    pub fn color(mut self, rgb: impl Into<String>) -> Self {
        self.color = Some(rgb.into());
        self
    }

    /// Sets the line width in points.
    #[must_use]
    pub fn width(mut self, points: f64) -> Self {
        self.width_pt = Some(points);
        self
    }
}

/// A data table under the plot area: the series values in a grid that lines
/// up with the categories.
///
/// Every part is on by default, as Excel draws it. Only column, line and area
/// charts take one; set with [`ChartSpec::data_table`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct DataTable {
    /// Horizontal lines between the rows.
    pub horizontal_borders: bool,
    /// Vertical lines between the columns.
    pub vertical_borders: bool,
    /// A border around the whole table.
    pub outline: bool,
    /// Legend keys (series colour swatches) beside the row names.
    pub legend_keys: bool,
    /// Font for the table text. `None` leaves Excel's default.
    pub style: Option<TextStyle>,
}

impl Default for DataTable {
    fn default() -> Self {
        Self {
            horizontal_borders: true,
            vertical_borders: true,
            outline: true,
            legend_keys: true,
            style: None,
        }
    }
}

impl DataTable {
    /// A table with every border and the legend keys; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows or hides the horizontal lines between rows.
    #[must_use]
    pub fn horizontal_borders(mut self, show: bool) -> Self {
        self.horizontal_borders = show;
        self
    }

    /// Shows or hides the vertical lines between columns.
    #[must_use]
    pub fn vertical_borders(mut self, show: bool) -> Self {
        self.vertical_borders = show;
        self
    }

    /// Shows or hides the border around the table.
    #[must_use]
    pub fn outline(mut self, show: bool) -> Self {
        self.outline = show;
        self
    }

    /// Shows or hides the legend keys beside the row names.
    #[must_use]
    pub fn legend_keys(mut self, show: bool) -> Self {
        self.legend_keys = show;
        self
    }

    /// Sets the table font.
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }
}

/// A manual position for a chart title or an axis title.
///
/// `x` and `y` are fractions of the whole chart, measured from its top-left
/// corner, and place the title's top-left corner. A title sizes itself, so
/// there is no width or height. Set with [`ChartSpec::title_position`] or
/// [`Axis::title_position`]; without one Excel places the title itself.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Position {
    /// Left edge, from 0 to 1.
    pub x: f64,
    /// Top edge, from 0 to 1.
    pub y: f64,
}

impl Position {
    /// A position at (`x`, `y`), as fractions of the chart. Checked at render
    /// time.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A manual position and size for the plot area or the legend.
///
/// Every number is a fraction of the whole chart, measured from its top-left
/// corner: `x` and `y` place the rectangle's top-left corner, `width` and
/// `height` size it. `Layout::new(0.1, 0.15, 0.7, 0.65)` leaves a tenth of the
/// chart free on the left and 15% on top.
///
/// Set with [`ChartSpec::plot_area_layout`] or [`ChartSpec::legend_layout`].
/// Without one Excel lays the element out itself.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Layout {
    /// Left edge, from 0 to 1.
    pub x: f64,
    /// Top edge, from 0 to 1.
    pub y: f64,
    /// Width, above 0 and at most 1.
    pub width: f64,
    /// Height, above 0 and at most 1.
    pub height: f64,
    /// Plot area only: size the rectangle *inside* the axes, leaving the tick
    /// labels and axis titles outside it (on by default, as Excel writes it).
    /// Off, the rectangle includes them. Ignored for a legend.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub inner: bool,
}

impl Layout {
    /// A layout at (`x`, `y`) of size `width` by `height`, all as fractions of
    /// the chart. Checked at render time.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
            inner: true,
        }
    }

    /// For a plot area: include tick labels and axis titles in the rectangle.
    #[must_use]
    pub fn outer(mut self) -> Self {
        self.inner = false;
        self
    }
}

/// Font settings for a piece of chart text.
///
/// Unset fields inherit from the chart's text style, then from Excel.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct TextStyle {
    /// Size in points, 1-4000.
    pub size_pt: Option<f64>,
    /// Bold or not.
    pub bold: Option<bool>,
    /// Italic or not.
    pub italic: Option<bool>,
    /// Colour as `RRGGBB`.
    pub color: Option<String>,
    /// Typeface name, e.g. `"Inter"`.
    pub font: Option<String>,
    /// Underline style.
    pub underline: Option<Underline>,
    /// Extra space between characters in points; negative condenses.
    pub spacing_pt: Option<f64>,
    /// Capitals.
    pub caps: Option<Caps>,
    /// Raises (positive) or lowers the text by a percentage of its size:
    /// 30 is Excel's superscript, -25 its subscript. -100 to 100.
    pub baseline_pct: Option<i16>,
    /// Single strikethrough or not.
    pub strike: Option<bool>,
}

/// One run of a title: a piece of text with its own formatting.
///
/// Give a title several runs to mix formats in it, e.g. a bold word followed
/// by a smaller grey one. A run with no style takes the title's own
/// `title_style`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct TitleRun {
    /// The run's text. A newline starts a new line of the title.
    pub text: String,
    /// This run's formatting. `None` uses the title's `title_style`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub style: Option<TextStyle>,
}

impl TitleRun {
    /// A run that takes the title's style.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: None,
        }
    }

    /// Gives this run its own formatting.
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }
}

impl TextStyle {
    /// An empty style; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the size in points.
    #[must_use]
    pub fn size(mut self, points: f64) -> Self {
        self.size_pt = Some(points);
        self
    }

    /// Sets bold on or off.
    #[must_use]
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Sets italic on or off.
    #[must_use]
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Underlines the text; [`Underline::None`] stops an inherited one.
    #[must_use]
    pub fn underline(mut self, underline: Underline) -> Self {
        self.underline = Some(underline);
        self
    }

    /// Spaces the characters `points` further apart; negative condenses.
    #[must_use]
    pub fn spacing(mut self, points: f64) -> Self {
        self.spacing_pt = Some(points);
        self
    }

    /// Sets capitals.
    #[must_use]
    pub fn caps(mut self, caps: Caps) -> Self {
        self.caps = Some(caps);
        self
    }

    /// Raises or lowers the text by `percent` of its size: 30 is a
    /// superscript, -25 a subscript.
    #[must_use]
    pub fn baseline(mut self, percent: i16) -> Self {
        self.baseline_pct = Some(percent);
        self
    }

    /// Strikes the text through, or stops an inherited strikethrough.
    #[must_use]
    pub fn strike(mut self, strike: bool) -> Self {
        self.strike = Some(strike);
        self
    }

    /// Sets the colour, six hex digits without `#`.
    #[must_use]
    pub fn color(mut self, rgb: impl Into<String>) -> Self {
        self.color = Some(rgb.into());
        self
    }

    /// Sets the typeface.
    #[must_use]
    pub fn font(mut self, name: impl Into<String>) -> Self {
        self.font = Some(name.into());
        self
    }
}

/// A fill or line: nothing, a solid colour, a gradient or a pattern.
///
/// A gradient or pattern suits fills (chart and plot areas, bars, areas,
/// slices) and is allowed on lines too; Excel draws them as it would from its
/// own dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Paint {
    /// Draw nothing.
    None,
    /// A solid colour, six hex digits without `#`.
    Color(String),
    /// A linear colour blend.
    Gradient(Gradient),
    /// A two-colour pattern.
    Pattern(Pattern),
    /// An image, reached through a relationship on the chart part. A fill
    /// only: lines refuse it.
    Picture(PictureFill),
    /// A solid theme or system colour, or one made darker, lighter or
    /// partly transparent; see [`Color`].
    Solid(Color),
}

/// A colour slot of the workbook theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ThemeColor {
    /// Text 1, usually black.
    Text1,
    /// Background 1, usually white.
    Background1,
    /// Text 2.
    Text2,
    /// Background 2.
    Background2,
    /// Accent 1.
    Accent1,
    /// Accent 2.
    Accent2,
    /// Accent 3.
    Accent3,
    /// Accent 4.
    Accent4,
    /// Accent 5.
    Accent5,
    /// Accent 6.
    Accent6,
    /// Hyperlink.
    Hyperlink,
    /// Followed hyperlink.
    FollowedHyperlink,
}

impl ThemeColor {
    /// Every value.
    pub const ALL: [ThemeColor; 12] = [
        ThemeColor::Text1,
        ThemeColor::Background1,
        ThemeColor::Text2,
        ThemeColor::Background2,
        ThemeColor::Accent1,
        ThemeColor::Accent2,
        ThemeColor::Accent3,
        ThemeColor::Accent4,
        ThemeColor::Accent5,
        ThemeColor::Accent6,
        ThemeColor::Hyperlink,
        ThemeColor::FollowedHyperlink,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Text1 => "tx1",
            Self::Background1 => "bg1",
            Self::Text2 => "tx2",
            Self::Background2 => "bg2",
            Self::Accent1 => "accent1",
            Self::Accent2 => "accent2",
            Self::Accent3 => "accent3",
            Self::Accent4 => "accent4",
            Self::Accent5 => "accent5",
            Self::Accent6 => "accent6",
            Self::Hyperlink => "hlink",
            Self::FollowedHyperlink => "folHlink",
        }
    }
}

/// A colour of the operating system's scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum SystemColor {
    /// The window background, usually white.
    Window,
    /// Window text, usually black.
    WindowText,
}

impl SystemColor {
    /// Every value.
    pub const ALL: [SystemColor; 2] = [SystemColor::Window, SystemColor::WindowText];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::WindowText => "windowText",
        }
    }
}

impl SystemColor {
    /// The value Excel records beside the name.
    pub(crate) fn last(self) -> &'static str {
        match self {
            Self::Window => "FFFFFF",
            Self::WindowText => "000000",
        }
    }
}

/// Where a [`Color`] takes its hue from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ColorBase {
    /// Six hex digits without `#`.
    Rgb(String),
    /// A slot of the workbook theme, so the colour follows a theme change.
    Theme(ThemeColor),
    /// A colour of the operating system's scheme.
    System(SystemColor),
}

/// A colour that is more than six hex digits: a theme or system colour,
/// made darker or lighter, or partly transparent. Use it through
/// [`Paint::Solid`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Color {
    /// The hue.
    pub base: ColorBase,
    /// Luminance times this percentage, 0-100.
    #[cfg_attr(feature = "serde", serde(default))]
    pub lum_mod: Option<u8>,
    /// Then plus this percentage of full luminance, 0-100.
    #[cfg_attr(feature = "serde", serde(default))]
    pub lum_off: Option<u8>,
    /// Transparency, 0-100 percent.
    #[cfg_attr(feature = "serde", serde(default))]
    pub transparency: Option<u8>,
}

impl Color {
    fn of(base: ColorBase) -> Self {
        Self {
            base,
            lum_mod: None,
            lum_off: None,
            transparency: None,
        }
    }

    /// An RGB colour, six hex digits without `#`.
    pub fn rgb(rgb: impl Into<String>) -> Self {
        Self::of(ColorBase::Rgb(rgb.into()))
    }

    /// A theme colour.
    pub fn theme(slot: ThemeColor) -> Self {
        Self::of(ColorBase::Theme(slot))
    }

    /// A system colour.
    pub fn system(color: SystemColor) -> Self {
        Self::of(ColorBase::System(color))
    }

    /// Darker by `percent` (above 100 counts as 100), as Excel's colour
    /// picker's "Darker 25%" is `darker(25)`.
    #[must_use]
    pub fn darker(mut self, percent: u8) -> Self {
        self.lum_mod = Some(100u8.saturating_sub(percent));
        self.lum_off = None;
        self
    }

    /// Lighter by `percent` (above 100 counts as 100), as Excel's colour
    /// picker's "Lighter 40%" is `lighter(40)`.
    #[must_use]
    pub fn lighter(mut self, percent: u8) -> Self {
        let percent = percent.min(100);
        self.lum_mod = Some(100 - percent);
        self.lum_off = Some(percent);
        self
    }

    /// Makes the colour `percent` transparent, 0-100.
    #[must_use]
    pub fn transparency(mut self, percent: u8) -> Self {
        self.transparency = Some(percent);
        self
    }
}

/// A corner a gradient spreads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Corner {
    /// Top left.
    TopLeft,
    /// Top right.
    TopRight,
    /// Bottom left.
    BottomLeft,
    /// Bottom right.
    BottomRight,
}

impl Corner {
    /// Every value.
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];
}

/// The point a scaled or skewed shadow is anchored to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ShadowAlign {
    /// Top left.
    TopLeft,
    /// Top.
    Top,
    /// Top right.
    TopRight,
    /// Left.
    Left,
    /// Centre.
    Center,
    /// Right.
    Right,
    /// Bottom left.
    BottomLeft,
    /// Bottom.
    Bottom,
    /// Bottom right.
    BottomRight,
}

impl ShadowAlign {
    /// Every value.
    pub const ALL: [ShadowAlign; 9] = [
        ShadowAlign::TopLeft,
        ShadowAlign::Top,
        ShadowAlign::TopRight,
        ShadowAlign::Left,
        ShadowAlign::Center,
        ShadowAlign::Right,
        ShadowAlign::BottomLeft,
        ShadowAlign::Bottom,
        ShadowAlign::BottomRight,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::TopLeft => "tl",
            Self::Top => "t",
            Self::TopRight => "tr",
            Self::Left => "l",
            Self::Center => "ctr",
            Self::Right => "r",
            Self::BottomLeft => "bl",
            Self::Bottom => "b",
            Self::BottomRight => "br",
        }
    }
}

/// How the ends of a line, and of each dash, are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum LineCap {
    /// Rounded.
    Round,
    /// Squared off past the end.
    Square,
    /// Cut at the end.
    Flat,
}

impl LineCap {
    /// Every value.
    pub const ALL: [LineCap; 3] = [LineCap::Round, LineCap::Square, LineCap::Flat];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Round => "rnd",
            Self::Square => "sq",
            Self::Flat => "flat",
        }
    }
}

/// An underline style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Underline {
    /// No underline.
    None,
    /// Single.
    Single,
    /// Double.
    Double,
    /// Heavy.
    Heavy,
    /// Dotted.
    Dotted,
    /// Dashed.
    Dash,
    /// Wavy.
    Wavy,
    /// Under words, not spaces.
    Words,
}

impl Underline {
    /// Every value.
    pub const ALL: [Underline; 8] = [
        Underline::None,
        Underline::Single,
        Underline::Double,
        Underline::Heavy,
        Underline::Dotted,
        Underline::Dash,
        Underline::Wavy,
        Underline::Words,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Single => "sng",
            Self::Double => "dbl",
            Self::Heavy => "heavy",
            Self::Dotted => "dotted",
            Self::Dash => "dash",
            Self::Wavy => "wavy",
            Self::Words => "words",
        }
    }
}

/// Capitals applied to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Caps {
    /// As typed.
    None,
    /// Small capitals.
    Small,
    /// All capitals.
    All,
}

impl Caps {
    /// Every value.
    pub const ALL: [Caps; 3] = [Caps::None, Caps::Small, Caps::All];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Small => "small",
            Self::All => "all",
        }
    }
}

/// The dash pattern of a line: DrawingML's preset dashes (`a:prstDash`).
/// The `System` ones scale with the line width less than the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Dash {
    /// `solid`.
    Solid,
    /// `dot`.
    Dot,
    /// `dash`.
    Dash,
    /// `lgDash`.
    LongDash,
    /// `dashDot`.
    DashDot,
    /// `lgDashDot`.
    LongDashDot,
    /// `lgDashDotDot`.
    LongDashDotDot,
    /// `sysDash`.
    SystemDash,
    /// `sysDot`.
    SystemDot,
    /// `sysDashDot`.
    SystemDashDot,
    /// `sysDashDotDot`.
    SystemDashDotDot,
}

impl Dash {
    /// Every value.
    pub const ALL: [Dash; 11] = [
        Dash::Solid,
        Dash::Dot,
        Dash::Dash,
        Dash::LongDash,
        Dash::DashDot,
        Dash::LongDashDot,
        Dash::LongDashDotDot,
        Dash::SystemDash,
        Dash::SystemDot,
        Dash::SystemDashDot,
        Dash::SystemDashDotDot,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Dot => "dot",
            Self::Dash => "dash",
            Self::LongDash => "lgDash",
            Self::DashDot => "dashDot",
            Self::LongDashDot => "lgDashDot",
            Self::LongDashDotDot => "lgDashDotDot",
            Self::SystemDash => "sysDash",
            Self::SystemDot => "sysDot",
            Self::SystemDashDot => "sysDashDot",
            Self::SystemDashDotDot => "sysDashDotDot",
        }
    }
}

/// An image that fills a shape: bars, areas, slices, bubbles and the chart,
/// plot area and 3-D walls.
///
/// The image bytes are a package part of your own (`/xl/media/image1.png`).
/// Point the chart part at it with an entry of type
/// [`IMAGE_RELATIONSHIP_TYPE`](crate::IMAGE_RELATIONSHIP_TYPE) in the chart's
/// `_rels` part; [`chart_relationships`](crate::chart_relationships) writes
/// that part.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct PictureFill {
    /// The relationship on the chart part that reaches the image, e.g. `"rId1"`.
    pub relationship_id: String,
    /// Repeat the image at its own size instead of stretching it to fit.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tile: bool,
}

impl PictureFill {
    /// An image stretched to fill the shape, reached by `relationship_id`.
    pub fn new(relationship_id: impl Into<String>) -> Self {
        Self {
            relationship_id: relationship_id.into(),
            tile: false,
        }
    }

    /// Repeats the image instead of stretching it.
    #[must_use]
    pub fn tiled(mut self) -> Self {
        self.tile = true;
        self
    }
}

/// One colour stop of a [`Gradient`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct GradientStop {
    /// Where the stop sits along the gradient, as a percentage from 0 to 100.
    pub position: u8,
    /// Its colour, six hex digits without `#`.
    pub color: String,
}

/// A linear gradient: two to ten colour stops blended along an angle.
///
/// `angle` runs clockwise from left-to-right: 0 blends left to right, 90 top
/// to bottom. Checked at render time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Gradient {
    /// Direction in degrees, 0 to 359.
    pub angle: u16,
    /// The stops, two to ten, positions not decreasing.
    pub stops: Vec<GradientStop>,
    /// How the stops spread; the angle applies to a linear one only.
    #[cfg_attr(feature = "serde", serde(default))]
    pub shape: GradientShape,
    /// The corner a non-linear gradient spreads from; `None` is the centre.
    #[cfg_attr(feature = "serde", serde(default))]
    pub corner: Option<Corner>,
}

impl Gradient {
    /// A gradient at `angle` degrees through `stops`, each a position
    /// (percent) and a colour: `Gradient::linear(90, [(0, "8E0DD1"), (100, "FFFFFF")])`.
    pub fn linear<C: Into<String>>(angle: u16, stops: impl IntoIterator<Item = (u8, C)>) -> Self {
        Self {
            angle,
            stops: stops
                .into_iter()
                .map(|(position, color)| GradientStop {
                    position,
                    color: color.into(),
                })
                .collect(),
            shape: GradientShape::Linear,
            corner: None,
        }
    }

    /// A gradient of `stops` spreading from a `corner` in a `shape`.
    pub fn from_corner<C: Into<String>>(
        shape: GradientShape,
        corner: Corner,
        stops: impl IntoIterator<Item = (u8, C)>,
    ) -> Self {
        Self {
            shape,
            corner: Some(corner),
            ..Self::linear(0, stops)
        }
    }

    /// A gradient of `stops` spreading from the centre in a `shape`:
    /// radial, rectangular or along the outline.
    pub fn from_center<C: Into<String>>(
        shape: GradientShape,
        stops: impl IntoIterator<Item = (u8, C)>,
    ) -> Self {
        Self {
            shape,
            ..Self::linear(0, stops)
        }
    }
}

/// A line drawn as one or several parallel strokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Compound {
    /// One stroke.
    Single,
    /// Two equal strokes.
    Double,
    /// A thick stroke then a thin one.
    ThickThin,
    /// A thin stroke then a thick one.
    ThinThick,
    /// Three strokes, the middle one thick.
    Triple,
}

impl Compound {
    /// Every value.
    pub const ALL: [Compound; 5] = [
        Compound::Single,
        Compound::Double,
        Compound::ThickThin,
        Compound::ThinThick,
        Compound::Triple,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Single => "sng",
            Self::Double => "dbl",
            Self::ThickThin => "thickThin",
            Self::ThinThick => "thinThick",
            Self::Triple => "tri",
        }
    }
}

/// An arrowhead at one end of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Arrow {
    /// A filled triangle.
    Triangle,
    /// An open arrow.
    Open,
    /// A stealth arrow.
    Stealth,
    /// A diamond.
    Diamond,
    /// An oval.
    Oval,
}

impl Arrow {
    /// Every value.
    pub const ALL: [Arrow; 5] = [
        Arrow::Triangle,
        Arrow::Open,
        Arrow::Stealth,
        Arrow::Diamond,
        Arrow::Oval,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Triangle => "triangle",
            Self::Open => "arrow",
            Self::Stealth => "stealth",
            Self::Diamond => "diamond",
            Self::Oval => "oval",
        }
    }
}

/// The shape of a bevel on a shape's top edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Bevel {
    /// Circle.
    Circle,
    /// Relaxed inset.
    RelaxedInset,
    /// Cross.
    Cross,
    /// Cool slant.
    CoolSlant,
    /// Angle.
    Angle,
    /// Soft round.
    SoftRound,
    /// Convex.
    Convex,
    /// Slope.
    Slope,
    /// Divot.
    Divot,
    /// Riblet.
    Riblet,
    /// Hard edge.
    HardEdge,
    /// Art deco.
    ArtDeco,
}

impl Bevel {
    /// Every value.
    pub const ALL: [Bevel; 12] = [
        Bevel::Circle,
        Bevel::RelaxedInset,
        Bevel::Cross,
        Bevel::CoolSlant,
        Bevel::Angle,
        Bevel::SoftRound,
        Bevel::Convex,
        Bevel::Slope,
        Bevel::Divot,
        Bevel::Riblet,
        Bevel::HardEdge,
        Bevel::ArtDeco,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::RelaxedInset => "relaxedInset",
            Self::Cross => "cross",
            Self::CoolSlant => "coolSlant",
            Self::Angle => "angle",
            Self::SoftRound => "softRound",
            Self::Convex => "convex",
            Self::Slope => "slope",
            Self::Divot => "divot",
            Self::Riblet => "riblet",
            Self::HardEdge => "hardEdge",
            Self::ArtDeco => "artDeco",
        }
    }
}

/// How a gradient spreads its stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum GradientShape {
    /// Along a line at the gradient's angle.
    #[default]
    Linear,
    /// In circles out from the centre.
    Radial,
    /// In rectangles out from the centre.
    Rectangular,
    /// Following the shape's outline, out from the centre.
    Path,
}

impl GradientShape {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Linear => "",
            Self::Radial => "circle",
            Self::Rectangular => "rect",
            Self::Path => "shape",
        }
    }
}

/// A preset two-colour pattern, as in Excel's "Pattern fill" list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum PatternKind {
    /// 5% foreground.
    Percent5,
    /// 10% foreground.
    Percent10,
    /// 20% foreground.
    Percent20,
    /// 25% foreground.
    Percent25,
    /// 30% foreground.
    Percent30,
    /// 40% foreground.
    Percent40,
    /// 50% foreground.
    Percent50,
    /// 60% foreground.
    Percent60,
    /// 70% foreground.
    Percent70,
    /// 75% foreground.
    Percent75,
    /// 80% foreground.
    Percent80,
    /// 90% foreground.
    Percent90,
    /// Horizontal stripes.
    Horizontal,
    /// Vertical stripes.
    Vertical,
    /// Light horizontal stripes.
    LightHorizontal,
    /// Light vertical stripes.
    LightVertical,
    /// Dark horizontal stripes.
    DarkHorizontal,
    /// Dark vertical stripes.
    DarkVertical,
    /// Narrow horizontal stripes.
    NarrowHorizontal,
    /// Narrow vertical stripes.
    NarrowVertical,
    /// Dashed horizontal stripes.
    DashedHorizontal,
    /// Dashed vertical stripes.
    DashedVertical,
    /// Horizontal and vertical lines.
    Cross,
    /// Downward diagonal stripes.
    DownwardDiagonal,
    /// Upward diagonal stripes.
    UpwardDiagonal,
    /// Light downward diagonal stripes.
    LightDownwardDiagonal,
    /// Light upward diagonal stripes.
    LightUpwardDiagonal,
    /// Dark downward diagonal stripes.
    DarkDownwardDiagonal,
    /// Dark upward diagonal stripes.
    DarkUpwardDiagonal,
    /// Wide downward diagonal stripes.
    WideDownwardDiagonal,
    /// Wide upward diagonal stripes.
    WideUpwardDiagonal,
    /// Dashed downward diagonal stripes.
    DashedDownwardDiagonal,
    /// Dashed upward diagonal stripes.
    DashedUpwardDiagonal,
    /// Crossing diagonal lines.
    DiagonalCross,
    /// Small checkerboard.
    SmallCheckerBoard,
    /// Large checkerboard.
    LargeCheckerBoard,
    /// Small grid.
    SmallGrid,
    /// Large grid.
    LargeGrid,
    /// Dotted grid.
    DottedGrid,
    /// Small confetti.
    SmallConfetti,
    /// Large confetti.
    LargeConfetti,
    /// Horizontal bricks.
    HorizontalBrick,
    /// Diagonal bricks.
    DiagonalBrick,
    /// Solid diamonds.
    SolidDiamond,
    /// Open diamonds.
    OpenDiamond,
    /// Dotted diamonds.
    DottedDiamond,
    /// Plaid.
    Plaid,
    /// Spheres.
    Sphere,
    /// Weave.
    Weave,
    /// Divots.
    Divot,
    /// Shingles.
    Shingle,
    /// Waves.
    Wave,
    /// Trellis.
    Trellis,
    /// Zigzag.
    ZigZag,
}

impl PatternKind {
    /// Every pattern.
    pub const ALL: [PatternKind; 54] = [
        PatternKind::Percent5,
        PatternKind::Percent10,
        PatternKind::Percent20,
        PatternKind::Percent25,
        PatternKind::Percent30,
        PatternKind::Percent40,
        PatternKind::Percent50,
        PatternKind::Percent60,
        PatternKind::Percent70,
        PatternKind::Percent75,
        PatternKind::Percent80,
        PatternKind::Percent90,
        PatternKind::Horizontal,
        PatternKind::Vertical,
        PatternKind::LightHorizontal,
        PatternKind::LightVertical,
        PatternKind::DarkHorizontal,
        PatternKind::DarkVertical,
        PatternKind::NarrowHorizontal,
        PatternKind::NarrowVertical,
        PatternKind::DashedHorizontal,
        PatternKind::DashedVertical,
        PatternKind::Cross,
        PatternKind::DownwardDiagonal,
        PatternKind::UpwardDiagonal,
        PatternKind::LightDownwardDiagonal,
        PatternKind::LightUpwardDiagonal,
        PatternKind::DarkDownwardDiagonal,
        PatternKind::DarkUpwardDiagonal,
        PatternKind::WideDownwardDiagonal,
        PatternKind::WideUpwardDiagonal,
        PatternKind::DashedDownwardDiagonal,
        PatternKind::DashedUpwardDiagonal,
        PatternKind::DiagonalCross,
        PatternKind::SmallCheckerBoard,
        PatternKind::LargeCheckerBoard,
        PatternKind::SmallGrid,
        PatternKind::LargeGrid,
        PatternKind::DottedGrid,
        PatternKind::SmallConfetti,
        PatternKind::LargeConfetti,
        PatternKind::HorizontalBrick,
        PatternKind::DiagonalBrick,
        PatternKind::SolidDiamond,
        PatternKind::OpenDiamond,
        PatternKind::DottedDiamond,
        PatternKind::Plaid,
        PatternKind::Sphere,
        PatternKind::Weave,
        PatternKind::Divot,
        PatternKind::Shingle,
        PatternKind::Wave,
        PatternKind::Trellis,
        PatternKind::ZigZag,
    ];

    pub(crate) fn code(self) -> &'static str {
        match self {
            PatternKind::Percent5 => "pct5",
            PatternKind::Percent10 => "pct10",
            PatternKind::Percent20 => "pct20",
            PatternKind::Percent25 => "pct25",
            PatternKind::Percent30 => "pct30",
            PatternKind::Percent40 => "pct40",
            PatternKind::Percent50 => "pct50",
            PatternKind::Percent60 => "pct60",
            PatternKind::Percent70 => "pct70",
            PatternKind::Percent75 => "pct75",
            PatternKind::Percent80 => "pct80",
            PatternKind::Percent90 => "pct90",
            PatternKind::Horizontal => "horz",
            PatternKind::Vertical => "vert",
            PatternKind::LightHorizontal => "ltHorz",
            PatternKind::LightVertical => "ltVert",
            PatternKind::DarkHorizontal => "dkHorz",
            PatternKind::DarkVertical => "dkVert",
            PatternKind::NarrowHorizontal => "narHorz",
            PatternKind::NarrowVertical => "narVert",
            PatternKind::DashedHorizontal => "dashHorz",
            PatternKind::DashedVertical => "dashVert",
            PatternKind::Cross => "cross",
            PatternKind::DownwardDiagonal => "dnDiag",
            PatternKind::UpwardDiagonal => "upDiag",
            PatternKind::LightDownwardDiagonal => "ltDnDiag",
            PatternKind::LightUpwardDiagonal => "ltUpDiag",
            PatternKind::DarkDownwardDiagonal => "dkDnDiag",
            PatternKind::DarkUpwardDiagonal => "dkUpDiag",
            PatternKind::WideDownwardDiagonal => "wdDnDiag",
            PatternKind::WideUpwardDiagonal => "wdUpDiag",
            PatternKind::DashedDownwardDiagonal => "dashDnDiag",
            PatternKind::DashedUpwardDiagonal => "dashUpDiag",
            PatternKind::DiagonalCross => "diagCross",
            PatternKind::SmallCheckerBoard => "smCheck",
            PatternKind::LargeCheckerBoard => "lgCheck",
            PatternKind::SmallGrid => "smGrid",
            PatternKind::LargeGrid => "lgGrid",
            PatternKind::DottedGrid => "dotGrid",
            PatternKind::SmallConfetti => "smConfetti",
            PatternKind::LargeConfetti => "lgConfetti",
            PatternKind::HorizontalBrick => "horzBrick",
            PatternKind::DiagonalBrick => "diagBrick",
            PatternKind::SolidDiamond => "solidDmnd",
            PatternKind::OpenDiamond => "openDmnd",
            PatternKind::DottedDiamond => "dotDmnd",
            PatternKind::Plaid => "plaid",
            PatternKind::Sphere => "sphere",
            PatternKind::Weave => "weave",
            PatternKind::Divot => "divot",
            PatternKind::Shingle => "shingle",
            PatternKind::Wave => "wave",
            PatternKind::Trellis => "trellis",
            PatternKind::ZigZag => "zigZag",
        }
    }
}

/// A pattern fill: a preset pattern drawn in a foreground colour over a
/// background colour.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Pattern {
    /// Which pattern.
    pub kind: PatternKind,
    /// Colour of the pattern's marks, six hex digits without `#`.
    pub foreground: String,
    /// Colour behind them, six hex digits without `#`.
    pub background: String,
}

impl Pattern {
    /// A `kind` pattern in `foreground` over `background`.
    pub fn new(
        kind: PatternKind,
        foreground: impl Into<String>,
        background: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            foreground: foreground.into(),
            background: background.into(),
        }
    }
}

/// The fill and border of the chart area or the plot area.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct AreaStyle {
    /// Background. `None` leaves Excel's default.
    pub fill: Option<Paint>,
    /// Border colour. `None` leaves Excel's default.
    pub border: Option<Paint>,
    /// Border width in points.
    pub border_width_pt: Option<f64>,
    /// Dash pattern of the border.
    pub border_dash: Option<Dash>,
    /// Shadow, glow and soft edge.
    pub effects: Option<Effects>,
}

impl AreaStyle {
    /// An empty style; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fills with a solid colour, six hex digits without `#`.
    #[must_use]
    pub fn fill(mut self, rgb: impl Into<String>) -> Self {
        self.fill = Some(Paint::Color(rgb.into()));
        self
    }

    /// Fills with a gradient, a pattern or any other [`Paint`].
    #[must_use]
    pub fn fill_paint(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }

    /// Draws the border with a gradient, a pattern or any other [`Paint`].
    #[must_use]
    pub fn border_paint(mut self, paint: Paint) -> Self {
        self.border = Some(paint);
        self
    }

    /// Removes the fill, leaving the area transparent.
    #[must_use]
    pub fn no_fill(mut self) -> Self {
        self.fill = Some(Paint::None);
        self
    }

    /// Draws a solid border, six hex digits without `#`.
    #[must_use]
    pub fn border(mut self, rgb: impl Into<String>) -> Self {
        self.border = Some(Paint::Color(rgb.into()));
        self
    }

    /// Removes the border.
    #[must_use]
    pub fn no_border(mut self) -> Self {
        self.border = Some(Paint::None);
        self
    }

    /// Sets the border width in points.
    #[must_use]
    pub fn border_width(mut self, points: f64) -> Self {
        self.border_width_pt = Some(points);
        self
    }

    /// Draws the border dashed or dotted.
    #[must_use]
    pub fn border_dash(mut self, dash: Dash) -> Self {
        self.border_dash = Some(dash);
        self
    }

    /// Adds shadow, glow or soft-edge effects.
    #[must_use]
    pub fn effects(mut self, effects: Effects) -> Self {
        self.effects = Some(effects);
        self
    }
}

/// A drop shadow, or an inner one.
///
/// Defaults match Excel's "Offset: Bottom Right" preset: 4 pt blur, 3 pt away
/// at 45 degrees, black at 40 % opacity.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Shadow {
    /// Colour, six hex digits without `#`.
    pub color: String,
    /// How far the edge is blurred, in points (0 to 100).
    pub blur_pt: f64,
    /// How far the shadow sits from the shape, in points (0 to 200).
    pub distance_pt: f64,
    /// Direction in degrees clockwise from the right (0 to 359).
    pub angle: u16,
    /// Opacity as a percentage (0 to 100).
    pub opacity: u8,
    /// Horizontal and vertical size in percent (-200 to 200; negative
    /// flips). Outer shadows only.
    #[cfg_attr(feature = "serde", serde(default))]
    pub scale: Option<(i16, i16)>,
    /// Horizontal and vertical skew in degrees (-89 to 89). Outer shadows
    /// only.
    #[cfg_attr(feature = "serde", serde(default))]
    pub skew: Option<(f64, f64)>,
    /// The point scaling and skew are anchored to. Outer shadows only.
    #[cfg_attr(feature = "serde", serde(default))]
    pub align: Option<ShadowAlign>,
}

impl Shadow {
    /// A shadow of `rgb` with the default blur, distance, angle and opacity.
    pub fn new(rgb: impl Into<String>) -> Self {
        Self {
            color: rgb.into(),
            blur_pt: 4.0,
            distance_pt: 3.0,
            angle: 45,
            opacity: 40,
            scale: None,
            skew: None,
            align: None,
        }
    }

    /// Sizes the shadow: `x` and `y` percent of the shape, negative to flip.
    /// Outer shadows only.
    #[must_use]
    pub fn scale(mut self, x: i16, y: i16) -> Self {
        self.scale = Some((x, y));
        self
    }

    /// Skews the shadow by `x` and `y` degrees. Outer shadows only.
    #[must_use]
    pub fn skew(mut self, x: f64, y: f64) -> Self {
        self.skew = Some((x, y));
        self
    }

    /// Anchors the shadow's scale and skew. Outer shadows only.
    #[must_use]
    pub fn align(mut self, align: ShadowAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// Sets the blur in points.
    #[must_use]
    pub fn blur(mut self, points: f64) -> Self {
        self.blur_pt = points;
        self
    }

    /// Sets the distance from the shape in points.
    #[must_use]
    pub fn distance(mut self, points: f64) -> Self {
        self.distance_pt = points;
        self
    }

    /// Sets the direction in degrees clockwise from the right.
    #[must_use]
    pub fn angle(mut self, degrees: u16) -> Self {
        self.angle = degrees;
        self
    }

    /// Sets the opacity as a percentage.
    #[must_use]
    pub fn opacity(mut self, percent: u8) -> Self {
        self.opacity = percent;
        self
    }
}

/// A soft coloured halo around a shape.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct Glow {
    /// Colour, six hex digits without `#`.
    pub color: String,
    /// How far the glow reaches, in points (0 to 150).
    pub radius_pt: f64,
    /// Opacity as a percentage (0 to 100).
    pub opacity: u8,
}

impl Glow {
    /// A glow of `rgb` reaching `radius_pt` points, at 40 % opacity.
    pub fn new(rgb: impl Into<String>, radius_pt: f64) -> Self {
        Self {
            color: rgb.into(),
            radius_pt,
            opacity: 40,
        }
    }

    /// Sets the opacity as a percentage.
    #[must_use]
    pub fn opacity(mut self, percent: u8) -> Self {
        self.opacity = percent;
        self
    }
}

/// Shadow, glow and soft-edge effects on a shape. Each is optional.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct Effects {
    /// A shadow behind the shape.
    pub shadow: Option<Shadow>,
    /// A shadow inside the shape's edge.
    pub inner_shadow: Option<Shadow>,
    /// A halo around the shape.
    pub glow: Option<Glow>,
    /// Fades the shape's edge over this many points (0 to 100).
    pub soft_edge_pt: Option<f64>,
    /// A 6-point bevel of this shape on the top edge.
    pub bevel: Option<Bevel>,
    /// One of DrawingML's 20 preset shadows (1-20), with the shadow's
    /// colour, opacity, distance and angle.
    pub preset_shadow: Option<(u8, Shadow)>,
}

impl Effects {
    /// No effects; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a shadow behind the shape.
    #[must_use]
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Adds a shadow inside the shape's edge.
    #[must_use]
    pub fn inner_shadow(mut self, shadow: Shadow) -> Self {
        self.inner_shadow = Some(shadow);
        self
    }

    /// Adds a glow.
    #[must_use]
    pub fn glow(mut self, glow: Glow) -> Self {
        self.glow = Some(glow);
        self
    }

    /// Fades the edge over `points`.
    #[must_use]
    pub fn soft_edge(mut self, points: f64) -> Self {
        self.soft_edge_pt = Some(points);
        self
    }

    /// Bevels the top edge.
    #[must_use]
    pub fn bevel(mut self, bevel: Bevel) -> Self {
        self.bevel = Some(bevel);
        self
    }

    /// Adds preset shadow `preset` (1-20), coloured and placed by `shadow`;
    /// its blur, scale, skew and alignment do not apply.
    #[must_use]
    pub fn preset_shadow(mut self, preset: u8, shadow: Shadow) -> Self {
        self.preset_shadow = Some((preset, shadow));
        self
    }
}

/// Where an axis puts its tick labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum TickLabels {
    /// Beside the axis. Excel's default.
    #[default]
    NextTo,
    /// At the low end of the crossing axis — keeps labels clear of negative
    /// bars.
    Low,
    /// At the high end of the crossing axis.
    High,
    /// No tick labels.
    None,
}

impl TickLabels {
    pub(crate) fn code(self) -> &'static str {
        match self {
            TickLabels::NextTo => "nextTo",
            TickLabels::Low => "low",
            TickLabels::High => "high",
            TickLabels::None => "none",
        }
    }
}

/// How an axis draws its tick marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum TickMark {
    /// Outside the plot.
    Out,
    /// Inside the plot.
    In,
    /// Straddling the axis.
    Cross,
    /// No marks.
    None,
}

impl TickMark {
    pub(crate) fn code(self) -> &'static str {
        match self {
            TickMark::Out => "out",
            TickMark::In => "in",
            TickMark::Cross => "cross",
            TickMark::None => "none",
        }
    }
}

/// How a value axis scales its tick labels, e.g. showing 1,500,000 as 1.5 with
/// a "Millions" caption. The data is unchanged; only the labels are.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum DisplayUnit {
    /// Divide by 100.
    Hundreds,
    /// Divide by 1,000.
    Thousands,
    /// Divide by 10,000.
    TenThousands,
    /// Divide by 100,000.
    HundredThousands,
    /// Divide by 1,000,000.
    Millions,
    /// Divide by 10,000,000.
    TenMillions,
    /// Divide by 100,000,000.
    HundredMillions,
    /// Divide by 1,000,000,000.
    Billions,
    /// Divide by 1,000,000,000,000.
    Trillions,
    /// Divide by this number, which must be finite and above 0.
    Custom(f64),
}

impl DisplayUnit {
    pub(crate) fn element(self) -> String {
        let built_in = |name: &str| format!(r#"<c:builtInUnit val="{name}"/>"#);
        match self {
            DisplayUnit::Hundreds => built_in("hundreds"),
            DisplayUnit::Thousands => built_in("thousands"),
            DisplayUnit::TenThousands => built_in("tenThousands"),
            DisplayUnit::HundredThousands => built_in("hundredThousands"),
            DisplayUnit::Millions => built_in("millions"),
            DisplayUnit::TenMillions => built_in("tenMillions"),
            DisplayUnit::HundredMillions => built_in("hundredMillions"),
            DisplayUnit::Billions => built_in("billions"),
            DisplayUnit::Trillions => built_in("trillions"),
            DisplayUnit::Custom(divisor) => format!(r#"<c:custUnit val="{divisor}"/>"#),
        }
    }
}

/// The unit of a date axis, finest first: `Days < Months < Years`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum DateUnit {
    /// One tick per day.
    Days,
    /// One tick per month.
    Months,
    /// One tick per year.
    Years,
}

impl DateUnit {
    pub(crate) fn code(self) -> &'static str {
        match self {
            DateUnit::Days => "days",
            DateUnit::Months => "months",
            DateUnit::Years => "years",
        }
    }
}

/// Where the legend goes, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum LegendPosition {
    /// To the right of the plot. Excel's own default.
    #[default]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct Axis {
    /// An axis title, if any.
    pub title: Option<String>,
    /// An axis title of several differently formatted runs. Use this or
    /// `title`, not both.
    pub title_runs: Option<Vec<TitleRun>>,
    /// Whether to draw major gridlines across the plot from this axis.
    pub major_gridlines: bool,
    /// Whether to draw minor gridlines.
    pub minor_gridlines: bool,
    /// Whether the axis is drawn. A deleted axis still exists and still scales
    /// the plot — it is just not shown.
    pub visible: bool,
    /// An Excel number format for the tick labels, e.g. `"0.0%"`.
    pub number_format: Option<String>,
    /// Fixed scale minimum. `None` lets Excel choose. Value axes and the x axis
    /// of a scatter or bubble chart only; not written for category axes.
    pub min: Option<f64>,
    /// Fixed scale maximum. `None` lets Excel choose. Same axes as `min`.
    pub max: Option<f64>,
    /// Distance between major ticks. Value axes and the x axis of a scatter or
    /// bubble chart only. `None` lets Excel choose.
    pub major_unit: Option<f64>,
    /// Distance between minor ticks. Same axes as `major_unit`.
    pub minor_unit: Option<f64>,
    /// Logarithmic scale with this base (2-1000). Same axes as `min`.
    pub log_base: Option<u16>,
    /// Plot from high to low instead of low to high.
    pub reversed: bool,
    /// Where the tick labels go.
    pub tick_labels: TickLabels,
    /// Major tick mark style. `None` leaves Excel's default.
    pub major_tick: Option<TickMark>,
    /// Minor tick mark style. `None` leaves Excel's default.
    pub minor_tick: Option<TickMark>,
    /// Tick label rotation in degrees, -90 to 90.
    pub label_rotation: Option<i16>,
    /// Font for the axis title.
    pub title_style: Option<TextStyle>,
    /// Manual position of the axis title. Needs `title`.
    pub title_position: Option<Position>,
    /// Colour of the axis line. `None` leaves Excel's default;
    /// [`Paint::None`] hides the line.
    pub line: Option<Paint>,
    /// Width of the axis line in points.
    pub line_width_pt: Option<f64>,
    /// Dash pattern of the axis line.
    pub line_dash: Option<Dash>,
    /// Colour, width and dash of the major gridlines, from the style's
    /// border. Needs `major_gridlines`.
    pub gridline_style: Option<AreaStyle>,
    /// Font for the tick labels.
    pub label_style: Option<TextStyle>,
    /// Cross the other axis at its maximum instead of at zero. On a horizontal
    /// bar chart's value axis this moves the axis to the other side.
    pub crosses_max: bool,
    /// Cross the other axis at this value, in the *other* axis's units. Set on
    /// a category axis, `crosses_at(50.0)` puts it where the value axis reads
    /// 50; set on a value axis it is a position on the category axis (the
    /// first category is 1). Excludes `crosses_max`. On a secondary value
    /// axis it replaces the default of crossing at the far end.
    pub crosses_at: Option<f64>,
    /// Scale the tick labels, e.g. to thousands. Value axes, and the x axis of
    /// a scatter or bubble chart.
    pub display_unit: Option<DisplayUnit>,
    /// Print the Excel caption for `display_unit` ("Thousands") on the axis.
    /// Needs `display_unit`.
    pub display_unit_label: bool,
    /// Replaces the caption's text ("Thousands") with your own. Needs
    /// `display_unit`, and must not be empty. Shows the caption.
    pub display_unit_caption: Option<String>,
    /// Draw a date axis with this unit. Category axes of bar, column, line and
    /// area charts only.
    pub date_unit: Option<DateUnit>,
    /// What `major_unit` counts on a date axis: `major_unit = 3` with
    /// `Months` puts a labelled tick every three months. Date axes only, and
    /// never finer than the axis's own unit.
    pub major_time_unit: Option<DateUnit>,
    /// What `minor_unit` counts on a date axis. Same rules as
    /// `major_time_unit`.
    pub minor_time_unit: Option<DateUnit>,
}

impl Default for Axis {
    /// Visible, untitled, no gridlines, scaled automatically.
    fn default() -> Self {
        Self {
            title: None,
            title_runs: None,
            major_gridlines: false,
            minor_gridlines: false,
            visible: true,
            number_format: None,
            min: None,
            max: None,
            major_unit: None,
            minor_unit: None,
            log_base: None,
            reversed: false,
            tick_labels: TickLabels::NextTo,
            major_tick: None,
            minor_tick: None,
            label_rotation: None,
            title_style: None,
            title_position: None,
            line: None,
            line_width_pt: None,
            line_dash: None,
            gridline_style: None,
            label_style: None,
            crosses_max: false,
            crosses_at: None,
            display_unit: None,
            display_unit_label: false,
            display_unit_caption: None,
            date_unit: None,
            major_time_unit: None,
            minor_time_unit: None,
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

    /// Turns minor gridlines on or off.
    #[must_use]
    pub fn minor_gridlines(mut self, on: bool) -> Self {
        self.minor_gridlines = on;
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

    /// Fixes the distance between minor ticks.
    #[must_use]
    pub fn minor_unit(mut self, value: f64) -> Self {
        self.minor_unit = Some(value);
        self
    }

    /// Uses a logarithmic scale of the given base, 2-1000. Values at or below
    /// zero cannot be plotted on one.
    #[must_use]
    pub fn log(mut self, base: u16) -> Self {
        self.log_base = Some(base);
        self
    }

    /// Plots from high to low.
    #[must_use]
    pub fn reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }

    /// Places the tick labels.
    #[must_use]
    pub fn tick_labels(mut self, position: TickLabels) -> Self {
        self.tick_labels = position;
        self
    }

    /// Sets the major tick mark style.
    #[must_use]
    pub fn major_tick(mut self, mark: TickMark) -> Self {
        self.major_tick = Some(mark);
        self
    }

    /// Sets the minor tick mark style.
    #[must_use]
    pub fn minor_tick(mut self, mark: TickMark) -> Self {
        self.minor_tick = Some(mark);
        self
    }

    /// Rotates the tick labels, -90 to 90 degrees.
    #[must_use]
    pub fn label_rotation(mut self, degrees: i16) -> Self {
        self.label_rotation = Some(degrees);
        self
    }

    /// Titles the axis with several runs, each with its own formatting. Use
    /// this or [`Axis::title`], not both.
    #[must_use]
    pub fn title_runs(mut self, runs: impl IntoIterator<Item = TitleRun>) -> Self {
        self.title_runs = Some(runs.into_iter().collect());
        self
    }

    /// Styles the axis title.
    #[must_use]
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// Draws the axis line in a solid colour, six hex digits without `#`.
    #[must_use]
    pub fn line(mut self, rgb: impl Into<String>) -> Self {
        self.line = Some(Paint::Color(rgb.into()));
        self
    }

    /// Draws the axis line with a gradient, a pattern or any other [`Paint`].
    #[must_use]
    pub fn line_paint(mut self, paint: Paint) -> Self {
        self.line = Some(paint);
        self
    }

    /// Hides the axis line, leaving tick labels and gridlines.
    #[must_use]
    pub fn no_line(mut self) -> Self {
        self.line = Some(Paint::None);
        self
    }

    /// Sets the axis line width in points.
    #[must_use]
    pub fn line_width(mut self, points: f64) -> Self {
        self.line_width_pt = Some(points);
        self
    }

    /// Draws the axis line dashed or dotted.
    #[must_use]
    pub fn line_dash(mut self, dash: Dash) -> Self {
        self.line_dash = Some(dash);
        self
    }

    /// Formats the major gridlines with the style's border colour, width
    /// and dash, and turns them on.
    #[must_use]
    pub fn gridline_style(mut self, style: AreaStyle) -> Self {
        self.major_gridlines = true;
        self.gridline_style = Some(style);
        self
    }

    /// Places the axis title by hand; see [`Position`]. The axis needs a
    /// title: combining this with no title is refused at render time.
    #[must_use]
    pub fn title_position(mut self, position: Position) -> Self {
        self.title_position = Some(position);
        self
    }

    /// Styles the tick labels.
    #[must_use]
    pub fn label_style(mut self, style: TextStyle) -> Self {
        self.label_style = Some(style);
        self
    }

    /// Crosses the other axis at its maximum.
    #[must_use]
    pub fn crosses_max(mut self, on: bool) -> Self {
        self.crosses_max = on;
        self
    }

    /// Scales the tick labels by `unit`, e.g. [`DisplayUnit::Thousands`].
    #[must_use]
    pub fn display_units(mut self, unit: DisplayUnit) -> Self {
        self.display_unit = Some(unit);
        self
    }

    /// Shows or hides the caption Excel prints beside a scaled axis.
    #[must_use]
    pub fn display_units_label(mut self, show: bool) -> Self {
        self.display_unit_label = show;
        self
    }

    /// Words the caption beside a scaled axis yourself, e.g. "USD millions",
    /// and shows it.
    #[must_use]
    pub fn display_units_caption(mut self, text: impl Into<String>) -> Self {
        self.display_unit_caption = Some(text.into());
        self.display_unit_label = true;
        self
    }

    /// Crosses the other axis at `value`, in that axis's units; see
    /// [`Axis::crosses_at`](struct.Axis.html#structfield.crosses_at).
    #[must_use]
    pub fn crosses_at(mut self, value: f64) -> Self {
        self.crosses_at = Some(value);
        self
    }

    /// Draws a date axis. Meaningful on a category axis only.
    #[must_use]
    pub fn dates(mut self, unit: DateUnit) -> Self {
        self.date_unit = Some(unit);
        self
    }

    /// Labels a date axis every `count` `unit`s, e.g. `date_major(3,
    /// DateUnit::Months)`. Sets `major_unit` and `major_time_unit` together.
    /// Checked at render time: a whole number of at least 1, on a date axis,
    /// no finer than the axis's own unit.
    #[must_use]
    pub fn date_major(mut self, count: u16, unit: DateUnit) -> Self {
        self.major_unit = Some(f64::from(count));
        self.major_time_unit = Some(unit);
        self
    }

    /// Puts minor ticks on a date axis every `count` `unit`s. Same rules as
    /// [`Axis::date_major`].
    #[must_use]
    pub fn date_minor(mut self, count: u16, unit: DateUnit) -> Self {
        self.minor_unit = Some(f64::from(count));
        self.minor_time_unit = Some(unit);
        self
    }
}

/// Where a data label sits relative to its point or bar.
///
/// Which positions are legal depends on the chart kind; an illegal one is
/// refused at render time with [`ChartError::InvalidDataLabelPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
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
    /// Font for the labels.
    pub style: Option<TextStyle>,
    /// Fill, border and effects of the label boxes.
    pub area: Option<AreaStyle>,
    /// Draw leader lines from labels that sit away from their slice. Pie and
    /// doughnut only.
    pub leader_lines: bool,
    /// Show the text of each series' label range. Every series then needs
    /// [`Series::with_label_range`].
    pub cells: bool,
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

    /// Fills, borders or adds effects to the label boxes.
    #[must_use]
    pub fn area(mut self, style: AreaStyle) -> Self {
        self.area = Some(style);
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

    /// Also show the text of each series' label range ("Value From Cells").
    #[must_use]
    pub fn with_cells(mut self) -> Self {
        self.cells = true;
        self
    }

    /// Also draw leader lines from outlying labels to their slices (pie and
    /// doughnut).
    #[must_use]
    pub fn with_leader_lines(mut self) -> Self {
        self.leader_lines = true;
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

    /// Styles the label text.
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }
}

/// An override for one point's data label, set with
/// [`Series::with_point_label`].
///
/// Whatever it leaves unset (what to show, number format, position, font) is
/// inherited from the chart-wide [`DataLabels`]. If the chart has none, the
/// point shows its value. A series with any override writes its own label
/// settings, so the other points of that series keep the chart-wide look.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct PointLabel {
    /// Remove this point's label. Cannot be combined with `text`.
    pub hidden: bool,
    /// Replace the label with this literal text.
    pub text: Option<String>,
    /// Replace the label with a mix of literal text and live fields, such as
    /// `[CATEGORY NAME]: [VALUE]`. Cannot be combined with `text`.
    pub parts: Vec<LabelPart>,
    /// Where to put this label. Checked against the chart kind like
    /// [`DataLabels::position`].
    pub position: Option<DataLabelPosition>,
    /// Font for this label.
    pub style: Option<TextStyle>,
}

impl PointLabel {
    /// A label that is not drawn.
    pub fn hidden() -> Self {
        Self {
            hidden: true,
            ..Self::default()
        }
    }

    /// A label showing `text` instead of the value.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// A label built from literal text and live fields, in order. Excel keeps
    /// the fields up to date as the data changes.
    pub fn parts(parts: impl IntoIterator<Item = LabelPart>) -> Self {
        Self {
            parts: parts.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Places this label.
    #[must_use]
    pub fn at(mut self, position: DataLabelPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Styles this label's text.
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Some(style);
        self
    }
}

/// The shape of a 3-D bar or column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum BarShape {
    /// A box (Excel's default).
    Box,
    /// A cylinder.
    Cylinder,
    /// A cone, narrowing to its own value.
    Cone,
    /// A cone, narrowing to the largest value on the chart.
    ConeToMax,
    /// A pyramid, narrowing to its own value.
    Pyramid,
    /// A pyramid, narrowing to the largest value on the chart.
    PyramidToMax,
}

impl BarShape {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Box => "box",
            Self::Cylinder => "cylinder",
            Self::Cone => "cone",
            Self::ConeToMax => "coneToMax",
            Self::Pyramid => "pyramid",
            Self::PyramidToMax => "pyramidToMax",
        }
    }
}

/// How a 3-D chart is viewed, set with [`ChartSpec::view_3d`].
///
/// Whatever is left unset takes Excel's own default for the kind: a bar,
/// column, line or area chart is turned 15 degrees up and 20 around with
/// right-angle axes; a pie is tilted 30 degrees with a perspective of 30.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct View3D {
    /// Tilt up or down, -90 to 90 degrees.
    pub rotation_x: Option<i16>,
    /// Turn around the vertical, 0 to 360 degrees.
    pub rotation_y: Option<u16>,
    /// Perspective, 0 to 240. Needs `right_angle_axes` off.
    pub perspective: Option<u8>,
    /// Keep the axes at right angles. Not on a pie.
    pub right_angle_axes: Option<bool>,
    /// Height as a percentage of the base, 5 to 500.
    pub height_percent: Option<u16>,
    /// Depth as a percentage of the base, 20 to 2000.
    pub depth_percent: Option<u16>,
    /// Gap between series in depth, as a percentage, 0 to 500. Not on a pie.
    pub gap_depth: Option<u16>,
    /// Shape of a bar or column. Bar and column only.
    pub bar_shape: Option<BarShape>,
    /// Put each series of a clustered bar or column in its own row of depth,
    /// with a depth axis. Clustered bar and column only.
    pub depth_axis: bool,
    /// Fill and border of the floor. Not on a pie.
    pub floor: Option<AreaStyle>,
    /// Fill and border of the side wall. Not on a pie.
    pub side_wall: Option<AreaStyle>,
    /// Fill and border of the back wall. Not on a pie.
    pub back_wall: Option<AreaStyle>,
}

impl View3D {
    /// Excel's default view for the kind.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tilts the chart, -90 to 90 degrees.
    #[must_use]
    pub fn rotation_x(mut self, degrees: i16) -> Self {
        self.rotation_x = Some(degrees);
        self
    }

    /// Turns the chart, 0 to 360 degrees.
    #[must_use]
    pub fn rotation_y(mut self, degrees: u16) -> Self {
        self.rotation_y = Some(degrees);
        self
    }

    /// Sets the perspective, 0 to 240, and turns right-angle axes off.
    #[must_use]
    pub fn perspective(mut self, amount: u8) -> Self {
        self.perspective = Some(amount);
        self.right_angle_axes = Some(false);
        self
    }

    /// Keeps the axes at right angles, or not.
    #[must_use]
    pub fn right_angle_axes(mut self, on: bool) -> Self {
        self.right_angle_axes = Some(on);
        self
    }

    /// Sets the height as a percentage of the base, 5 to 500.
    #[must_use]
    pub fn height_percent(mut self, percent: u16) -> Self {
        self.height_percent = Some(percent);
        self
    }

    /// Sets the depth as a percentage of the base, 20 to 2000.
    #[must_use]
    pub fn depth_percent(mut self, percent: u16) -> Self {
        self.depth_percent = Some(percent);
        self
    }

    /// Sets the gap between series in depth, 0 to 500 percent.
    #[must_use]
    pub fn gap_depth(mut self, percent: u16) -> Self {
        self.gap_depth = Some(percent);
        self
    }

    /// Shapes the bars or columns.
    #[must_use]
    pub fn bar_shape(mut self, shape: BarShape) -> Self {
        self.bar_shape = Some(shape);
        self
    }

    /// Gives a clustered bar or column a depth axis, one row per series.
    #[must_use]
    pub fn with_depth_axis(mut self) -> Self {
        self.depth_axis = true;
        self
    }

    /// Styles the floor.
    #[must_use]
    pub fn floor(mut self, style: AreaStyle) -> Self {
        self.floor = Some(style);
        self
    }

    /// Styles the side wall.
    #[must_use]
    pub fn side_wall(mut self, style: AreaStyle) -> Self {
        self.side_wall = Some(style);
        self
    }

    /// Styles the back wall.
    #[must_use]
    pub fn back_wall(mut self, style: AreaStyle) -> Self {
        self.back_wall = Some(style);
        self
    }
}

/// Which points a pie-of-pie or bar-of-pie moves to its second plot.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum OfPieSplit {
    /// Let Excel choose.
    #[default]
    Auto,
    /// The last `n` points (at least 1).
    LastPoints(u32),
    /// Points whose value is below this.
    ValueBelow(f64),
    /// Points that are below this percentage of the total, 0-100 exclusive.
    PercentBelow(f64),
    /// These points, by zero-based index. At least one, no repeats.
    Points(Vec<usize>),
}

/// How a pie-of-pie or bar-of-pie divides itself, set with
/// [`ChartSpec::of_pie`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct OfPie {
    /// Which points go to the second plot.
    pub split: OfPieSplit,
    /// The second plot's size as a percentage of the first, 5-200. Excel's
    /// default is 75.
    pub second_size: u16,
    /// Draw the lines joining the two plots.
    pub series_lines: bool,
}

impl Default for OfPie {
    fn default() -> Self {
        Self {
            split: OfPieSplit::Auto,
            second_size: 75,
            series_lines: true,
        }
    }
}

impl OfPie {
    /// Automatic split, 75% second plot, with joining lines.
    pub fn new() -> Self {
        Self::default()
    }

    /// Chooses which points move to the second plot.
    #[must_use]
    pub fn split(mut self, split: OfPieSplit) -> Self {
        self.split = split;
        self
    }

    /// Sizes the second plot, 5-200 percent of the first.
    #[must_use]
    pub fn second_size(mut self, percent: u16) -> Self {
        self.second_size = percent;
        self
    }

    /// Drops the lines joining the two plots.
    #[must_use]
    pub fn without_series_lines(mut self) -> Self {
        self.series_lines = false;
        self
    }
}

/// A live value a data label can show inside its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum LabelField {
    /// The point's value.
    Value,
    /// The point's category name.
    CategoryName,
    /// The series name.
    SeriesName,
    /// The point's share of the whole. Pie and doughnut only.
    Percentage,
    /// The point's cell from the series' label range. Needs
    /// [`Series::with_label_range`].
    CellRange,
}

impl LabelField {
    /// The `type` code and the placeholder text Excel writes.
    pub(crate) fn code(self) -> (&'static str, &'static str) {
        match self {
            Self::Value => ("VALUE", "[VALUE]"),
            Self::CategoryName => ("CATEGORYNAME", "[CATEGORY NAME]"),
            Self::SeriesName => ("SERIESNAME", "[SERIES NAME]"),
            Self::Percentage => ("PERCENTAGE", "[PERCENTAGE]"),
            Self::CellRange => ("CELLRANGE", "[CELLRANGE]"),
        }
    }
}

/// One piece of a mixed data label: literal text or a live field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum LabelPart {
    /// Literal text, such as a separator.
    Text(String),
    /// A live field.
    Field(LabelField),
}

impl LabelPart {
    /// Literal text.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// A live field.
    pub fn field(field: LabelField) -> Self {
        Self::Field(field)
    }
}

/// A further set of series drawn over a chart's own, optionally against a
/// second value axis — a column chart with a line on the right-hand scale.
///
/// Combining is limited to what Excel draws on one shared category axis:
/// column, line and area kinds, with the same orientation as the chart's own.
/// A scatter (`Scatter` or `ScatterLines`) can also be added to a column, line
/// or area chart: on the primary axes its x values are positions along the
/// category axis, and on the secondary axis it gets its own pair of value axes.
/// A scatter cannot share a secondary axis with another kind, cannot be the
/// chart the others are added to, and bubble charts cannot be combined at all.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Plot {
    pub(crate) kind: ChartKind,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) series: Vec<Series>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) secondary_axis: bool,
}

impl Plot {
    /// An empty plot of `kind` on the primary axes.
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            series: Vec::new(),
            secondary_axis: false,
        }
    }

    /// Appends a series.
    #[must_use]
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Plots against a second value axis on the right, configured with
    /// [`ChartSpec::secondary_value_axis`].
    #[must_use]
    pub fn on_secondary_axis(mut self) -> Self {
        self.secondary_axis = true;
        self
    }
}

#[cfg(feature = "serde")]
fn default_gap_width() -> u16 {
    150
}

#[cfg(feature = "serde")]
fn default_hole_size() -> u8 {
    50
}

#[cfg(feature = "serde")]
fn default_bubble_scale() -> u16 {
    100
}

/// A chart, described rather than drawn.
///
/// Every method other than [`ChartSpec::new`] is optional: a spec with one
/// series renders a valid chart.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChartSpec {
    pub(crate) kind: ChartKind,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_runs: Option<Vec<TitleRun>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) series: Vec<Series>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) legend: LegendPosition,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) category_axis: Axis,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) value_axis: Axis,
    #[cfg_attr(feature = "serde", serde(default = "default_gap_width"))]
    pub(crate) gap_width: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) overlap: Option<i16>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) data_labels: Option<DataLabels>,
    #[cfg_attr(feature = "serde", serde(default = "default_hole_size"))]
    pub(crate) hole_size: u8,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) stock_bars: Option<(String, String)>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) rounded_corners: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) date_1904: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) language: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) style: Option<u8>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) pivot_source: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) drop_lines: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) high_low_lines: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) series_lines: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) of_pie: Option<OfPie>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) view_3d: Option<View3D>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) surface_bands: Vec<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) first_slice_angle: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_style: Option<TextStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_position: Option<Position>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) data_table: Option<DataTable>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) text_style: Option<TextStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) legend_overlay: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) legend_style: Option<TextStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) chart_area: Option<AreaStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) plot_area: Option<AreaStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) plot_area_layout: Option<Layout>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) legend_layout: Option<Layout>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) hidden_legend_entries: Vec<usize>,
    #[cfg_attr(feature = "serde", serde(default, rename = "plots"))]
    pub(crate) extra_plots: Vec<Plot>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) secondary_value_axis: Axis,
    #[cfg_attr(feature = "serde", serde(default = "default_bubble_scale"))]
    pub(crate) bubble_scale: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) bubble_3d: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_overlay: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) no_auto_title: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) series_axis: Option<Axis>,
}

impl ChartSpec {
    /// A chart of `kind` with defaults chosen to match Excel's own: legend on
    /// the right, both axes visible and untitled, no gridlines, no title.
    pub fn new(kind: ChartKind) -> Self {
        Self {
            kind,
            title: None,
            title_runs: None,
            series: Vec::new(),
            legend: LegendPosition::Right,
            category_axis: Axis::default(),
            value_axis: Axis::default(),
            gap_width: 150,
            overlap: None,
            data_labels: None,
            hole_size: 50,
            stock_bars: None,
            rounded_corners: false,
            date_1904: false,
            language: None,
            style: None,
            pivot_source: None,
            drop_lines: false,
            high_low_lines: false,
            series_lines: false,
            of_pie: None,
            view_3d: None,
            surface_bands: Vec::new(),
            first_slice_angle: 0,
            title_style: None,
            title_position: None,
            data_table: None,
            text_style: None,
            legend_overlay: false,
            legend_style: None,
            chart_area: None,
            plot_area: None,
            plot_area_layout: None,
            legend_layout: None,
            hidden_legend_entries: Vec::new(),
            extra_plots: Vec::new(),
            secondary_value_axis: Axis::default(),
            bubble_scale: 100,
            bubble_3d: false,
            title_overlay: false,
            no_auto_title: false,
            series_axis: None,
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

    /// Sets a chart title of several runs, each with its own formatting. Use
    /// this or [`ChartSpec::title`], not both.
    #[must_use]
    pub fn title_runs(mut self, runs: impl IntoIterator<Item = TitleRun>) -> Self {
        self.title_runs = Some(runs.into_iter().collect());
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

    /// Styles the chart title.
    #[must_use]
    pub fn title_style(mut self, style: TextStyle) -> Self {
        self.title_style = Some(style);
        self
    }

    /// The default font for every piece of text in the chart. Titles, axes,
    /// legend and labels inherit it unless they set their own.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// Lets the legend sit on top of the plot instead of shrinking it.
    #[must_use]
    pub fn legend_overlay(mut self, overlay: bool) -> Self {
        self.legend_overlay = overlay;
        self
    }

    /// Removes one entry from the legend. `index` counts the series in order
    /// across every plot (for a pie or doughnut, the points of its series).
    /// Needs a legend, and a series that exists: anything else is refused at
    /// render time. Repeats are ignored.
    #[must_use]
    pub fn hide_legend_entry(mut self, index: usize) -> Self {
        self.hidden_legend_entries.push(index);
        self
    }

    /// Styles the legend text.
    #[must_use]
    pub fn legend_style(mut self, style: TextStyle) -> Self {
        self.legend_style = Some(style);
        self
    }

    /// Fill and border of the whole chart.
    #[must_use]
    pub fn chart_area(mut self, style: AreaStyle) -> Self {
        self.chart_area = Some(style);
        self
    }

    /// Fill and border of the plot area, the rectangle inside the axes.
    #[must_use]
    pub fn plot_area(mut self, style: AreaStyle) -> Self {
        self.plot_area = Some(style);
        self
    }

    /// Shows a data table under the plot area; see [`DataTable`]. Only column,
    /// line and area charts take one; any other kind is refused at render
    /// time.
    #[must_use]
    pub fn data_table(mut self, table: DataTable) -> Self {
        self.data_table = Some(table);
        self
    }

    /// Lays the chart title over the plot area instead of making room for
    /// it. Needs a title.
    #[must_use]
    pub fn title_overlay(mut self) -> Self {
        self.title_overlay = true;
        self
    }

    /// Stops Excel giving an untitled chart a title of its own: with one
    /// series, Excel otherwise shows the series name. Refused with a title.
    #[must_use]
    pub fn no_auto_title(mut self) -> Self {
        self.no_auto_title = true;
        self
    }

    /// Formats the depth (series) axis of a 3-D chart that has one: a
    /// surface, or a 3-D line, standard area or column with a depth axis.
    /// Takes the title, gridlines, visibility, order, tick marks, tick
    /// labels, line and label style; number options do not apply.
    #[must_use]
    pub fn series_axis(mut self, axis: Axis) -> Self {
        self.series_axis = Some(axis);
        self
    }

    /// Places the chart title by hand; see [`Position`]. The chart needs a
    /// title: combining this with no title is refused at render time.
    #[must_use]
    pub fn title_position(mut self, position: Position) -> Self {
        self.title_position = Some(position);
        self
    }

    /// Places and sizes the plot area by hand; see [`Layout`].
    #[must_use]
    pub fn plot_area_layout(mut self, layout: Layout) -> Self {
        self.plot_area_layout = Some(layout);
        self
    }

    /// Places and sizes the legend by hand; see [`Layout`]. The legend must
    /// be shown: combining this with [`LegendPosition::None`] is refused at
    /// render time.
    #[must_use]
    pub fn legend_layout(mut self, layout: Layout) -> Self {
        self.legend_layout = Some(layout);
        self
    }

    /// Draws further series over the chart's own; see [`Plot`].
    #[must_use]
    pub fn plot(mut self, plot: Plot) -> Self {
        self.extra_plots.push(plot);
        self
    }

    /// Configures the right-hand value axis used by plots added with
    /// [`Plot::on_secondary_axis`].
    #[must_use]
    pub fn secondary_value_axis(mut self, axis: Axis) -> Self {
        self.secondary_value_axis = axis;
        self
    }

    /// Draws the bubbles as shaded spheres. Bubble charts only.
    #[must_use]
    pub fn bubble_3d(mut self) -> Self {
        self.bubble_3d = true;
        self
    }

    /// Bubble size as a percentage of the default, 0-300. Bubble charts only.
    #[must_use]
    pub fn bubble_scale(mut self, percent: u16) -> Self {
        self.bubble_scale = percent;
        self
    }

    /// Draws data labels on every series.
    #[must_use]
    pub fn data_labels(mut self, labels: DataLabels) -> Self {
        self.data_labels = Some(labels);
        self
    }

    /// Draws the chart in 3-D with the given view. Works on bar, column, line,
    /// area and pie kinds; refused on the rest, with added plots, and with
    /// error bars, trendlines or data-label positions where Excel has none.
    #[must_use]
    pub fn view_3d(mut self, view: View3D) -> Self {
        self.view_3d = Some(view);
        self
    }

    /// Colours the value bands of a surface or contour chart, lowest band
    /// first, as six-digit hex. Without it Excel picks the colours. Refused on
    /// any other chart, and on the wireframe kinds, which draw no bands.
    #[must_use]
    pub fn surface_bands<I, S>(mut self, colors: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.surface_bands = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Sets how a pie-of-pie or bar-of-pie splits its points and how its second
    /// plot looks. Refused on any other chart. Without it, Excel splits
    /// automatically.
    #[must_use]
    pub fn of_pie(mut self, settings: OfPie) -> Self {
        self.of_pie = Some(settings);
        self
    }

    /// Colours the up bars (close above open) and down bars (close below open)
    /// of an open-high-low-close stock chart, as six-digit hex. Without it
    /// Excel's own colours apply. Refused on any other chart.
    #[must_use]
    pub fn stock_bars(mut self, up: impl Into<String>, down: impl Into<String>) -> Self {
        self.stock_bars = Some((up.into(), down.into()));
        self
    }

    /// Colours the up bars (second series above the first) and down bars of a
    /// line chart, as six-digit hex. The same bars as [`ChartSpec::stock_bars`].
    /// Needs a plain line chart of at least two series.
    #[must_use]
    pub fn up_down_bars(self, up: impl Into<String>, down: impl Into<String>) -> Self {
        self.stock_bars(up, down)
    }

    /// Rounds the corners of the chart's border. Excel's schema default is on;
    /// this crate writes off unless asked.
    #[must_use]
    pub fn rounded_corners(mut self, on: bool) -> Self {
        self.rounded_corners = on;
        self
    }

    /// Dates are counted from 1904 rather than 1900. Set it when the workbook's
    /// own date system is 1904, so a date axis reads the same.
    #[must_use]
    pub fn date_1904(mut self, on: bool) -> Self {
        self.date_1904 = on;
        self
    }

    /// The chart's language tag, e.g. `"en-US"`.
    #[must_use]
    pub fn language(mut self, tag: impl Into<String>) -> Self {
        self.language = Some(tag.into());
        self
    }

    /// Makes this a pivot chart over the named pivot table, e.g.
    /// `"[Book1.xlsx]Sheet1!PivotTable1"`.
    ///
    /// The series still reference the pivot table's cells; the pivot table
    /// itself is a workbook part this crate does not write. Excel shows the
    /// field buttons and refreshes the chart with the table.
    ///
    /// Excel draws a pivot chart's series from their caches, not the sheet,
    /// so every series needs [`Series::with_cached_values`], and
    /// [`Series::with_cached_categories`] when it has categories; rendering
    /// refuses one without them.
    #[must_use]
    pub fn pivot_source(mut self, pivot_table: impl Into<String>) -> Self {
        self.pivot_source = Some(pivot_table.into());
        self
    }

    /// Excel's built-in chart style, 1 to 48.
    #[must_use]
    pub fn style(mut self, index: u8) -> Self {
        self.style = Some(index);
        self
    }

    /// Drops a line from each point to the category axis. Plain line and area
    /// charts only.
    #[must_use]
    pub fn drop_lines(mut self, on: bool) -> Self {
        self.drop_lines = on;
        self
    }

    /// Joins the highest and lowest point of each category. Plain line charts
    /// of at least two series only.
    #[must_use]
    pub fn high_low_lines(mut self, on: bool) -> Self {
        self.high_low_lines = on;
        self
    }

    /// Joins each series' bar tops across a stacked bar or column chart.
    #[must_use]
    pub fn series_lines(mut self, on: bool) -> Self {
        self.series_lines = on;
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

    /// The `[Content_Types].xml` entry for this part, ready to insert before
    /// `</Types>`. `part_name` is the package path with a leading slash, e.g.
    /// `"/xl/charts/chart1.xml"`.
    pub fn content_types_override(part_name: &str) -> String {
        format!(
            r#"<Override PartName="{}" ContentType="{}"/>"#,
            crate::xml::escape(part_name),
            Self::CONTENT_TYPE,
        )
    }
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
        // Excel leaves the chart-level flag out when no series has markers.
        assert!(!out.contains(r#"<c:marker val="#), "{out}");
        assert!(
            out.contains(r#"<c:marker><c:symbol val="none"/></c:marker>"#),
            "{out}"
        );
    }

    #[test]
    fn stacked_lines_group_and_keep_markers_off_unless_asked() {
        let out = rendered(ChartSpec::new(ChartKind::LinePercentStacked).series(one_series()));
        assert!(
            out.contains(r#"<c:grouping val="percentStacked"/>"#),
            "{out}"
        );
        assert!(out.contains(r#"<c:symbol val="none"/>"#), "{out}");
        let out = rendered(ChartSpec::new(ChartKind::LineMarkersStacked).series(one_series()));
        assert!(out.contains(r#"<c:grouping val="stacked"/>"#), "{out}");
        assert!(out.contains(r#"<c:marker val="1"/>"#), "{out}");
        // Excel's 3-D line has no stacked form.
        let three_d = ChartSpec::new(ChartKind::LineStacked)
            .view_3d(View3D::new())
            .series(one_series())
            .render();
        assert!(matches!(three_d, Err(ChartError::Unsupported { .. })));
    }

    #[test]
    fn a_series_explosion_pulls_every_slice_and_only_on_round_kinds() {
        let out =
            rendered(ChartSpec::new(ChartKind::Doughnut).series(one_series().with_explosion(25)));
        assert!(
            out.contains(r#"</c:tx><c:explosion val="25"/><c:cat>"#),
            "{out}"
        );
        for (kind, percent) in [(ChartKind::Line, 25), (ChartKind::Pie, 401)] {
            let refused = ChartSpec::new(kind)
                .series(one_series().with_explosion(percent))
                .render();
            assert!(refused.is_err(), "{kind:?} {percent}");
        }
    }

    #[test]
    fn bubbles_go_3d_only_on_a_bubble_chart() {
        let bubbles = one_series().with_bubble_sizes("'S'!$C$2:$C$5");
        let out = rendered(
            ChartSpec::new(ChartKind::Bubble)
                .bubble_3d()
                .series(bubbles),
        );
        assert!(out.contains(r#"<c:bubble3D val="1"/>"#), "{out}");
        let refused = ChartSpec::new(ChartKind::Line)
            .bubble_3d()
            .series(one_series())
            .render();
        assert!(matches!(refused, Err(ChartError::Unsupported { .. })));
    }

    #[test]
    fn a_title_can_overlay_and_an_untitled_chart_can_refuse_excels_title() {
        let out = rendered(
            ChartSpec::new(ChartKind::Pie)
                .title("T")
                .title_overlay()
                .series(one_series()),
        );
        assert!(
            out.contains(r#"<c:overlay val="1"/></c:title><c:autoTitleDeleted val="0"/>"#),
            "{out}"
        );
        let out = rendered(
            ChartSpec::new(ChartKind::Pie)
                .no_auto_title()
                .series(one_series()),
        );
        assert!(
            out.contains(r#"<c:chart><c:autoTitleDeleted val="1"/>"#),
            "{out}"
        );
        for spec in [
            ChartSpec::new(ChartKind::Pie).title_overlay(),
            ChartSpec::new(ChartKind::Pie).title("T").no_auto_title(),
        ] {
            assert!(matches!(
                spec.series(one_series()).render(),
                Err(ChartError::Unsupported { .. })
            ));
        }
    }

    #[test]
    fn a_series_axis_takes_axis_options_only_where_one_exists() {
        let axis = Axis::default()
            .title("Depth")
            .gridlines(true)
            .reversed(true)
            .hidden();
        let out = rendered(
            ChartSpec::new(ChartKind::Line)
                .view_3d(View3D::new())
                .series_axis(axis.clone())
                .series(one_series()),
        );
        let start = out.find("<c:serAx>").expect("a series axis");
        let ser_ax = &out[start..out[start..].find("</c:serAx>").unwrap() + start];
        for piece in [
            r#"<c:orientation val="maxMin"/>"#,
            r#"<c:delete val="1"/>"#,
            "<c:majorGridlines/>",
            "<a:t>Depth</a:t>",
            r#"<c:majorTickMark val="out"/><c:minorTickMark val="none"/>"#,
        ] {
            assert!(ser_ax.contains(piece), "{piece} in {ser_ax}");
        }
        let flat = ChartSpec::new(ChartKind::ColumnClustered)
            .series_axis(axis)
            .series(one_series());
        assert!(matches!(flat.render(), Err(ChartError::Unsupported { .. })));
    }

    #[test]
    fn smooth_scatter_lines_take_excels_smooth_style() {
        let out = rendered(
            ChartSpec::new(ChartKind::ScatterLines).series(one_series().with_smooth(true)),
        );
        assert!(
            out.contains(r#"<c:scatterStyle val="smoothMarker"/>"#),
            "{out}"
        );
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

    fn column(series: Series) -> ChartSpec {
        ChartSpec::new(ChartKind::ColumnClustered).series(series)
    }

    #[test]
    fn colours_take_a_theme_or_system_base_brightness_and_transparency() {
        let paint = |color| one_series().with_fill(Paint::Solid(color));
        let out = rendered(column(paint(Color::theme(ThemeColor::Accent1).darker(25))));
        assert!(
            out.contains(r#"<a:schemeClr val="accent1"><a:lumMod val="75000"/></a:schemeClr>"#),
            "{out}"
        );
        let out = rendered(column(paint(
            Color::system(SystemColor::Window).lighter(40),
        )));
        assert!(
            out.contains(r#"<a:sysClr val="window" lastClr="FFFFFF"><a:lumMod val="60000"/><a:lumOff val="40000"/></a:sysClr>"#),
            "{out}"
        );
        let out = rendered(column(paint(Color::rgb("8e0dd1").transparency(30))));
        assert!(
            out.contains(r#"<a:srgbClr val="8E0DD1"><a:alpha val="70000"/></a:srgbClr>"#),
            "{out}"
        );
        let mut too_bright = Color::rgb("8E0DD1");
        too_bright.lum_mod = Some(101);
        assert!(matches!(
            column(paint(too_bright)).render(),
            Err(ChartError::InvalidFill { .. })
        ));
    }

    #[test]
    fn a_series_line_takes_dash_compound_arrows_and_cap_only_where_drawn() {
        let styled = one_series()
            .with_dash(Dash::Dot)
            .with_compound(Compound::ThinThick)
            .with_arrows(None, Some(Arrow::Triangle))
            .with_cap(LineCap::Square);
        let out = rendered(ChartSpec::new(ChartKind::Line).series(styled.clone()));
        assert!(
            out.contains(r#"<a:ln cmpd="thinThick" cap="sq"><a:prstDash val="dot"/><a:round/><a:tailEnd type="triangle"/></a:ln>"#),
            "{out}"
        );
        for kind in [ChartKind::ColumnClustered, ChartKind::Scatter] {
            assert!(matches!(
                ChartSpec::new(kind).series(styled.clone()).render(),
                Err(ChartError::Unsupported { .. })
            ));
        }
        let gridlines = Axis::default()
            .gridline_style(AreaStyle::new().border("FF0000").border_dash(Dash::Dash));
        let out = rendered(column(one_series()).value_axis(gridlines));
        assert!(
            out.contains(r#"<c:majorGridlines><c:spPr><a:ln><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:prstDash val="dash"/></a:ln></c:spPr></c:majorGridlines>"#),
            "{out}"
        );
    }

    #[test]
    fn bevels_preset_and_anchored_shadows_and_corner_gradients_write_as_excel_does() {
        let effects = |effects| column(one_series().with_effects(effects));
        let out = rendered(effects(Effects::new().bevel(Bevel::Circle)));
        assert!(out.contains("<a:sp3d><a:bevelT/></a:sp3d>"), "{out}");
        let out = rendered(effects(Effects::new().bevel(Bevel::ArtDeco)));
        assert!(out.contains(r#"<a:bevelT prst="artDeco"/>"#), "{out}");
        let out = rendered(effects(
            Effects::new()
                .preset_shadow(3, Shadow::new("000000"))
                .shadow(
                    Shadow::new("000000")
                        .scale(100, -23)
                        .align(ShadowAlign::Bottom),
                ),
        ));
        assert!(out.contains(r#"<a:prstShdw prst="shdw3""#), "{out}");
        assert!(
            out.contains(r#"dir="2700000" sy="-23000" rotWithShape="0">"#),
            "{out}"
        );
        for bad in [
            Effects::new().preset_shadow(21, Shadow::new("000000")),
            Effects::new().inner_shadow(Shadow::new("000000").scale(50, 50)),
        ] {
            assert!(effects(bad).render().is_err());
        }
        let corner = Gradient::from_corner(
            GradientShape::Rectangular,
            Corner::TopLeft,
            [(0, "8E0DD1"), (100, "FFFFFF")],
        );
        let out = rendered(column(one_series().with_fill(Paint::Gradient(corner))));
        assert!(
            out.contains(r#"<a:path path="rect"><a:fillToRect r="100000" b="100000"/></a:path><a:tileRect l="-100000" t="-100000"/>"#),
            "{out}"
        );
        let linear_corner = Gradient {
            corner: Some(Corner::TopLeft),
            ..Gradient::linear(0, [(0, "8E0DD1"), (100, "FFFFFF")])
        };
        assert!(matches!(
            column(one_series().with_fill(Paint::Gradient(linear_corner))).render(),
            Err(ChartError::InvalidFill { .. })
        ));
    }

    #[test]
    fn text_takes_underline_caps_spacing_and_baseline_and_labels_a_box() {
        let style = TextStyle::new()
            .underline(Underline::Double)
            .caps(Caps::All)
            .spacing(3.0)
            .baseline(30);
        let out = rendered(column(one_series()).value_axis(Axis::default().label_style(style)));
        assert!(
            out.contains(r#"<a:defRPr u="dbl" cap="all" spc="300" baseline="30000"/>"#),
            "{out}"
        );
        let too_high = Axis::default().label_style(TextStyle::new().baseline(101));
        assert!(column(one_series()).value_axis(too_high).render().is_err());
        let labels = DataLabels::values()
            .number_format("0.0")
            .area(AreaStyle::new().fill("F0F0F0"));
        let out = rendered(column(one_series()).data_labels(labels));
        assert!(
            out.contains(r#"<c:numFmt formatCode="0.0" sourceLinked="0"/><c:spPr><a:solidFill><a:srgbClr val="F0F0F0"/></a:solidFill></c:spPr>"#),
            "{out}"
        );
    }
}
