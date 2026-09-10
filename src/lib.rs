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
//!     .series(Series {
//!         name: SeriesName::Literal("Series A".to_string()),
//!         categories: Some("'Sheet1'!$A$3:$A$38".to_string()),
//!         values: "'Sheet1'!$C$3:$C$38".to_string(),
//!     })
//!     .render()
//!     .expect("a chart");
//!
//! assert!(String::from_utf8_lossy(&part.xml).contains("<c:barChart>"));
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod drawing;
mod error;
mod metrics;
mod render;
mod spec;
mod template;
mod xml;

pub use drawing::{
    anchor_xml, drawing_part, drawing_relationships, GraphicFrame, CHART_RELATIONSHIP_TYPE,
};
pub use error::ChartError;
pub use metrics::{two_cell_anchor, CellAnchor, RowMetrics, TwoCellAnchor, UnknownHeight};
pub use spec::{Axis, ChartKind, ChartPart, ChartSpec, LegendPosition, Series, SeriesName};
pub use template::TemplateChart;

/// The crate README, compiled as a doctest so its examples cannot rot.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
struct ReadmeDoctests;
