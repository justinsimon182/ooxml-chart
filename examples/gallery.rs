//! Renders one chart of every kind, formatted, into a directory.
//!
//! Run with `cargo run --example gallery -- <output-dir>`. Handy for diffing
//! output between versions or feeding another tool's validator.

use ooxml_chart::{
    AreaStyle, Axis, ChartKind, ChartSpec, DataLabelPosition, DataLabels, DateUnit, DisplayUnit,
    ErrorAmount, ErrorBarSide, ErrorBars, ErrorValues, Layout, LegendPosition, MarkerSymbol, Plot,
    PointFormat, PointLabel, Position, Series, SeriesName, TextStyle, TickLabels, Trendline,
    TrendlineKind,
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
        ("radar_filled", ChartKind::RadarFilled),
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
    // Every kind again with every option that applies to it switched on, so a
    // schema validator sees each code path.
    for (file, kind) in kinds {
        let round = matches!(kind, ChartKind::Pie | ChartKind::Doughnut);
        let trendable = matches!(
            kind,
            ChartKind::Line
                | ChartKind::LineMarkers
                | ChartKind::Area
                | ChartKind::ColumnClustered
                | ChartKind::BarClustered
                | ChartKind::Scatter
                | ChartKind::ScatterLines
        );
        let mut first = series("North", "B", "8E0DD1")
            .with_line_width(2.0)
            .with_marker(MarkerSymbol::Square, 6)
            .with_point(1, PointFormat::new().color("0090B2").explosion(8));
        if trendable {
            first = first.with_trendline(
                Trendline::new(TrendlineKind::Polynomial(2))
                    .name("Fit")
                    .color("104991")
                    .width(1.0)
                    .show_r_squared(),
            );
        }
        let mut spec = ChartSpec::new(kind)
            .title(format!("{file} (full)"))
            .title_style(
                TextStyle::new()
                    .size(14.0)
                    .bold(true)
                    .color("010102")
                    .font("Inter"),
            )
            .text_style(TextStyle::new().size(9.0))
            .chart_area(
                AreaStyle::new()
                    .fill("F8F6F0")
                    .border("68248C")
                    .border_width(1.0),
            )
            .plot_area(AreaStyle::new().no_fill())
            .legend_overlay(true)
            .legend_style(TextStyle::new().italic(true))
            .data_labels(
                DataLabels::values()
                    .with_category()
                    .number_format("0.0")
                    .style(TextStyle::new().size(8.0)),
            )
            .series(first);
        if !round {
            spec = spec
                .series(series("South", "C", "0090B2"))
                .category_axis(
                    Axis::default()
                        .title("Quarter")
                        .minor_gridlines(true)
                        .major_tick(ooxml_chart::TickMark::Cross)
                        .tick_labels(TickLabels::Low)
                        .label_rotation(-30)
                        .label_style(TextStyle::new().size(8.0)),
                )
                .value_axis(
                    Axis::default()
                        .title("Value")
                        .title_style(TextStyle::new().bold(true))
                        .gridlines(true)
                        .min(0.0)
                        .max(20.0)
                        .major_unit(5.0)
                        .minor_unit(1.0)
                        .number_format("0")
                        .crosses_max(true),
                );
        }
        let part = spec.render().expect("a chart");
        fs::write(dir.join(format!("{file}_full.xml")), part.xml).expect("write chart");
    }

    // Drawings hosting a chart, one per anchor type.
    let frame = || ooxml_chart::GraphicFrame {
        id: 2,
        name: "Chart 1".to_string(),
        relationship_id: "rId1".to_string(),
        edit_as: None,
    };
    let corner = ooxml_chart::CellAnchor {
        col: 1,
        col_offset_emu: 0,
        row: 1,
        row_offset_emu: 0,
    };
    let two = ooxml_chart::TwoCellAnchor {
        from: corner,
        to: ooxml_chart::CellAnchor {
            col: 9,
            col_offset_emu: 4762,
            row: 20,
            row_offset_emu: 9525,
        },
    };
    let drawings = [
        ("drawing_two_cell", ooxml_chart::Anchor::TwoCell(two)),
        (
            "drawing_one_cell",
            ooxml_chart::Anchor::OneCell {
                from: corner,
                width_emu: 5_000_000,
                height_emu: 3_000_000,
            },
        ),
        (
            "drawing_absolute",
            ooxml_chart::Anchor::Absolute {
                x_emu: 100_000,
                y_emu: 200_000,
                width_emu: 5_000_000,
                height_emu: 3_000_000,
            },
        ),
    ];
    for (file, anchor) in drawings {
        let part = ooxml_chart::drawing_part_with(&[(anchor, frame())]);
        fs::write(dir.join(format!("{file}.xml")), part).expect("write drawing");
    }

    // Features that need more than a kind.
    let extras = [
        (
            "bubble",
            ChartSpec::new(ChartKind::Bubble)
                .title("bubble")
                .series(series("North", "B", "8E0DD1").with_bubble_sizes("'Sheet1'!$D$2:$D$5")),
        ),
        (
            "combo_secondary_axis",
            ChartSpec::new(ChartKind::ColumnClustered)
                .title("Revenue and margin")
                .series(series("Revenue", "B", "104991"))
                .plot(
                    Plot::new(ChartKind::LineMarkers)
                        .series(series("Margin", "C", "28EAE4"))
                        .on_secondary_axis(),
                )
                .secondary_value_axis(Axis::default().title("Margin").number_format("0%")),
        ),
        (
            "styled_pie",
            ChartSpec::new(ChartKind::Pie)
                .title("Share")
                .title_style(TextStyle::new().size(16.0).bold(true).color("010102"))
                .chart_area(
                    AreaStyle::new()
                        .fill("F8F6F0")
                        .border("68248C")
                        .border_width(1.0),
                )
                .series(
                    series("Share", "B", "000000")
                        .with_point(0, PointFormat::new().color("8E0DD1").explosion(10))
                        .with_point(1, PointFormat::new().color("0090B2"))
                        .with_point(2, PointFormat::new().color("28EAE4"))
                        .with_point(3, PointFormat::new().color("104991")),
                ),
        ),
        (
            "trend_and_log_axis",
            ChartSpec::new(ChartKind::ScatterLines)
                .title("Growth")
                .category_axis(Axis::default().title("Quarter").minor_gridlines(true))
                .value_axis(
                    Axis::default()
                        .log(10)
                        .min(1.0)
                        .tick_labels(TickLabels::Low)
                        .label_rotation(-30),
                )
                .series(
                    series("Users", "B", "8E0DD1").with_trendline(
                        Trendline::new(TrendlineKind::Exponential)
                            .show_equation()
                            .forward(1.0),
                    ),
                ),
        ),
        (
            "point_overrides_line",
            ChartSpec::new(ChartKind::LineMarkers)
                .title("Point overrides")
                .data_labels(DataLabels::values().at(DataLabelPosition::Above))
                .series(
                    series("North", "B", "8E0DD1")
                        .with_marker(MarkerSymbol::Circle, 6)
                        .with_point(
                            2,
                            PointFormat::new()
                                .marker(MarkerSymbol::Diamond, 11)
                                .color("28EAE4"),
                        )
                        .with_point_label(0, PointLabel::hidden())
                        .with_point_label(
                            2,
                            PointLabel::text("Peak")
                                .at(DataLabelPosition::Right)
                                .style(TextStyle::new().bold(true)),
                        ),
                ),
        ),
        (
            "point_overrides_column",
            ChartSpec::new(ChartKind::ColumnClustered)
                .title("Point overrides")
                .series(
                    series("North", "B", "8E0DD1")
                        .with_point(1, PointFormat::new().color("0090B2"))
                        .with_point_label(
                            1,
                            PointLabel::text("Best").at(DataLabelPosition::OutsideEnd),
                        )
                        .with_trendline(Trendline::new(TrendlineKind::Linear)),
                ),
        ),
        (
            "error_bars_column",
            ChartSpec::new(ChartKind::ColumnClustered)
                .title("Error bars")
                .series(
                    series("North", "B", "8E0DD1").with_error_bars(
                        ErrorBars::new(ErrorAmount::Custom {
                            plus: Some(ErrorValues::Reference("'Sheet1'!$D$2:$D$5".into())),
                            minus: Some(ErrorValues::Literal(vec![0.5, 1.0, 0.5, 1.5])),
                        })
                        .color("010102")
                        .width(1.25),
                    ),
                ),
        ),
        (
            "error_bars_scatter",
            ChartSpec::new(ChartKind::Scatter)
                .title("Error bars")
                .series(
                    series("North", "B", "0090B2")
                        .with_error_bars(ErrorBars::new(ErrorAmount::Percentage(10.0)).along_x())
                        .with_error_bars(
                            ErrorBars::new(ErrorAmount::StdDev(1.0))
                                .side(ErrorBarSide::Plus)
                                .end_cap(false),
                        ),
                ),
        ),
        (
            "error_bars_line",
            ChartSpec::new(ChartKind::LineMarkers)
                .title("Error bars")
                .series(
                    series("North", "B", "104991")
                        .with_error_bars(ErrorBars::new(ErrorAmount::Fixed(1.5))),
                ),
        ),
        (
            "manual_layout",
            ChartSpec::new(ChartKind::ColumnClustered)
                .title("Manual layout")
                .legend(LegendPosition::Right)
                .plot_area_layout(Layout::new(0.08, 0.15, 0.65, 0.7))
                .legend_layout(Layout::new(0.78, 0.35, 0.18, 0.25))
                .series(series("North", "B", "8E0DD1"))
                .series(series("South", "C", "0090B2")),
        ),
        (
            "manual_layout_pie_outer",
            ChartSpec::new(ChartKind::Pie)
                .title("Manual layout")
                .plot_area_layout(Layout::new(0.1, 0.15, 0.6, 0.8).outer())
                .series(series("Share", "B", "8E0DD1")),
        ),
        (
            "crosses_at",
            ChartSpec::new(ChartKind::LineMarkers)
                .title("Crosses at")
                .category_axis(Axis::default().crosses_at(5.0))
                .value_axis(Axis::default().crosses_at(2.0))
                .series(series("North", "B", "8E0DD1")),
        ),
        (
            "display_units",
            ChartSpec::new(ChartKind::ColumnClustered)
                .title("Display units")
                .value_axis(
                    Axis::default()
                        .display_units(DisplayUnit::Thousands)
                        .display_units_label(true),
                )
                .series(series("North", "B", "8E0DD1")),
        ),
        (
            "display_units_scatter",
            ChartSpec::new(ChartKind::Scatter)
                .title("Display units")
                .category_axis(Axis::default().display_units(DisplayUnit::Custom(250.0)))
                .value_axis(Axis::default().display_units(DisplayUnit::Millions))
                .series(series("North", "B", "0090B2")),
        ),
        (
            "title_position",
            ChartSpec::new(ChartKind::Line)
                .title("Placed title")
                .title_position(Position::new(0.6, 0.02))
                .value_axis(
                    Axis::default()
                        .title("Units")
                        .title_position(Position::new(0.02, 0.4)),
                )
                .series(series("North", "B", "8E0DD1")),
        ),
        (
            "axis_line",
            ChartSpec::new(ChartKind::Line)
                .title("Axis lines")
                .category_axis(Axis::default().line("8E0DD1").line_width(2.5))
                .value_axis(Axis::default().no_line())
                .series(series("North", "B", "0090B2")),
        ),
        (
            "date_axis",
            ChartSpec::new(ChartKind::Line)
                .title("Daily")
                .category_axis(
                    Axis::default()
                        .dates(DateUnit::Days)
                        .date_major(3, DateUnit::Months)
                        .date_minor(1, DateUnit::Months)
                        .number_format("mmm yy"),
                )
                .series(series("Users", "B", "0090B2")),
        ),
    ];
    for (file, spec) in extras {
        let part = spec.render().expect("a chart");
        fs::write(dir.join(format!("{file}.xml")), part.xml).expect("write chart");
    }
    println!("wrote the gallery to {}", dir.display());
}
