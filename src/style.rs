//! Chart style and colour parts.
//!
//! Excel 2013 and later store two optional parts beside a chart: a style part
//! (`xl/charts/style1.xml`, what the Chart Styles gallery selects) and a
//! colours part (`xl/charts/colors1.xml`, what "Change Colors" selects). A
//! chart draws from its own XML alone, so neither is needed to show it. They
//! matter when someone edits the chart: with them, the gallery and the colour
//! picker know which style the chart is in and restyle it from there.
//!
//! A chart reaches them through its `_rels` part: write one with
//! [`drawing_relationships_typed`](crate::drawing_relationships_typed), using
//! the relationship types below, and store it at `xl/charts/_rels/chartN.xml.rels`.
//!
//! The defaults here reproduce what Excel writes for a new chart (style 201,
//! colour palette 10).

use crate::error::ChartError;
use crate::render::{check_color, check_range};

/// The content type of a chart style part.
pub const CHART_STYLE_CONTENT_TYPE: &str = "application/vnd.ms-office.chartstyle+xml";

/// The content type of a chart colours part.
pub const CHART_COLORS_CONTENT_TYPE: &str = "application/vnd.ms-office.chartcolorstyle+xml";

/// The relationship type from a chart part to its style part.
pub const CHART_STYLE_RELATIONSHIP_TYPE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/chartStyle";

/// The relationship type from a chart part to its colours part.
pub const CHART_COLORS_RELATIONSHIP_TYPE: &str =
    "http://schemas.microsoft.com/office/2011/relationships/chartColorStyle";

/// The style part Excel writes for a new chart of an Office 2016 kind, byte for
/// byte, or `None` for the region map, which Excel styles from the map's own
/// data. Each kind has its own style number (a treemap's is 410, a waterfall's
/// 395), and its own default fills, borders and label colours: a treemap draws
/// white tile borders and white labels from it, so a chartEx stored with the
/// column chart's style part looks different once Excel redraws it.
#[must_use]
pub fn chart_ex_style(kind: crate::ChartExKind) -> Option<&'static [u8]> {
    use crate::ChartExKind as K;
    Some(match kind {
        K::Waterfall => include_bytes!("chart_ex_styles/waterfall.xml"),
        K::Funnel => include_bytes!("chart_ex_styles/funnel.xml"),
        K::Histogram => include_bytes!("chart_ex_styles/histogram.xml"),
        K::Pareto => include_bytes!("chart_ex_styles/pareto.xml"),
        K::BoxWhisker => include_bytes!("chart_ex_styles/boxwhisker.xml"),
        K::Treemap => include_bytes!("chart_ex_styles/treemap.xml"),
        K::Sunburst => include_bytes!("chart_ex_styles/sunburst.xml"),
        _ => return None,
    })
}

/// The colours part Excel writes for a new Office 2016 chart (palette 10), byte
/// for byte. It is the same for every kind.
#[must_use]
pub fn chart_ex_colors() -> &'static [u8] {
    include_bytes!("chart_ex_styles/colors.xml")
}

const NAMESPACES: &str = concat!(
    r#"xmlns:cs="http://schemas.microsoft.com/office/drawing/2012/chartStyle" "#,
    r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
);

/// The `[Content_Types].xml` entry for a chart style part, ready to insert
/// before `</Types>`. `part_name` has a leading slash, e.g.
/// `"/xl/charts/style1.xml"`.
#[must_use]
pub fn chart_style_content_types_override(part_name: &str) -> String {
    format!(
        r#"<Override PartName="{}" ContentType="{CHART_STYLE_CONTENT_TYPE}"/>"#,
        crate::xml::escape(part_name)
    )
}

/// The `[Content_Types].xml` entry for a chart colours part; see
/// [`chart_style_content_types_override`].
#[must_use]
pub fn chart_colors_content_types_override(part_name: &str) -> String {
    format!(
        r#"<Override PartName="{}" ContentType="{CHART_COLORS_CONTENT_TYPE}"/>"#,
        crate::xml::escape(part_name)
    )
}

