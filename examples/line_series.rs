//! Several series on one line chart, with a titled value axis and gridlines.
//!
//! Run with `cargo run -p ooxml-chart --example line_series`.

use ooxml_chart::{Axis, ChartKind, ChartSpec, LegendPosition, Series, SeriesName};

fn main() {
    let regions = [("North", "C"), ("South", "D"), ("East", "E")];

    let mut spec = ChartSpec::new(ChartKind::LineMarkers)
        .title("Revenue by region")
        .legend(LegendPosition::Bottom)
        .value_axis(Axis {
            title: Some("Revenue".to_string()),
            major_gridlines: true,
            visible: true,
        });

    for (name, column) in regions {
        spec = spec.series(Series {
            name: SeriesName::Literal(name.to_string()),
            categories: Some("'Sheet1'!$A$3:$A$38".to_string()),
            values: format!("'Sheet1'!${column}$3:${column}$38"),
        });
    }

    let part = spec.render().expect("a chart");
    println!("{}", String::from_utf8_lossy(&part.xml));
}
