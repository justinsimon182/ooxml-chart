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
    /// How far a pie slice is pulled out from the centre, as a percentage of
    /// the radius (0-400). Pie and doughnut only.
    pub explosion: Option<u16>,
    /// Marker shape and size in points (2-72) for this point alone. Line,
    /// scatter and unfilled radar series only.
    pub marker: Option<(MarkerSymbol, u8)>,
}

impl PointFormat {
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
    /// Line width in points.
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_width_pt: Option<f64>,
    /// Draw the line as a smooth curve rather than straight segments.
    #[cfg_attr(feature = "serde", serde(default))]
    pub smooth: bool,
    /// Marker shape and size in points (2-72).
    #[cfg_attr(feature = "serde", serde(default))]
    pub marker: Option<(MarkerSymbol, u8)>,
    /// Category labels to cache in the part, so viewers that do not
    /// recalculate can still draw the axis.
    #[cfg_attr(feature = "serde", serde(default))]
    pub categories_cache: Option<Vec<String>>,
    /// Values to cache in the part, so viewers that do not recalculate can
    /// still draw the plot. Non-finite entries are written as blanks.
    #[cfg_attr(feature = "serde", serde(default))]
    pub values_cache: Option<Vec<f64>>,
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
            bubble_sizes: None,
            points: Vec::new(),
            point_labels: Vec::new(),
            error_bars: Vec::new(),
            trendlines: Vec::new(),
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

/// A fill or line: nothing, or a solid colour.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Paint {
    /// Draw nothing.
    None,
    /// A solid colour, six hex digits without `#`.
    Color(String),
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
            label_style: None,
            crosses_max: false,
            crosses_at: None,
            display_unit: None,
            display_unit_label: false,
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

/// A further set of series drawn over a chart's own, optionally against a
/// second value axis — a column chart with a line on the right-hand scale.
///
/// Combining is limited to what Excel draws on one shared category axis:
/// column, line and area kinds, with the same orientation as the chart's own.
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
    pub(crate) first_slice_angle: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_style: Option<TextStyle>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) title_position: Option<Position>,
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
    #[cfg_attr(feature = "serde", serde(default, rename = "plots"))]
    pub(crate) extra_plots: Vec<Plot>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) secondary_value_axis: Axis,
    #[cfg_attr(feature = "serde", serde(default = "default_bubble_scale"))]
    pub(crate) bubble_scale: u16,
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
            title_style: None,
            title_position: None,
            text_style: None,
            legend_overlay: false,
            legend_style: None,
            chart_area: None,
            plot_area: None,
            plot_area_layout: None,
            legend_layout: None,
            extra_plots: Vec::new(),
            secondary_value_axis: Axis::default(),
            bubble_scale: 100,
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