/// The look of a chart style part: Excel's default style, with the colours of
/// text and lines and the text sizes changeable.
///
/// ```
/// use ooxml_chart::ChartStyle;
/// let part = ChartStyle::new().text_color("404040").title_size(16.0).render()?;
/// # Ok::<(), ooxml_chart::ChartError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct ChartStyle {
    /// The style's number. Excel's default is 201; it uses the number to
    /// highlight the style in the gallery.
    pub id: u32,
    /// Colour of titles, axis labels, the legend and data labels, six hex
    /// digits without `#`. `None` is Excel's grey (text 1 at 65 %).
    pub text_color: Option<String>,
    /// Colour of axis lines, gridlines and the chart border. `None` is
    /// Excel's light grey.
    pub line_color: Option<String>,
    /// Chart title size in points (1-4000). `None` is 14.
    pub title_size_pt: Option<f64>,
    /// Size of axis labels, the legend and data labels in points (1-4000).
    /// `None` is 9.
    pub text_size_pt: Option<f64>,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            id: 201,
            text_color: None,
            line_color: None,
            title_size_pt: None,
            text_size_pt: None,
        }
    }
}

impl ChartStyle {
    /// Excel's default style; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the style's number.
    #[must_use]
    pub fn id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Sets the colour of text, six hex digits without `#`.
    #[must_use]
    pub fn text_color(mut self, rgb: impl Into<String>) -> Self {
        self.text_color = Some(rgb.into());
        self
    }

    /// Sets the colour of axis lines, gridlines and the chart border.
    #[must_use]
    pub fn line_color(mut self, rgb: impl Into<String>) -> Self {
        self.line_color = Some(rgb.into());
        self
    }

    /// Sets the chart title size in points.
    #[must_use]
    pub fn title_size(mut self, points: f64) -> Self {
        self.title_size_pt = Some(points);
        self
    }

    /// Sets the size of axis labels, the legend and data labels in points.
    #[must_use]
    pub fn text_size(mut self, points: f64) -> Self {
        self.text_size_pt = Some(points);
        self
    }

    /// The part's bytes, to be stored at e.g. `xl/charts/style1.xml`.
    ///
    /// # Errors
    ///
    /// A colour that is not six hex digits, or a size outside 1 to 4000
    /// points.
    pub fn render(&self) -> Result<Vec<u8>, ChartError> {
        for color in [&self.text_color, &self.line_color].into_iter().flatten() {
            check_color(color)?;
        }
        let size = |field: &'static str, points: Option<f64>, default: i64| {
            let Some(points) = points else {
                return Ok(default);
            };
            // Hundredths of a point in the file; a NaN size becomes -1.
            let hundredths = if points.is_finite() {
                (points * 100.0).round() as i64
            } else {
                -1
            };
            check_range(field, hundredths, 100, 400_000).map(|()| hundredths)
        };
        let title = size("chart title size", self.title_size_pt, 1400)?;
        let small = size("chart text size", self.text_size_pt, 900)?;
        Ok(style_xml(self, title, small).into_bytes())
    }
}

fn scheme(lum_mod: u32, lum_off: u32) -> String {
    format!(
        r#"<a:schemeClr val="tx1"><a:lumMod val="{lum_mod}"/><a:lumOff val="{lum_off}"/></a:schemeClr>"#
    )
}

