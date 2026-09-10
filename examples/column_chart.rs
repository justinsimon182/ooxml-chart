//! The smallest useful chart: one series of columns.
//!
//! Run with `cargo run -p ooxml-chart --example column_chart`.

use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

fn main() {
    let part = ChartSpec::new(ChartKind::ColumnClustered)
        .title("Monthly totals")
        .series(Series {
            name: SeriesName::Literal("Series A".to_string()),
            categories: Some("'Sheet1'!$A$3:$A$38".to_string()),
            values: "'Sheet1'!$C$3:$C$38".to_string(),
        })
        .render()
        .expect("a chart");

    // Store these bytes at e.g. `xl/charts/chart1.xml`, register
    // `part.content_type` in `[Content_Types].xml`, and point a drawing at it.
    println!("{}", String::from_utf8_lossy(&part.xml));
}
