//! Writes one chart part per chart type over `Sheet1!A1:C5`, for
//! `scripts/excel_visual.py` to draw beside Excel's own chart of that type.
//!
//! `cargo run --example visual -- <dir>`

use std::{env, fs, path::PathBuf};

use ooxml_chart::{
    chart_ex_colors, chart_ex_style, BarShape, ChartEx, ChartExKind, ChartKind, ChartSpec, OfPie,
    Series, SeriesName, View3D,
};

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
        let range = if matches!(kind, ChartKind::Pie | ChartKind::Doughnut) {
            "A1:B5"
        } else {
            "A1:C5"
        };
        let part = spec.render().expect("a chart");
        fs::write(dir.join(format!("{name}.xml")), part.xml).expect("write chart");
        list.push_str(&format!("{name} {excel} {range}\n"));
    }
    // Kinds that need more than two series, or a view.
    let titled = |kind| ChartSpec::new(kind).excel_look().title("Chart Title");
    let three = |spec: ChartSpec| {
        spec.series(series("B"))
            .series(series("C"))
            .series(series("D"))
    };
    let two = |spec: ChartSpec| spec.series(series("B")).series(series("C"));
    let bubble = || {
        Series::new(
            SeriesName::Reference("Sheet1!$B$1".into()),
            "Sheet1!$B$2:$B$5",
        )
        .with_categories("Sheet1!$A$2:$A$5")
        .with_bubble_sizes("Sheet1!$C$2:$C$5")
    };
    let view = View3D::new;
    let extra: Vec<(&str, ChartSpec, i32, &str)> = vec![
        (
            "bubble",
            ChartSpec::new(ChartKind::Bubble)
                .excel_look()
                .title("HB")
                .series(bubble()),
            15,
            "A1:C5",
        ),
        (
            "stock_hlc",
            three(titled(ChartKind::StockHighLowClose)),
            88,
            "A1:D5",
        ),
        ("surface", three(titled(ChartKind::Surface)), 83, "A1:D5"),
        (
            "surface_wireframe",
            three(titled(ChartKind::SurfaceWireframe)),
            84,
            "A1:D5",
        ),
        ("contour", three(titled(ChartKind::Contour)), 85, "A1:D5"),
        (
            "contour_wireframe",
            three(titled(ChartKind::ContourWireframe)),
            86,
            "A1:D5",
        ),
        (
            "pie_of_pie",
            ChartSpec::new(ChartKind::PieOfPie)
                .excel_look()
                .title("HB")
                .series(series("B"))
                .of_pie(OfPie::new()),
            68,
            "A1:B5",
        ),
        (
            "column_3d",
            two(titled(ChartKind::ColumnClustered)).view_3d(view()),
            54,
            "A1:C5",
        ),
        (
            "column_3d_cylinder",
            two(titled(ChartKind::ColumnClustered)).view_3d(view().bar_shape(BarShape::Cylinder)),
            92,
            "A1:C5",
        ),
        (
            "bar_3d",
            two(titled(ChartKind::BarClustered)).view_3d(view()),
            60,
            "A1:C5",
        ),
        (
            "pie_3d",
            ChartSpec::new(ChartKind::Pie)
                .excel_look()
                .title("HB")
                .series(series("B"))
                .view_3d(view()),
            -4102,
            "A1:B5",
        ),
        (
            "area_3d",
            two(titled(ChartKind::Area)).view_3d(view()),
            -4098,
            "A1:C5",
        ),
        (
            "line_3d",
            two(titled(ChartKind::Line)).view_3d(view()),
            -4101,
            "A1:C5",
        ),
    ];
    // The Office 2016 kinds, each over the same cells.
    for (name, kind, excel) in [
        ("cx_waterfall", ChartExKind::Waterfall, 119),
        ("cx_funnel", ChartExKind::Funnel, 123),
        ("cx_treemap", ChartExKind::Treemap, 117),
        ("cx_sunburst", ChartExKind::Sunburst, 120),
        ("cx_histogram", ChartExKind::Histogram, 118),
        ("cx_pareto", ChartExKind::Pareto, 122),
        ("cx_box_whisker", ChartExKind::BoxWhisker, 121),
    ] {
        let mut chart = ChartEx::new(kind, "Sheet1!$B$2:$B$5").title("Chart Title");
        if !matches!(kind, ChartExKind::Histogram | ChartExKind::BoxWhisker) {
            chart = chart.categories("Sheet1!$A$2:$A$5");
        }
        let range = if matches!(kind, ChartExKind::Histogram | ChartExKind::BoxWhisker) {
            "B1:B5"
        } else {
            "A1:B5"
        };
        match chart.render() {
            Ok(xml) => {
                fs::write(dir.join(format!("{name}.xml")), xml).expect("write chart");
                let style = chart_ex_style(kind).expect("a style part");
                fs::write(dir.join(format!("{name}.style.xml")), style).expect("write style");
                fs::write(dir.join(format!("{name}.colors.xml")), chart_ex_colors())
                    .expect("write colours");
                list.push_str(&format!("{name} {excel} {range}\n"));
            }
            Err(error) => eprintln!("skipped {name}: {error}"),
        }
    }
    for (name, spec, excel, range) in extra {
        match spec.render() {
            Ok(part) => {
                fs::write(dir.join(format!("{name}.xml")), part.xml).expect("write chart");
                list.push_str(&format!("{name} {excel} {range}\n"));
            }
            Err(error) => eprintln!("skipped {name}: {error}"),
        }
    }
    fs::write(dir.join("kinds.txt"), list).expect("write list");
}