fn style_xml(style: &ChartStyle, title_sz: i64, small_sz: i64) -> String {
    let srgb = |rgb: &str| format!(r#"<a:srgbClr val="{}"/>"#, rgb.to_ascii_uppercase());
    let text = style
        .text_color
        .as_deref()
        .map_or_else(|| scheme(65_000, 35_000), srgb);
    let label = style
        .text_color
        .as_deref()
        .map_or_else(|| scheme(75_000, 25_000), srgb);
    let rule = style
        .line_color
        .as_deref()
        .map_or_else(|| scheme(15_000, 85_000), srgb);
    let tx1 = r#"<a:schemeClr val="tx1"/>"#;
    let dk1 = r#"<a:schemeClr val="dk1"/>"#;
    let none = r#"<cs:lnRef idx="0"/><cs:fillRef idx="0"/><cs:effectRef idx="0"/>"#;
    let auto = r#"<cs:styleClr val="auto"/>"#;
    // Fonts are the theme's minor font; only the colour varies.
    let font = |color: &str| format!(r#"<cs:fontRef idx="minor">{color}</cs:fontRef>"#);
    let plain = |color: &str| format!("{none}{}", font(color));
    let line = |color: &str| {
        format!(
            r#"<cs:spPr><a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill>{color}</a:solidFill><a:round/></a:ln></cs:spPr>"#
        )
    };
    let rule_line = line(&rule);
    let phclr = |width: &str, extra: &str, dash: &str| {
        format!(
            r#"<cs:spPr><a:ln w="{width}"{extra}><a:solidFill><a:schemeClr val="phClr"/></a:solidFill>{dash}</a:ln></cs:spPr>"#
        )
    };
    let size = |sz: i64| format!(r#"<cs:defRPr sz="{sz}" kern="1200"/>"#);
    let area_mods = r#" mods="allowNoFillOverride allowNoLineOverride""#;
    let callout_body = concat!(
        r#"<cs:bodyPr rot="0" spcFirstLastPara="1" vertOverflow="clip" horzOverflow="clip" "#,
        r#"vert="horz" wrap="square" lIns="36576" tIns="18288" rIns="36576" bIns="18288" "#,
        r#"anchor="ctr" anchorCtr="1"><a:spAutoFit/></cs:bodyPr>"#,
    );
    let light = |lum_mod: u32, lum_off: u32| line(&scheme(lum_mod, lum_off));
    let parts = [
        ("axisTitle", String::new(), plain(&text), size(1000)),
        (
            "categoryAxis",
            String::new(),
            format!("{}{rule_line}{}", plain(&text), size(small_sz)),
            String::new(),
        ),
        (
            "chartArea",
            area_mods.to_string(),
            format!(
                r#"{}<cs:spPr><a:solidFill><a:schemeClr val="bg1"/></a:solidFill><a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill>{rule}</a:solidFill><a:round/></a:ln></cs:spPr>{}"#,
                plain(tx1),
                size(1000)
            ),
            String::new(),
        ),
        (
            "dataLabel",
            String::new(),
            format!("{}{}", plain(&label), size(small_sz)),
            String::new(),
        ),
        (
            "dataLabelCallout",
            String::new(),
            format!(
                r#"{}<cs:spPr><a:solidFill><a:schemeClr val="lt1"/></a:solidFill><a:ln><a:solidFill><a:schemeClr val="dk1"><a:lumMod val="25000"/><a:lumOff val="75000"/></a:schemeClr></a:solidFill></a:ln></cs:spPr>{}{callout_body}"#,
                plain(
                    r#"<a:schemeClr val="dk1"><a:lumMod val="65000"/><a:lumOff val="35000"/></a:schemeClr>"#
                ),
                size(small_sz)
            ),
            String::new(),
        ),
        (
            "dataPoint",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0"/><cs:fillRef idx="1">{auto}</cs:fillRef><cs:effectRef idx="0"/>{}"#,
                font(tx1)
            ),
            String::new(),
        ),
        (
            "dataPoint3D",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0"/><cs:fillRef idx="1">{auto}</cs:fillRef><cs:effectRef idx="0"/>{}"#,
                font(tx1)
            ),
            String::new(),
        ),
        (
            "dataPointLine",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0">{auto}</cs:lnRef><cs:fillRef idx="1"/><cs:effectRef idx="0"/>{}{}"#,
                font(tx1),
                phclr("28575", r#" cap="rnd""#, "<a:round/>")
            ),
            String::new(),
        ),
        (
            "dataPointMarker",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0">{auto}</cs:lnRef><cs:fillRef idx="1">{auto}</cs:fillRef><cs:effectRef idx="0"/>{}{}"#,
                font(tx1),
                phclr("9525", "", "")
            ),
            String::new(),
        ),
        (
            "dataPointMarkerLayout",
            r#" symbol="circle" size="5""#.to_string(),
            "/".to_string(),
            String::new(),
        ),
        (
            "dataPointWireframe",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0">{auto}</cs:lnRef><cs:fillRef idx="1"/><cs:effectRef idx="0"/>{}{}"#,
                font(tx1),
                phclr("9525", r#" cap="rnd""#, "<a:round/>")
            ),
            String::new(),
        ),
        (
            "dataTable",
            String::new(),
            format!(
                r#"{}<cs:spPr><a:noFill/><a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill>{rule}</a:solidFill><a:round/></a:ln></cs:spPr>{}"#,
                plain(&text),
                size(small_sz)
            ),
            String::new(),
        ),
        (
            "downBar",
            String::new(),
            format!(
                r#"{}<cs:spPr><a:solidFill>{}</a:solidFill><a:ln w="9525"><a:solidFill>{}</a:solidFill></a:ln></cs:spPr>"#,
                plain(dk1),
                r#"<a:schemeClr val="dk1"><a:lumMod val="65000"/><a:lumOff val="35000"/></a:schemeClr>"#,
                scheme(65_000, 35_000)
            ),
            String::new(),
        ),
        (
            "dropLine",
            String::new(),
            format!("{}{}", plain(tx1), light(35_000, 65_000)),
            String::new(),
        ),
        (
            "errorBar",
            String::new(),
            format!("{}{}", plain(tx1), light(65_000, 35_000)),
            String::new(),
        ),
        (
            "floor",
            String::new(),
            format!(
                "{}<cs:spPr><a:noFill/><a:ln><a:noFill/></a:ln></cs:spPr>",
                plain(tx1)
            ),
            String::new(),
        ),
        (
            "gridlineMajor",
            String::new(),
            format!("{}{rule_line}", plain(tx1)),
            String::new(),
        ),
        (
            "gridlineMinor",
            String::new(),
            format!("{}{}", plain(tx1), light(5_000, 95_000)),
            String::new(),
        ),
        (
            "hiLoLine",
            String::new(),
            format!("{}{}", plain(tx1), light(75_000, 25_000)),
            String::new(),
        ),
        (
            "leaderLine",
            String::new(),
            format!("{}{}", plain(tx1), light(35_000, 65_000)),
            String::new(),
        ),
        (
            "legend",
            String::new(),
            format!("{}{}", plain(&text), size(small_sz)),
            String::new(),
        ),
        ("plotArea", area_mods.to_string(), plain(tx1), String::new()),
        (
            "plotArea3D",
            area_mods.to_string(),
            plain(tx1),
            String::new(),
        ),
        (
            "seriesAxis",
            String::new(),
            format!("{}{}", plain(&text), size(small_sz)),
            String::new(),
        ),
        (
            "seriesLine",
            String::new(),
            format!("{}{}", plain(tx1), light(35_000, 65_000)),
            String::new(),
        ),
        (
            "title",
            String::new(),
            format!(
                r#"{}<cs:defRPr sz="{title_sz}" b="0" kern="1200" spc="0" baseline="0"/>"#,
                plain(&text)
            ),
            String::new(),
        ),
        (
            "trendline",
            String::new(),
            format!(
                r#"<cs:lnRef idx="0">{auto}</cs:lnRef><cs:fillRef idx="0"/><cs:effectRef idx="0"/>{}{}"#,
                font(tx1),
                phclr("19050", r#" cap="rnd""#, r#"<a:prstDash val="sysDot"/>"#)
            ),
            String::new(),
        ),
        (
            "trendlineLabel",
            String::new(),
            format!("{}{}", plain(&text), size(small_sz)),
            String::new(),
        ),
        (
            "upBar",
            String::new(),
            format!(
                r#"{}<cs:spPr><a:solidFill><a:schemeClr val="lt1"/></a:solidFill><a:ln w="9525"><a:solidFill>{rule}</a:solidFill></a:ln></cs:spPr>"#,
                plain(dk1)
            ),
            String::new(),
        ),
        (
            "valueAxis",
            String::new(),
            format!("{}{}", plain(&text), size(small_sz)),
            String::new(),
        ),
        (
            "wall",
            String::new(),
            format!(
                "{}<cs:spPr><a:noFill/><a:ln><a:noFill/></a:ln></cs:spPr>",
                plain(tx1)
            ),
            String::new(),
        ),
    ];
    let mut out = String::with_capacity(10_240);
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    out.push_str(&format!(
        r#"<cs:chartStyle {NAMESPACES} id="{}">"#,
        style.id
    ));
    for (tag, attrs, body, tail) in parts {
        if body == "/" {
            out.push_str(&format!("<cs:{tag}{attrs}/>"));
        } else {
            out.push_str(&format!("<cs:{tag}{attrs}>{body}{tail}</cs:{tag}>"));
        }
    }
    out.push_str("</cs:chartStyle>");
    out
}

/// How a colours part spreads its colours over series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ColorMethod {
    /// Series take the colours in turn, then repeat them in lighter and darker
    /// variations. Excel's default.
    #[default]
    Cycle,
    /// Series take shades running from the first colour to the last, with the
    /// colours as the ends.
    WithinLinear,
    /// As [`ColorMethod::WithinLinear`], running across the colours.
    AcrossLinear,
    /// As [`ColorMethod::WithinLinear`], in reverse.
    WithinLinearReversed,
    /// As [`ColorMethod::AcrossLinear`], in reverse.
    AcrossLinearReversed,
}

