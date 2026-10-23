//! Writes one chart part per chart type over `Sheet1!A1:C5`, for
//! `scripts/excel_visual.py` to draw beside Excel's own chart of that type.
//!
//! `cargo run --example visual -- <dir>`

use std::{env, fs, path::PathBuf};

use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

/// Each kind with its Excel `XlChartType` number.
pub const KINDS: &[(&str, ChartKind, i32)] = &[
    ("bar_clustered", ChartKind::BarClustered, 57),
    ("bar_stacked", ChartKind::BarStacked, 58),
    ("bar_percent", ChartKind::BarPercentStacked, 59),
    ("column_clustered", ChartKind::ColumnClustered, 51),
    ("column_stacked", ChartKind::ColumnStacked, 52),
    ("column_percent", ChartKind::ColumnPercentStacked, 53),
    ("line", ChartKind::Line, 4),
    ("line_markers", ChartKind::LineMarkers, 65),
    ("line_stacked", ChartKind::LineStacked, 63),
    ("line_percent", ChartKind::LinePercentStacked, 64),
    ("line_markers_stacked", ChartKind::LineMarkersStacked, 66),
    (
        "line_markers_percent",
        ChartKind::LineMarkersPercentStacked,
        67,
    ),
    ("area", ChartKind::Area, 1),
    ("area_stacked", ChartKind::AreaStacked, 76),
    ("area_percent", ChartKind::AreaPercentStacked, 77),
    ("scatter", ChartKind::Scatter, -4169),
    ("scatter_lines", ChartKind::ScatterLines, 74),
    ("pie", ChartKind::Pie, 5),
    ("doughnut", ChartKind::Doughnut, -4120),
    ("radar", ChartKind::Radar, -4151),
    ("radar_filled", ChartKind::RadarFilled, 82),
];

fn main() {
    let dir = PathBuf::from(env::args().nth(1).expect("usage: visual <dir>"));
    fs::create_dir_all(&dir).expect("create dir");
    let series = |column: &str| {
        Series::new(
            SeriesName::Reference(format!("Sheet1!${column}$1")),
            format!("Sheet1!${column}$2:${column}$5"),
        )
        .with_categories("Sheet1!$A$2:$A$5")
    };
    let mut list = String::new();
    for &(name, kind, excel) in KINDS {
        let mut spec = ChartSpec::new(kind).excel_look().series(series("B"));
        // Excel titles a one-series chart with the series' name, the rest "Chart Title".
        spec = if matches!(kind, ChartKind::Pie | ChartKind::Doughnut) {
            spec.title("HB")
        } else {
            spec.series(series("C")).title("Chart Title")
        };
        let part = spec.render().expect("a chart");
        fs::write(dir.join(format!("{name}.xml")), part.xml).expect("write chart");
        list.push_str(&format!("{name} {excel}\n"));
    }
    fs::write(dir.join("kinds.txt"), list).expect("write list");
}
