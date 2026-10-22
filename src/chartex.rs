//! Office 2016 chart kinds: waterfall, funnel, histogram, Pareto, box and
//! whisker, treemap, sunburst and region map.
//!
//! These are not `c:` charts. Excel stores them in a separate part type, a
//! `cx:chartSpace` at `xl/charts/chartExN.xml`, reached from the drawing by a
//! relationship of type [`CHART_EX_RELATIONSHIP_TYPE`]. The drawing's frame
//! must say so too: build it with [`GraphicFrame::for_chart_ex`](crate::GraphicFrame::for_chart_ex),
//! which also gives readers without chartEx a notice shape.
//!
//! Excel itself writes each range as a hidden workbook name
//! (`_xlchart.v1.0`). It reads a plain reference just as well, and recreates
//! the names when it saves, so this crate writes plain references and leaves
//! the workbook alone.

use crate::error::ChartError;
use crate::reference;
use crate::render::{check_area_style, check_text_style, sp_pr_xml, tx_pr_xml};
use crate::spec::{AreaStyle, Paint, SeriesName, TextStyle};
use crate::xml::escape;

/// The content type of a `chartEx` part.
pub const CHART_EX_CONTENT_TYPE: &str = "application/vnd.ms-office.chartex+xml";

/// The relationship type from a drawing to a `chartEx` part.
pub const CHART_EX_RELATIONSHIP_TYPE: &str =
    "http://schemas.microsoft.com/office/2014/relationships/chartEx";

/// The `[Content_Types].xml` entry for a `chartEx` part, ready to insert
/// before `</Types>`. `part_name` has a leading slash, e.g.
/// `"/xl/charts/chartEx1.xml"`.
#[must_use]
pub fn chart_ex_content_types_override(part_name: &str) -> String {
    format!(
        r#"<Override PartName="{}" ContentType="{CHART_EX_CONTENT_TYPE}"/>"#,
        escape(part_name)
    )
}

/// An Office 2016 chart kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ChartExKind {
    /// Running total, each bar starting where the last ended.
    Waterfall,
    /// Stages narrowing from the top.
    Funnel,
    /// Values counted into bins.
    Histogram,
    /// Sorted bars with a cumulative-percentage line.
    Pareto,
    /// Quartiles, whiskers, mean and outliers of each category.
    BoxWhisker,
    /// Nested rectangles sized by value.
    Treemap,
    /// Nested rings sized by value.
    Sunburst,
    /// Countries, states or other regions shaded by value. Categories are
    /// region names; Excel looks them up on Bing when it opens the file.
    RegionMap,
}

impl ChartExKind {
    fn layout_id(self) -> &'static str {
        match self {
            Self::Waterfall => "waterfall",
            Self::Funnel => "funnel",
            Self::Histogram | Self::Pareto => "clusteredColumn",
            Self::BoxWhisker => "boxWhisker",
            Self::Treemap => "treemap",
            Self::Sunburst => "sunburst",
            Self::RegionMap => "regionMap",
        }
    }

    /// Histograms and box plots can draw values alone; the rest need labels.
    fn needs_categories(self) -> bool {
        !matches!(self, Self::Histogram | Self::BoxWhisker)
    }

    /// Excel shows a legend by default on these kinds only.
    fn default_legend(self) -> bool {
        matches!(self, Self::Waterfall | Self::Treemap | Self::RegionMap)
    }
}

/// How a histogram or Pareto chart groups its values.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Bins {
    /// This many bins of equal width.
    Count(u32),
    /// Bins of this width.
    Width(f64),
}

/// Another series on a box and whisker chart, sharing the first's categories.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct ChartExSeries {
    /// The values.
    pub values: String,
    /// The series name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: Option<SeriesName>,
    /// The series fill, border and effects.
    #[cfg_attr(feature = "serde", serde(default))]
    pub style: Option<AreaStyle>,
}

impl ChartExSeries {
    /// A series plotting `values`.
    pub fn new(values: impl Into<String>) -> Self {
        Self {
            values: values.into(),
            name: None,
            style: None,
        }
    }

