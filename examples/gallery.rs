//! Renders one chart of every kind, formatted, into a directory.
//!
//! Run with `cargo run --example gallery -- <output-dir>`. Handy for diffing
//! output between versions or feeding another tool's validator.

use ooxml_chart::{
    Axis, ChartKind, ChartSpec, DataLabelPosition, DataLabels, LegendPosition, MarkerSymbol,
    Series, SeriesName,
};
use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "gallery".to_string()));
    fs::create_dir_all(&dir).expect("output directory");

    let series = |name: &str, column: &str, color: &str| {
        Series::new(
            SeriesName::Literal(name.to_string()),
            format!("'Sheet1'!${column}$2:${column}$5"),
        )
        .with_categories("'Sheet1'!$A$2:$A$5")
        .with_color(color)
        .with_cached_categories(vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()])
        .with_cached_values(vec![4.0, 7.5, 6.0, 9.25])
    };

    let kinds = [
        ("bar_clustered", ChartKind::BarClustered),
        ("bar_stacked", ChartKind::BarStacked),
        ("bar_percent", ChartKind::BarPercentStacked),
        ("column_clustered", ChartKind::ColumnClustered),
        ("column_stacked", ChartKind::ColumnStacked),
        ("column_percent", ChartKind::ColumnPercentStacked),
        ("line", ChartKind::Line),
        ("line_markers", ChartKind::LineMarkers),
        ("area", ChartKind::Area),
        ("area_stacked", ChartKind::AreaStacked),
        ("area_percent", ChartKind::AreaPercentStacked),
        ("scatter", ChartKind::Scatter),
        ("scatter_lines", ChartKind::ScatterLines),
        ("pie", ChartKind::Pie),
        ("doughnut", ChartKind::Doughnut),
        ("radar", ChartKind::Radar),
    ];

    for (file, kind) in kinds {
        let round = matches!(kind, ChartKind::Pie | ChartKind::Doughnut);
        let mut first = series("North", "B", "8E0DD1");
        let mut second = series("South", "C", "0090B2");
        if kind == ChartKind::LineMarkers {
            first = first.with_marker(MarkerSymbol::Circle, 7).with_smooth(true);
            second = second
                .with_marker(MarkerSymbol::Diamond, 7)
                .with_line_width(2.5);
        }
        let mut spec = ChartSpec::new(kind)
            .title(format!("{file} (gallery)"))
            .legend(LegendPosition::Bottom)
            .series(first);
        if !round {
            spec = spec
                .series(second)
                .value_axis(Axis::default().title("Value").gridlines(true).min(0.0));
        }
        if matches!(kind, ChartKind::BarClustered | ChartKind::ColumnClustered) {
            spec = spec.data_labels(
                DataLabels::values()
                    .at(DataLabelPosition::OutsideEnd)
                    .number_format("0.0"),
            );
        } else if round {
            spec = spec.data_labels(DataLabels::default().with_category().with_percent());
        }
        let part = spec.render().expect("a chart");
        fs::write(dir.join(format!("{file}.xml")), part.xml).expect("write chart");
    }
    println!("wrote {} charts to {}", kinds.len(), dir.display());
}
