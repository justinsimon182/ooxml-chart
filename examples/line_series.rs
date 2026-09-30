//! Several series on one line chart, with a titled value axis and gridlines.
//!
//! Run with `cargo run -p ooxml-chart --example line_series`.

use ooxml_chart::{Axis, ChartKind, ChartSpec, LegendPosition, Series, SeriesName};

fn main() {
    let regions = [("North", "C"), ("South", "D"), ("East", "E")];

    let mut spec = ChartSpec::new(ChartKind::LineMarkers)
        .title("Revenue by region")
        .legend(LegendPosition::Bottom)
        .value_axis(Axis::default().title("Revenue").gridlines(true));

    for (name, column) in regions {
        spec = spec.series(
            Series::new(
                SeriesName::Literal(name.to_string()),
                format!("'Sheet1'!${column}$3:${column}$38"),
            )
            .with_categories("'Sheet1'!$A$3:$A$38"),
        );
    }

    let part = spec.render().expect("a chart");
    println!("{}", String::from_utf8_lossy(&part.xml));
}
