//! Office 2016 chart kinds: waterfall, funnel, histogram, Pareto, box and
//! whisker, treemap and sunburst.
//!
//! These are not `c:` charts. Excel stores them in a separate part type, a
//! `cx:chartSpace` at `xl/charts/chartExN.xml`, reached from the drawing by a
//! relationship of type [`CHART_EX_RELATIONSHIP_TYPE`]. The drawing's frame
//! must say so too: build it with [`GraphicFrame::for_chart_ex`](crate::GraphicFrame::for_chart_ex).
//!
//! Excel itself writes each range as a hidden workbook name
//! (`_xlchart.v1.0`). It reads a plain reference just as well, and recreates
//! the names when it saves, so this crate writes plain references and leaves
//! the workbook alone.

use crate::error::ChartError;
use crate::reference;
use crate::spec::SeriesName;
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
        }
    }

    /// Histograms and box plots can draw values alone; the rest need labels.
    fn needs_categories(self) -> bool {
        !matches!(self, Self::Histogram | Self::BoxWhisker)
    }

    /// Excel shows a legend by default on these kinds only.
    fn default_legend(self) -> bool {
        matches!(self, Self::Waterfall | Self::Treemap)
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

/// An Office 2016 chart with one series.
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
        }
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
        if let Some(SeriesName::Reference(name)) = &self.name {
            reference::check(name)?;
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
            r#"<cx:chartData><cx:data id="0">"#,
        ));
        if let Some(categories) = &self.categories {
            out.push_str(&format!(
                r#"<cx:strDim type="cat"><cx:f>{}</cx:f></cx:strDim>"#,
                escape(categories)
            ));
        }
        // Hierarchical kinds size their shapes; the rest plot values.
        let dim = if matches!(kind, ChartExKind::Treemap | ChartExKind::Sunburst) {
            "size"
        } else {
            "val"
        };
        out.push_str(&format!(
            r#"<cx:numDim type="{dim}"><cx:f>{}</cx:f></cx:numDim></cx:data></cx:chartData><cx:chart>"#,
            escape(&self.values)
        ));
        if let Some(title) = &self.title {
            out.push_str(&format!(
                r#"<cx:title pos="t" align="ctr" overlay="0"><cx:tx><cx:txData><cx:v>{}</cx:v></cx:txData></cx:tx></cx:title>"#,
                escape(title)
            ));
        }
        out.push_str(&format!(
            r#"<cx:plotArea><cx:plotAreaRegion><cx:series layoutId="{}" uniqueId="{{00000000-0000-0000-0000-000000000001}}">"#,
            kind.layout_id()
        ));
        match &self.name {
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
        // Data labels as Excel writes them for a new chart of each kind.
        out.push_str(match kind {
            ChartExKind::Waterfall => r#"<cx:dataLabels pos="outEnd"><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#,
            ChartExKind::Funnel => r#"<cx:dataLabels><cx:visibility seriesName="0" categoryName="0" value="1"/></cx:dataLabels>"#,
            ChartExKind::Treemap => r#"<cx:dataLabels pos="inEnd"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#,
            ChartExKind::Sunburst => r#"<cx:dataLabels pos="ctr"><cx:visibility seriesName="0" categoryName="1" value="0"/></cx:dataLabels>"#,
            _ => "",
        });
        out.push_str(r#"<cx:dataId val="0"/>"#);
        let bins = match self.bins {
            Some(Bins::Count(count)) => format!(r#"<cx:binCount val="{count}"/>"#),
            Some(Bins::Width(width)) => format!(r#"<cx:binSize val="{width}"/>"#),
            None => String::new(),
        };
        match kind {
            ChartExKind::Waterfall => {
                out.push_str("<cx:layoutPr><cx:subtotals>");
                for point in &self.subtotals {
                    out.push_str(&format!(r#"<cx:idx val="{point}"/>"#));
                }
                out.push_str("</cx:subtotals></cx:layoutPr>");
            }
            ChartExKind::Histogram => out.push_str(&format!(
                r#"<cx:layoutPr><cx:binning intervalClosed="r">{bins}</cx:binning></cx:layoutPr>"#
            )),
            ChartExKind::Pareto if bins.is_empty() => out.push_str(
                r#"<cx:layoutPr><cx:aggregation/></cx:layoutPr><cx:axisId val="1"/>"#,
            ),
            ChartExKind::Pareto => out.push_str(&format!(
                r#"<cx:layoutPr><cx:binning intervalClosed="r">{bins}</cx:binning></cx:layoutPr><cx:axisId val="1"/>"#
            )),
            ChartExKind::BoxWhisker => out.push_str(concat!(
                r#"<cx:layoutPr><cx:visibility meanLine="0" meanMarker="1" nonoutliers="0" outliers="1"/>"#,
                r#"<cx:statistics quartileMethod="exclusive"/></cx:layoutPr>"#,
            )),
            ChartExKind::Treemap => {
                out.push_str(r#"<cx:layoutPr><cx:parentLabelLayout val="overlapping"/></cx:layoutPr>"#)
            }
            _ => {}
        }
        out.push_str("</cx:series>");
        if kind == ChartExKind::Pareto {
            out.push_str(r#"<cx:series layoutId="paretoLine" ownerIdx="0" uniqueId="{00000000-0000-0000-0000-000000000002}"><cx:axisId val="2"/></cx:series>"#);
        }
        out.push_str("</cx:plotAreaRegion>");
        if kind == ChartExKind::Funnel {
            out.push_str(
                r#"<cx:axis id="1"><cx:catScaling gapWidth="0.06"/><cx:tickLabels/></cx:axis>"#,
            );
        }
        let gap = match kind {
            ChartExKind::Waterfall => Some("0.5"),
            ChartExKind::Histogram | ChartExKind::Pareto => Some("0"),
            ChartExKind::BoxWhisker => Some("1"),
            _ => None,
        };
        if let Some(gap) = gap {
            out.push_str(&format!(
                r#"<cx:axis id="0"><cx:catScaling gapWidth="{gap}"/><cx:tickLabels/></cx:axis><cx:axis id="1"><cx:valScaling/><cx:majorGridlines/><cx:tickLabels/></cx:axis>"#
            ));
        }
        if kind == ChartExKind::Pareto {
            out.push_str(r#"<cx:axis id="2"><cx:valScaling max="1" min="0"/><cx:units unit="percentage"/><cx:tickLabels/></cx:axis>"#);
        }
        out.push_str("</cx:plotArea>");
        if self.legend.unwrap_or(kind.default_legend()) {
            out.push_str(r#"<cx:legend pos="t" align="ctr" overlay="0"/>"#);
        }
        out.push_str("</cx:chart></cx:chartSpace>");
        Ok(out.into_bytes())
    }
}