    /// Sets the series fill, border and effects.
    #[must_use]
    pub fn style(mut self, style: AreaStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the series name.
    #[must_use]
    pub fn name(mut self, name: SeriesName) -> Self {
        self.name = Some(name);
        self
    }
}

/// How a box and whisker chart computes its quartiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum QuartileMethod {
    /// Median excluded from both halves (Excel's default, `QUARTILE.EXC`).
    #[default]
    Exclusive,
    /// Median included in both halves (`QUARTILE.INC`).
    Inclusive,
}

/// What a box and whisker chart draws besides the boxes. The default is
/// Excel's: mean markers and outliers shown, mean line and inner points hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields, default))]
#[non_exhaustive]
pub struct BoxWhiskerStyle {
    /// A line joining the means of the boxes.
    pub mean_line: bool,
    /// A marker at each box's mean.
    pub mean_marker: bool,
    /// Every point inside the whiskers.
    pub inner_points: bool,
    /// Points beyond the whiskers.
    pub outliers: bool,
    /// How quartiles are computed.
    pub quartile_method: QuartileMethod,
}

impl Default for BoxWhiskerStyle {
    fn default() -> Self {
        Self {
            mean_line: false,
            mean_marker: true,
            inner_points: false,
            outliers: true,
            quartile_method: QuartileMethod::Exclusive,
        }
    }
}

impl BoxWhiskerStyle {
    /// Excel's default; see the type docs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows or hides the line joining the means.
    #[must_use]
    pub fn mean_line(mut self, show: bool) -> Self {
        self.mean_line = show;
        self
    }

    /// Shows or hides the mean markers.
    #[must_use]
    pub fn mean_marker(mut self, show: bool) -> Self {
        self.mean_marker = show;
        self
    }

    /// Shows or hides the points inside the whiskers.
    #[must_use]
    pub fn inner_points(mut self, show: bool) -> Self {
        self.inner_points = show;
        self
    }

    /// Shows or hides the outliers.
    #[must_use]
    pub fn outliers(mut self, show: bool) -> Self {
        self.outliers = show;
        self
    }

    /// Sets the quartile method.
    #[must_use]
    pub fn quartile_method(mut self, method: QuartileMethod) -> Self {
        self.quartile_method = method;
        self
    }
}

/// Where a treemap puts the labels of its parent categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ParentLabels {
    /// Over the top-left of the parent's rectangles (Excel's default).
    Overlapping,
    /// In a banner above the parent's rectangles.
    Banner,
    /// Not shown.
    None,
}

