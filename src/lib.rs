//! Author OOXML (Excel) chart parts and their drawing anchors.
//!
//! This crate emits the XML for a chart and for where it sits. It does not open,
//! read, write, or zip `.xlsx` packages — that is the caller's job, and keeping
//! it out is what makes this crate small enough to depend on.
//!
//! # Two layers
//!
//! [`ChartSpec`] describes a chart and renders it to a [`ChartPart`]. That is
//! all most callers need.
//!
//! [`RowMetrics`] and [`two_cell_anchor`] convert a pixel size into the cell
//! anchor OOXML actually stores, and [`drawing_part`] emits the drawing that
//! hosts the chart. Use these only if you are placing a chart yourself.
//!
//! # Adding a chart to a family that already exists
//!
//! Prefer [`ChartSpec::from_template`]. Cloning a chart that is known to render
//! correctly and swapping only its data references inherits every colour, font,
//! axis format and legend setting — none of which you then have to get right.
//!
//! ```
//! use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};
//!
//! let part = ChartSpec::new(ChartKind::ColumnClustered)
//!     .title("Monthly totals")
//!     .series(
//!         Series::new(
//!             SeriesName::Literal("Series A".to_string()),
//!             "'Sheet1'!$C$3:$C$38",
//!         )
//!         .with_categories("'Sheet1'!$A$3:$A$38"),
//!     )
//!     .render()
//!     .expect("a chart");
//!
//! assert!(String::from_utf8_lossy(&part.xml).contains("<c:barChart>"));
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(missing_debug_implementations, unreachable_pub)]

mod chartex;
mod chartsheet;
mod drawing;
mod error;
mod metrics;
mod reference;
mod render;
mod spec;
mod style;
mod template;
mod xml;

pub use chartex::{
    chart_ex_content_types_override, Bins, BoxWhiskerStyle, ChartEx, ChartExKind, ChartExSeries,
    ParentLabels, QuartileMethod, CHART_EX_CONTENT_TYPE, CHART_EX_RELATIONSHIP_TYPE,
};
pub use chartsheet::{
    chartsheet_content_types_override, chartsheet_drawing_part, chartsheet_part,
    workbook_chartsheet_relationship, workbook_sheet_element, CHARTSHEET_CONTENT_TYPE,
    CHARTSHEET_RELATIONSHIP_TYPE,
};
pub use drawing::{
    anchor_xml, anchor_xml_for, chart_relationships, drawing_content_types_override,
    drawing_object_xml, drawing_part, drawing_part_objects, drawing_part_with,
    drawing_relationships, drawing_relationships_typed, worksheet_drawing_element,
    worksheet_drawing_relationship, Anchor, DrawingObject, GraphicFrame, Picture, TextBox,
    CHART_RELATIONSHIP_TYPE, DRAWING_CONTENT_TYPE, DRAWING_RELATIONSHIP_TYPE,
    IMAGE_RELATIONSHIP_TYPE,
};
pub use error::ChartError;
pub use metrics::{two_cell_anchor, CellAnchor, RowMetrics, TwoCellAnchor, UnknownHeight};
pub use spec::{
    AreaStyle, Arrow, Axis, BarShape, Bevel, Caps, ChartKind, ChartPart, ChartSpec, Color,
    ColorBase, Compound, Corner, Dash, DataLabelPosition, DataLabels, DataTable, DateUnit,
    DisplayUnit, Effects, ErrorAmount, ErrorAxis, ErrorBarSide, ErrorBars, ErrorValues, Glow,
    Gradient, GradientShape, GradientStop, LabelField, LabelPart, Layout, LegendPosition, LineCap,
    MarkerSymbol, OfPie, OfPieSplit, Paint, Pattern, PatternKind, PictureFill, Plot, PointFormat,
    PointLabel, Position, Series, SeriesName, Shadow, ShadowAlign, SystemColor, TextStyle,
    ThemeColor, TickLabels, TickMark, TitleRun, Trendline, TrendlineKind, Underline, View3D,
};
pub use style::{
    chart_colors_content_types_override, chart_style_content_types_override, ChartColors,
    ChartStyle, ColorMethod, CHART_COLORS_CONTENT_TYPE, CHART_COLORS_RELATIONSHIP_TYPE,
    CHART_STYLE_CONTENT_TYPE, CHART_STYLE_RELATIONSHIP_TYPE,
};
pub use template::TemplateChart;

/// The crate README, compiled as a doctest so its examples cannot rot.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
struct ReadmeDoctests;