impl ColorMethod {
    fn code(self) -> &'static str {
        match self {
            ColorMethod::Cycle => "cycle",
            ColorMethod::WithinLinear => "withinLinear",
            ColorMethod::AcrossLinear => "acrossLinear",
            ColorMethod::WithinLinearReversed => "withinLinearReversed",
            ColorMethod::AcrossLinearReversed => "acrossLinearReversed",
        }
    }
}

/// A colours part: the palette Excel's "Change Colors" offers for the chart.
///
/// With no colours set it is Excel's own palette, the theme's six accents.
///
/// ```
/// use ooxml_chart::ChartColors;
/// let part = ChartColors::new()
///     .colors(["8E0DD1", "0090B2", "28EAE4", "104991"])
///     .render()?;
/// # Ok::<(), ooxml_chart::ChartError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[non_exhaustive]
pub struct ChartColors {
    /// The palette's number. Excel's default is 10; it uses the number to
    /// highlight the palette in the picker.
    pub id: u32,
    /// How the colours are spread over series.
    pub method: ColorMethod,
    /// The palette, six hex digits each without `#`. Empty is the theme's six
    /// accent colours.
    pub colors: Vec<String>,
}

impl Default for ChartColors {
    fn default() -> Self {
        Self {
            id: 10,
            method: ColorMethod::Cycle,
            colors: Vec::new(),
        }
    }
}