/// An Office 2016 chart. Every kind has one series, except box and whisker,
/// which takes more with [`ChartEx::add_series`].
///
/// ```
/// use ooxml_chart::{ChartEx, ChartExKind};
///
/// let xml = ChartEx::new(ChartExKind::Waterfall, "'Sheet1'!$B$2:$B$7")
///     .categories("'Sheet1'!$A$2:$A$7")
///     .title("Cash flow")
///     .subtotals([5])
///     .render()
///     .unwrap();
/// assert!(String::from_utf8(xml).unwrap().contains(r#"layoutId="waterfall""#));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub struct ChartEx {
    /// The chart kind.
    pub kind: ChartExKind,
    /// The values, e.g. `"'Sheet1'!$B$2:$B$7"`.
    pub values: String,
    /// The category labels. Treemap and sunburst take several columns, one
    /// per level, e.g. `"'Sheet1'!$A$2:$B$7"`. Required except on a histogram
    /// or a box and whisker chart.
    #[cfg_attr(feature = "serde", serde(default))]
    pub categories: Option<String>,
    /// The series name.
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: Option<SeriesName>,
    /// The chart title.
    #[cfg_attr(feature = "serde", serde(default))]
    pub title: Option<String>,
    /// Show the legend, or `None` for Excel's default for the kind (shown on
    /// waterfall and treemap only).
    #[cfg_attr(feature = "serde", serde(default))]
    pub legend: Option<bool>,
    /// Waterfall only: the zero-based points drawn as totals, not changes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub subtotals: Vec<u32>,
    /// Histogram and Pareto only: the binning, or `None` for Excel's automatic bins.
    #[cfg_attr(feature = "serde", serde(default))]
    pub bins: Option<Bins>,
    /// Box and whisker only: further series after the first.
    #[cfg_attr(feature = "serde", serde(default))]
    pub more_series: Vec<ChartExSeries>,
    /// Box and whisker only: what is drawn, or `None` for Excel's default.
    #[cfg_attr(feature = "serde", serde(default))]
    pub box_whisker: Option<BoxWhiskerStyle>,
    /// Treemap only: where parent labels go, or `None` for Excel's default.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parent_labels: Option<ParentLabels>,
    /// Show value labels, or `None` for Excel's default for the kind (shown
    /// on waterfall, funnel, treemap and sunburst).
    #[cfg_attr(feature = "serde", serde(default))]
    pub data_labels: Option<bool>,
    /// Histogram and Pareto only: values below this share one underflow bin.
    #[cfg_attr(feature = "serde", serde(default))]
    pub underflow: Option<f64>,
    /// Histogram and Pareto only: values above this share one overflow bin.
    #[cfg_attr(feature = "serde", serde(default))]
    pub overflow: Option<f64>,
    /// Histogram and Pareto only: bins include their lower bound rather than
    /// their upper one (Excel's default is upper).
    #[cfg_attr(feature = "serde", serde(default))]
    pub bins_closed_left: bool,
    /// Waterfall only: show the lines joining the bars, or `None` for
    /// Excel's default (shown).
    #[cfg_attr(feature = "serde", serde(default))]
    pub connector_lines: Option<bool>,
    /// The category axis title, on kinds with axes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub category_axis_title: Option<String>,
    /// The value axis title, on kinds with a value axis.
    #[cfg_attr(feature = "serde", serde(default))]
    pub value_axis_title: Option<String>,
    /// The first series' fill, border and effects.
    #[cfg_attr(feature = "serde", serde(default))]
    pub style: Option<AreaStyle>,
    /// Single points of the first series restyled, by zero-based index.
    #[cfg_attr(feature = "serde", serde(default))]
    pub points: Vec<(u32, AreaStyle)>,
    /// The chart area's fill, border and effects.
    #[cfg_attr(feature = "serde", serde(default))]
    pub chart_area: Option<AreaStyle>,
    /// The font of every text on the chart.
    #[cfg_attr(feature = "serde", serde(default))]
    pub text_style: Option<TextStyle>,
    /// Fill, border and effects of single chart elements; see
    /// [`ChartExElement`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub element_styles: Vec<(ChartExElement, AreaStyle)>,
}

/// A part of a chartEx chart that takes its own fill, border and effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ChartExElement {
    /// The plot area behind the data.
    PlotArea,
    /// The chart title's box. Needs a title.
    Title,
    /// The legend's box. Needs a legend.
    Legend,
    /// The data labels' boxes. Needs data labels.
    DataLabels,
    /// The category axis line. Waterfall, funnel, histogram, Pareto and box
    /// and whisker.
    CategoryAxis,
    /// The value axis line. Waterfall, histogram, Pareto and box and whisker.
    ValueAxis,
    /// The category axis title's box. Needs that title.
    CategoryAxisTitle,
    /// The value axis title's box. Needs that title.
    ValueAxisTitle,
    /// The value axis's major gridlines, from the style's border.
    Gridlines,
}

impl ChartEx {
    /// A chart of `kind` plotting `values`.
    pub fn new(kind: ChartExKind, values: impl Into<String>) -> Self {
        Self {
            kind,
            values: values.into(),
            categories: None,
            name: None,
            title: None,
            legend: None,
            subtotals: Vec::new(),
            bins: None,
            more_series: Vec::new(),
            box_whisker: None,
            parent_labels: None,
            data_labels: None,
            underflow: None,
            overflow: None,
            bins_closed_left: false,
            connector_lines: None,
            category_axis_title: None,
            value_axis_title: None,
            style: None,
            points: Vec::new(),
            chart_area: None,
            text_style: None,
            element_styles: Vec::new(),
        }
    }

    /// Box and whisker only: adds a series.
    #[must_use]
    pub fn add_series(mut self, series: ChartExSeries) -> Self {
        self.more_series.push(series);
        self
    }

    /// Box and whisker only: sets what is drawn.
    #[must_use]
    pub fn box_whisker(mut self, style: BoxWhiskerStyle) -> Self {
        self.box_whisker = Some(style);
        self
    }

    /// Treemap only: sets where parent labels go.
    #[must_use]
    pub fn parent_labels(mut self, labels: ParentLabels) -> Self {
        self.parent_labels = Some(labels);
        self
    }

    /// Histogram and Pareto only: gathers values below `limit` into one bin.
    #[must_use]
    pub fn underflow(mut self, limit: f64) -> Self {
        self.underflow = Some(limit);
        self
    }

    /// Histogram and Pareto only: gathers values above `limit` into one bin.
    #[must_use]
    pub fn overflow(mut self, limit: f64) -> Self {
        self.overflow = Some(limit);
        self
    }

    /// Histogram and Pareto only: bins include their lower bound.
    #[must_use]
    pub fn bins_closed_left(mut self) -> Self {
        self.bins_closed_left = true;
        self
    }

    /// Waterfall only: shows or hides the lines joining the bars.
    #[must_use]
    pub fn connector_lines(mut self, show: bool) -> Self {
        self.connector_lines = Some(show);
        self
    }