impl ChartColors {
    /// Excel's default palette; chain the setters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the palette's number.
    #[must_use]
    pub fn id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Sets how the colours are spread over series.
    #[must_use]
    pub fn method(mut self, method: ColorMethod) -> Self {
        self.method = method;
        self
    }

    /// Sets the palette, six hex digits each without `#`.
    #[must_use]
    pub fn colors<S: Into<String>>(mut self, colors: impl IntoIterator<Item = S>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// The part's bytes, to be stored at e.g. `xl/charts/colors1.xml`.
    ///
    /// # Errors
    ///
    /// A colour that is not six hex digits.
    pub fn render(&self) -> Result<Vec<u8>, ChartError> {
        let mut out = String::with_capacity(1024);
        out.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        out.push_str(&format!(
            r#"<cs:colorStyle {NAMESPACES} meth="{}" id="{}">"#,
            self.method.code(),
            self.id
        ));
        if self.colors.is_empty() {
            for accent in 1..=6 {
                out.push_str(&format!(r#"<a:schemeClr val="accent{accent}"/>"#));
            }
        }
        for color in &self.colors {
            check_color(color)?;
            out.push_str(&format!(
                r#"<a:srgbClr val="{}"/>"#,
                color.to_ascii_uppercase()
            ));
        }
        // The lighter and darker shades a cycling palette moves on to once
        // its colours run out, as Excel writes them.
        out.push_str(concat!(
            "<cs:variation/>",
            r#"<cs:variation><a:lumMod val="60000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="80000"/><a:lumOff val="20000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="80000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="60000"/><a:lumOff val="40000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="50000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="70000"/><a:lumOff val="30000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="70000"/></cs:variation>"#,
            r#"<cs:variation><a:lumMod val="50000"/><a:lumOff val="50000"/></cs:variation>"#,
        ));
        out.push_str("</cs:colorStyle>");
        Ok(out.into_bytes())
    }
}