    /// Sets the first series' fill, border and effects.
    #[must_use]
    pub fn style(mut self, style: AreaStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Restyles one point of the first series, by zero-based index. A
    /// treemap or sunburst point is one rectangle or segment.
    #[must_use]
    pub fn point(mut self, index: u32, style: AreaStyle) -> Self {
        self.points.push((index, style));
        self
    }

    /// Sets the chart area's fill, border and effects.
    #[must_use]
    pub fn chart_area(mut self, style: AreaStyle) -> Self {
        self.chart_area = Some(style);
        self
    }

    /// Sets the font of every text on the chart.
    #[must_use]
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    /// Sets one element's fill, border and effects, replacing any style it
    /// had. An element the chart does not draw is refused at render time.
    #[must_use]
    pub fn element_style(mut self, element: ChartExElement, style: AreaStyle) -> Self {
        self.element_styles.retain(|(had, _)| *had != element);
        self.element_styles.push((element, style));
        self
    }

    /// The `cx:spPr` for `element`, or nothing.
    fn element_sp_pr(&self, element: ChartExElement) -> String {
        self.element_styles
            .iter()
            .find(|(had, _)| *had == element)
            .map(|(_, style)| cx_sp_pr(style))
            .unwrap_or_default()
    }

    /// Titles the category axis.
    #[must_use]
    pub fn category_axis_title(mut self, text: impl Into<String>) -> Self {
        self.category_axis_title = Some(text.into());
        self
    }

    /// Titles the value axis.
    #[must_use]
    pub fn value_axis_title(mut self, text: impl Into<String>) -> Self {
        self.value_axis_title = Some(text.into());
        self
    }

    /// Shows or hides value labels.
    #[must_use]
    pub fn data_labels(mut self, show: bool) -> Self {
        self.data_labels = Some(show);
        self
    }

    /// Sets the category labels; see [`ChartEx::categories`](field@ChartEx::categories).
    #[must_use]
    pub fn categories(mut self, reference: impl Into<String>) -> Self {
        self.categories = Some(reference.into());
        self
    }

    /// Sets the series name.
    #[must_use]
    pub fn name(mut self, name: SeriesName) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the chart title.
    #[must_use]
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Shows or hides the legend.
    #[must_use]
    pub fn legend(mut self, show: bool) -> Self {
        self.legend = Some(show);
        self
    }

    /// Waterfall only: marks zero-based points as totals.
    #[must_use]
    pub fn subtotals(mut self, points: impl IntoIterator<Item = u32>) -> Self {
        self.subtotals = points.into_iter().collect();
        self
    }

    /// Histogram and Pareto only: sets the binning.
    #[must_use]
    pub fn bins(mut self, bins: Bins) -> Self {
        self.bins = Some(bins);
        self
    }

    fn check(&self) -> Result<(), ChartError> {
        let refuse = |what: &str| {
            Err(ChartError::Unsupported {
                what: format!("{what} on {:?}", self.kind),
            })
        };
        reference::check(&self.values)?;
        match &self.categories {
            Some(categories) => reference::check(categories)?,
            None if self.kind.needs_categories() => return refuse("no categories"),
            None => {}
        }
        let is_box = self.kind == ChartExKind::BoxWhisker;
        if !self.more_series.is_empty() && !is_box {
            return refuse("more than one series");
        }
        if self.box_whisker.is_some() && !is_box {
            return refuse("box and whisker options");
        }
        if self.parent_labels.is_some() && self.kind != ChartExKind::Treemap {
            return refuse("parent labels");
        }
        let styles = self
            .more_series
            .iter()
            .filter_map(|s| s.style.as_ref())
            .chain(self.style.iter())
            .chain(self.points.iter().map(|(_, style)| style))
            .chain(self.chart_area.iter())
            .chain(self.element_styles.iter().map(|(_, style)| style));
        for style in styles {
            // shortcut: picture fills are unverified in a chartEx part; allow
            // them once Excel has been shown to load one.
            if matches!(style.fill, Some(Paint::Picture(_))) {
                return refuse("a picture fill");
            }
            check_area_style(style)?;
        }
        if let Some(style) = &self.text_style {
            check_text_style(style)?;
        }
        for (element, _) in &self.element_styles {
            let kind = self.kind;
            let axes = matches!(
                kind,
                ChartExKind::Waterfall
                    | ChartExKind::Histogram
                    | ChartExKind::Pareto
                    | ChartExKind::BoxWhisker
            );
            let drawn = match element {
                ChartExElement::PlotArea => true,
                ChartExElement::Title => self.title.is_some(),
                ChartExElement::Legend => self.legend.unwrap_or(kind.default_legend()),
                ChartExElement::DataLabels => self.data_labels.unwrap_or(matches!(
                    kind,
                    ChartExKind::Waterfall
                        | ChartExKind::Funnel
                        | ChartExKind::Treemap
                        | ChartExKind::Sunburst
                )),
                ChartExElement::CategoryAxis => axes || kind == ChartExKind::Funnel,
                ChartExElement::ValueAxis | ChartExElement::Gridlines => axes,
                ChartExElement::CategoryAxisTitle => {
                    (axes || kind == ChartExKind::Funnel) && self.category_axis_title.is_some()
                }
                ChartExElement::ValueAxisTitle => axes && self.value_axis_title.is_some(),
            };
            if !drawn {
                return refuse(&format!(
                    "a style for {element:?}, which the chart does not draw"
                ));
            }
        }
        let binned = matches!(self.kind, ChartExKind::Histogram | ChartExKind::Pareto);
        if !binned && (self.underflow.is_some() || self.overflow.is_some() || self.bins_closed_left)
        {
            return refuse("overflow, underflow or closed-left bins");
        }
        if [self.underflow, self.overflow]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return refuse("an overflow or underflow that is not finite");
        }
        if let (Some(under), Some(over)) = (self.underflow, self.overflow) {
            if under >= over {
                return refuse("an underflow not below the overflow");
            }
        }
        if self.connector_lines.is_some() && self.kind != ChartExKind::Waterfall {
            return refuse("connector lines");
        }
        let has_axes = !matches!(
            self.kind,
            ChartExKind::Treemap | ChartExKind::Sunburst | ChartExKind::RegionMap
        );
        if !has_axes && self.category_axis_title.is_some() {
            return refuse("a category axis title");
        }
        if (!has_axes || self.kind == ChartExKind::Funnel) && self.value_axis_title.is_some() {
            return refuse("a value axis title");
        }
        for (values, name, _) in self.all_series() {
            reference::check(values)?;
            if let Some(SeriesName::Reference(name)) = name {
                reference::check(name)?;
            }
        }
        if !self.subtotals.is_empty() && self.kind != ChartExKind::Waterfall {
            return refuse("subtotals");
        }
        match self.bins {
            Some(_) if !matches!(self.kind, ChartExKind::Histogram | ChartExKind::Pareto) => {
                return refuse("bins")
            }
            Some(Bins::Count(0)) => return refuse("zero bins"),
            Some(Bins::Width(width)) if !(width.is_finite() && width > 0.0) => {
                return refuse("a bin width that is not positive")
            }
            _ => {}
        }
        Ok(())
    }

    fn series_xml(
        &self,
        out: &mut String,
        id: usize,
        name: &Option<SeriesName>,
        style: Option<&AreaStyle>,
    ) {
        let kind = self.kind;
        out.push_str(&format!(
            r#"<cx:series layoutId="{}" uniqueId="{{00000000-0000-0000-0000-{:012}}}">"#,
            kind.layout_id(),
            id + 1
        ));
        match name {
            Some(SeriesName::Literal(text)) => out.push_str(&format!(
                "<cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx>",
                escape(text)
            )),
            Some(SeriesName::Reference(cell)) => out.push_str(&format!(
                "<cx:tx><cx:txData><cx:f>{}</cx:f></cx:txData></cx:tx>",
                escape(cell)
            )),
            None => {}
        }
        if let Some(style) = style {
            out.push_str(&cx_sp_pr(style));
        }
        if id == 0 {
            for (index, style) in &self.points {
                out.push_str(&format!(
                    r#"<cx:dataPt idx="{index}">{}</cx:dataPt>"#,
                    cx_sp_pr(style)
                ));
            }
        }
        // Data labels as Excel writes them for a new chart of each kind.
        let default_labels = match kind {
            ChartExKind::Waterfall => Some(
                r#"<cx:dataLabels pos="outEnd"><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#,
            ),
            ChartExKind::Funnel => Some(
                r#"<cx:dataLabels><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#,
            ),
            ChartExKind::Treemap => Some(
                r#"<cx:dataLabels pos="inEnd"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#,
            ),
            ChartExKind::Sunburst => Some(
                r#"<cx:dataLabels pos="ctr"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#,
            ),
            _ => None,
        };
        // The labels' box style goes before their visibility.
        let box_style = self.element_sp_pr(ChartExElement::DataLabels);
        let styled =
            |labels: &str| labels.replace("<cx:visibility", &format!("{box_style}<cx:visibility"));
        match (self.data_labels, default_labels) {
            (Some(false), _) => {}
            (_, Some(labels)) => out.push_str(&styled(labels)),
            (Some(true), None) => out.push_str(&styled(
                r#"<cx:dataLabels><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#,
            )),
            (None, None) => {}
        }
        out.push_str(&format!(r#"<cx:dataId val="{id}"/>"#));
        let bins = match self.bins {
            Some(Bins::Count(count)) => format!(r#"<cx:binCount val="{count}"/>"#),
            Some(Bins::Width(width)) => format!(r#"<cx:binSize val="{width}"/>"#),
            None => String::new(),
        };
        let mut binning = format!(
            r#"<cx:binning intervalClosed="{}""#,
            if self.bins_closed_left { "l" } else { "r" }
        );
        if let Some(under) = self.underflow {
            binning.push_str(&format!(r#" underflow="{under}""#));
        }
        if let Some(over) = self.overflow {
            binning.push_str(&format!(r#" overflow="{over}""#));
        }
        let binning = format!("{binning}>{bins}</cx:binning>");
        let custom_bins = !bins.is_empty()
            || self.underflow.is_some()
            || self.overflow.is_some()
            || self.bins_closed_left;
        let flag = |on: bool| if on { "1" } else { "0" };
        match kind {
            ChartExKind::Waterfall => {
                out.push_str("<cx:layoutPr>");
                if let Some(show) = self.connector_lines {
                    out.push_str(&format!(
                        r#"<cx:visibility connectorLines="{}"/>"#,
                        flag(show)
                    ));
                }
                out.push_str("<cx:subtotals>");
                for point in &self.subtotals {
                    out.push_str(&format!(r#"<cx:idx val="{point}"/>"#));
                }
                out.push_str("</cx:subtotals></cx:layoutPr>");
            }
            ChartExKind::Histogram => {
                out.push_str(&format!("<cx:layoutPr>{binning}</cx:layoutPr>"))
            }
            // A Pareto without binning aggregates by category, as Excel's default.
            ChartExKind::Pareto if !custom_bins => {
                out.push_str(r#"<cx:layoutPr><cx:aggregation/></cx:layoutPr><cx:axisId val="1"/>"#)
            }
            ChartExKind::Pareto => out.push_str(&format!(
                r#"<cx:layoutPr>{binning}</cx:layoutPr><cx:axisId val="1"/>"#
            )),
            ChartExKind::BoxWhisker => {
                let style = self.box_whisker.unwrap_or_default();
                out.push_str(&format!(
                    r#"<cx:layoutPr><cx:visibility meanLine="{}" meanMarker="{}" nonoutliers="{}" outliers="{}"/><cx:statistics quartileMethod="{}"/></cx:layoutPr>"#,
                    flag(style.mean_line),
                    flag(style.mean_marker),
                    flag(style.inner_points),
                    flag(style.outliers),
                    match style.quartile_method {
                        QuartileMethod::Exclusive => "exclusive",
                        QuartileMethod::Inclusive => "inclusive",
                    },
                ));
            }
            ChartExKind::Treemap => out.push_str(&format!(
                r#"<cx:layoutPr><cx:parentLabelLayout val="{}"/></cx:layoutPr>"#,
                match self.parent_labels.unwrap_or(ParentLabels::Overlapping) {
                    ParentLabels::Overlapping => "overlapping",
                    ParentLabels::Banner => "banner",
                    ParentLabels::None => "none",
                }
            )),
            _ => {}
        }
        out.push_str("</cx:series>");
    }

    fn all_series(
        &self,
    ) -> impl Iterator<Item = (&String, &Option<SeriesName>, &Option<AreaStyle>)> {
        std::iter::once((&self.values, &self.name, &self.style)).chain(
            self.more_series
                .iter()
                .map(|s| (&s.values, &s.name, &s.style)),
        )
    }

    /// Renders the `chartEx` part.
    ///
    /// # Errors
    ///
    /// [`ChartError::InvalidReference`] for a malformed reference,
    /// [`ChartError::Unsupported`] for missing categories or an option the
    /// kind does not have, zero bins, or a bin width that is not positive.
    pub fn render(&self) -> Result<Vec<u8>, ChartError> {
        self.check()?;
        let kind = self.kind;
        let mut out = String::with_capacity(1536);
        out.push_str(concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<cx:chartSpace xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
            r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
            r#" xmlns:cx="http://schemas.microsoft.com/office/drawing/2014/chartex">"#,
            r#"<cx:chartData>"#,
        ));
        // Hierarchical kinds size their shapes, a region map colours them, the
        // rest plot values.
        let dim = match kind {
            ChartExKind::Treemap | ChartExKind::Sunburst => "size",
            ChartExKind::RegionMap => "colorVal",
            _ => "val",
        };
        for (id, (values, _, _)) in self.all_series().enumerate() {
            out.push_str(&format!(r#"<cx:data id="{id}">"#));
            if let Some(categories) = &self.categories {
                out.push_str(&format!(
                    r#"<cx:strDim type="cat"><cx:f>{}</cx:f></cx:strDim>"#,
                    escape(categories)
                ));
            }
            out.push_str(&format!(
                r#"<cx:numDim type="{dim}"><cx:f>{}</cx:f></cx:numDim></cx:data>"#,
                escape(values)
            ));
        }
        out.push_str("</cx:chartData><cx:chart>");
        if let Some(title) = &self.title {
            out.push_str(&format!(
                r#"<cx:title pos="t" align="ctr" overlay="0"><cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx>{}</cx:title>"#,
                escape(title),
                self.element_sp_pr(ChartExElement::Title)
            ));
        }
        out.push_str("<cx:plotArea><cx:plotAreaRegion>");
        let surface = self.element_sp_pr(ChartExElement::PlotArea);
        if !surface.is_empty() {
            out.push_str(&format!("<cx:plotSurface>{surface}</cx:plotSurface>"));
        }
        let count = self.all_series().count();
        for (id, (_, name, style)) in self.all_series().enumerate() {
            self.series_xml(&mut out, id, name, style.as_ref());
        }
        if kind == ChartExKind::Pareto {
            out.push_str(&format!(
                r#"<cx:series layoutId="paretoLine" ownerIdx="0" uniqueId="{{00000000-0000-0000-0000-{:012}}}"><cx:axisId val="2"/></cx:series>"#,
                count + 1
            ));
        }
        out.push_str("</cx:plotAreaRegion>");
        let axis_title = |text: &Option<String>, element| {
            text.as_ref().map_or(String::new(), |text| {
                format!(
                    "<cx:title><cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx>{}</cx:title>",
                    escape(text),
                    self.element_sp_pr(element)
                )
            })
        };
        let category_title =
            axis_title(&self.category_axis_title, ChartExElement::CategoryAxisTitle);
        let value_title = axis_title(&self.value_axis_title, ChartExElement::ValueAxisTitle);
        let category_line = self.element_sp_pr(ChartExElement::CategoryAxis);
        let value_line = self.element_sp_pr(ChartExElement::ValueAxis);
        let gridlines = match self.element_sp_pr(ChartExElement::Gridlines) {
            sp_pr if sp_pr.is_empty() => "<cx:majorGridlines/>".to_string(),
            sp_pr => format!("<cx:majorGridlines>{sp_pr}</cx:majorGridlines>"),
        };
        if kind == ChartExKind::Funnel {
            out.push_str(&format!(
                r#"<cx:axis id="1"><cx:catScaling gapWidth="0.0599999987"/>{category_title}<cx:tickLabels/>{category_line}</cx:axis>"#,
            ));
        }
        let gap = match kind {
            ChartExKind::Waterfall => Some("0.5"),
            ChartExKind::Histogram | ChartExKind::Pareto => Some("0"),
            ChartExKind::BoxWhisker => Some("1"),
            _ => None,
        };
        if let Some(gap) = gap {
            out.push_str(&format!(
                r#"<cx:axis id="0"><cx:catScaling gapWidth="{gap}"/>{category_title}<cx:tickLabels/>{category_line}</cx:axis><cx:axis id="1"><cx:valScaling/>{value_title}{gridlines}<cx:tickLabels/>{value_line}</cx:axis>"#
            ));
        }
        if kind == ChartExKind::Pareto {
            out.push_str(r#"<cx:axis id="2"><cx:valScaling max="1" min="0"/><cx:units unit="percentage"/><cx:tickLabels/></cx:axis>"#);
        }
        out.push_str("</cx:plotArea>");
        if self.legend.unwrap_or(kind.default_legend()) {
            let (pos, align) = if kind == ChartExKind::RegionMap {
                ("r", "min")
            } else {
                ("t", "ctr")
            };
            let sp_pr = self.element_sp_pr(ChartExElement::Legend);
            out.push_str(&if sp_pr.is_empty() {
                format!(r#"<cx:legend pos="{pos}" align="{align}" overlay="0"/>"#)
            } else {
                format!(r#"<cx:legend pos="{pos}" align="{align}" overlay="0">{sp_pr}</cx:legend>"#)
            });
        }
        out.push_str("</cx:chart>");
        if let Some(style) = &self.chart_area {
            out.push_str(&cx_sp_pr(style));
        }
        if let Some(style) = &self.text_style {
            // The c: text properties carry only a: content inside the wrapper.
            let tx_pr = tx_pr_xml(Some(style), None);
            out.push_str(
                &tx_pr
                    .replace("<c:txPr>", "<cx:txPr>")
                    .replace("</c:txPr>", "</cx:txPr>"),
            );
        }
        out.push_str("</cx:chartSpace>");
        Ok(out.into_bytes())
    }
}

/// A `c:spPr` re-tagged for the cx namespace; its content is all `a:`.
fn cx_sp_pr(style: &AreaStyle) -> String {
    let sp_pr = sp_pr_xml(style);
    format!(
        "<cx:spPr>{}</cx:spPr>",
        &sp_pr["<c:spPr>".len()..sp_pr.len() - "</c:spPr>".len()]
    )
}
