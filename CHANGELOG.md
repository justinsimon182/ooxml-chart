# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- `Series`, `Axis` and the new formatting types are `#[non_exhaustive]`. Build
  a series with `Series::new(name, values).with_categories(..)` and an axis with
  `Axis::default().title(..).gridlines(..)`; struct literals no longer compile
  outside this crate.
- Percent-stacked, area, scatter, doughnut and radar kinds join `ChartKind`
  (already `#[non_exhaustive]`).

### Fixed

- Characters XML 1.0 cannot carry (control characters, U+FFFE, U+FFFF) are
  dropped from titles, names and labels instead of making Excel report the file
  damaged.

- Horizontal bar charts wrote the category axis at `axPos="b"` and the value
  axis at `axPos="l"`; they now read sideways (`l` and `b`).
- Every generated chart now writes `<c:roundedCorners val="0"/>`. The schema
  default is true, so some readers drew rounded borders.
- `gap_width` over 500 and `overlap` outside -100..=100 are refused instead of
  written, since Excel repairs such a chart away.

### Added

- `Axis::display_units_caption` words the caption beside a scaled axis
  yourself. Refused without a display unit or with empty text.
- Surface charts: `ChartKind::Surface`, `SurfaceWireframe`, `Contour` and
  `ContourWireframe` write `<c:surface3DChart>` or `<c:surfaceChart>` with the
  depth axis they need. `ChartSpec::surface_bands` colours the value bands, and
  a surface takes `view_3d` (a contour is fixed to a view from above). Refused:
  data labels, series colours, markers, points, labels, trendlines and error bars,
  band colours on a wireframe or another chart, combinations, data tables, date
  axes and a view on a contour.
- 3-D charts: `ChartSpec::view_3d(View3D)` draws bar, column, line, area and
  pie kinds in 3-D (`<c:bar3DChart>`, `line3DChart`, `area3DChart`,
  `pie3DChart`). `View3D` sets the rotation, perspective, height and depth,
  gap depth, bar shape (`BarShape`), an optional depth axis on a clustered bar or
  column, and the floor and walls. Refused: other kinds, combinations, error
  bars, trendlines and data-label positions on anything but a pie, and settings
  that do not apply (floor on a pie, perspective with right-angle axes, ranges).
- Pie-of-pie and bar-of-pie: `ChartKind::PieOfPie` and `ChartKind::BarOfPie`
  write `<c:ofPieChart>`. `ChartSpec::of_pie(OfPie)` chooses the split (`Auto`,
  `LastPoints`, `ValueBelow`, `PercentBelow` or custom `Points`), the second
  plot's size and the joining lines; the gap between plots is `gap_width`.
  Points, data labels, leader lines and legend-entry deletion work as on a pie.
  Refused: more than one series, settings on another chart, bad sizes and
  splits, combinations, error bars and trendlines.
- Stock charts: `ChartKind::StockHighLowClose` (three series) and
  `ChartKind::StockOpenHighLowClose` (four), written as `<c:stockChart>` with
  hi-low lines, up/down bars on the latter (`ChartSpec::stock_bars` colours
  them), series lines switched off and a tick on the close of a high-low-close
  chart. A stock plot can be added to a column chart on the secondary axis for a
  volume chart, and takes a date axis. Refused: the wrong number of series, bar
  colours without an open-high-low-close plot, trendlines on a stock series.
- Scatter combined with column, line and area: add a `Scatter` or `ScatterLines`
  `Plot` to such a chart, on the primary axes (x values are positions on the
  category axis) or on the secondary axis (its own pair of value axes, x hidden,
  y on the right). Refused: a scatter and another kind on one secondary axis, a
  scatter as the base chart, a data table alongside a scatter, and bubble in any
  combination.
- Drawing packaging helpers: `DRAWING_CONTENT_TYPE`, `DRAWING_RELATIONSHIP_TYPE`,
  `drawing_content_types_override`, `worksheet_drawing_relationship` and
  `worksheet_drawing_element`, so a caller can register a drawing and link it
  from a worksheet without hard-coding the strings.
- Mixed text-and-field data labels: `PointLabel::parts` takes `LabelPart::text`
  and `LabelPart::field(LabelField::..)` (value, category name, series name,
  percentage, cell range) and writes `<a:fld>` runs with the `c15` field-table
  extension, so Excel keeps the fields live. Percentage is refused off pie and
  doughnut, a cell-range field without a label range, and parts combined with
  `text` or `hidden`.
- `Series::with_cached_name` caches the text of a referenced series name in
  `<c:strCache>`, as categories and values already could. Refused on a literal
  name.
- Reference syntax validation: every series reference (values, categories,
  name, bubble sizes, label range, custom error bars) and every template
  replacement is checked with `ChartError::InvalidReference`. It accepts quoted
  and unquoted sheet names, `[1]` workbook indexes, cells, ranges, whole
  columns and rows, defined names and parenthesised unions, and refuses what
  Excel would not parse (an unquoted name with spaces, a missing `!`, a stray
  colon, a column past XFD). It checks shape only, not whether a sheet exists.
- Gradient and pattern fills: `Paint` gains `Gradient` (two to ten stops at an
  angle) and `Pattern` (all 54 preset patterns, two colours). They work on chart
  and plot areas and borders (`AreaStyle::fill_paint`, `border_paint`), axis lines
  (`Axis::line_paint`), series (`Series::with_fill`, bars, areas, bubbles and
  filled radar) and points (`PointFormat::fill`). Bad stops, angles and colours
  are refused with `ChartError::InvalidFill` or `InvalidColor`. `Paint` is now
  `#[non_exhaustive]`.
- CI job `schema` validates every gallery part against the ECMA-376 transitional
  schemas, vendored in `scripts/schemas` (test-only, excluded from the published
  crate).
- Data label text from cells ("Value From Cells"): `Series::with_label_range`
  (and `with_cached_label_range`) plus `DataLabels::with_cells` write the
  Office 2013 `c15:datalabelsRange` and `c15:showDataLabelsRange` extensions.
  A range nobody shows, cell labels on a series with no range, an empty range
  and a cache with no range are refused.
- Legend entry deletion: `ChartSpec::hide_legend_entry(index)` writes
  `<c:legendEntry>` with `<c:delete>`. Refused with no legend or for a series
  that does not exist.
- Pie and doughnut leader lines: `DataLabels::with_leader_lines` writes
  `<c:showLeaderLines>`. Refused on other kinds.
- Data table: `ChartSpec::data_table` (`DataTable`: borders, outline, legend
  keys, font) writes `<c:dTable>` under the plot area of column, line and area
  charts. Other kinds are refused.
- Axis line styling: `Axis::line`, `no_line` and `line_width` write the axis
  `<c:spPr>` outline (colour, hidden, width). Bad colours and widths are
  refused.
- Manual title placement: `ChartSpec::title_position` and
  `Axis::title_position` (`Position`, fractions of the chart) write a
  `<c:layout>` inside the title. Refused with no title to place, or for a
  position outside 0 to 1.
- Display units: `Axis::display_units` (`DisplayUnit`, built-in or custom
  divisor) and `display_units_label` write `<c:dispUnits>` on value axes and on
  a scatter or bubble x axis. Refused on other category axes, for a
  non-positive custom divisor, and for a label with no unit.
- `Axis::crosses_at` writes `<c:crossesAt>`: where an axis crosses the other,
  in the other axis's units. Refused with `crosses_max`, when not finite, and
  at zero or below against a logarithmic axis.
- Date-axis tick spacing: `Axis::date_major` and `date_minor` write
  `majorUnit`/`majorTimeUnit` and the minor pair after `baseTimeUnit`. Refused
  when fractional, finer than the axis unit, or set on a non-date axis.
- Manual layout: `Layout`, `ChartSpec::plot_area_layout` and
  `ChartSpec::legend_layout` place and size the plot area (inner or outer) or
  the legend as fractions of the chart. New `ChartError::InvalidLayout`; a
  legend layout with no legend is refused.
- Error bars: `ErrorBars` and `Series::with_error_bars`, with `ErrorAmount`
  (fixed, percentage, standard deviation, standard error, custom from a range
  or literal values), `ErrorBarSide`, `ErrorAxis`, end caps, colour and width.
  Refused on pie, doughnut and radar, and for x bars off a scatter or bubble.
- Per-point overrides: `PointFormat::marker` restyles one marker, and its
  `color` now recolours a marker on line, scatter and radar series.
  `PointLabel` and `Series::with_point_label` hide, reword, move or restyle one
  data label; the rest of the series keeps the chart-wide labels.
- Template mode: `TemplateChart::with_axis_title` and `with_series_name` swap an
  axis title or a literal series name by document-order index
  (`axis_count`, `series_count`). New errors `TemplateIndexOutOfRange`,
  `NoAxisTitleInTemplate`, `NoSeriesNameInTemplate` and `UnclosedElement`.
- `scripts/validate_gallery.py` validates the gallery example's output against
  the ECMA-376 schemas; the gallery now also writes a fully-optioned variant of
  every kind and one drawing per anchor type.

- Combination charts: `Plot` and `ChartSpec::plot` draw column, line and area
  series together, optionally on a second value axis
  (`Plot::on_secondary_axis`, `ChartSpec::secondary_value_axis`).
- `ChartKind::Bubble` (`Series::with_bubble_sizes`, `ChartSpec::bubble_scale`)
  and `ChartKind::RadarFilled`.
- `TextStyle` for the title, axis titles, tick labels, legend, data labels and
  chart-wide text; `AreaStyle` and `Paint` for chart-area and plot-area fill and
  border; `ChartSpec::legend_overlay`.
- Axis options: `log`, `minor_gridlines`, `minor_unit`, `tick_labels`
  (`TickLabels`), `major_tick` and `minor_tick` (`TickMark`), `label_rotation`,
  `crosses_max`, and a date axis via `Axis::dates` (`DateUnit`).
- `PointFormat` and `Series::with_point` for per-point colour and pie explosion;
  `Trendline`, `TrendlineKind` and `Series::with_trendline`.
- `ChartPart::content_types_override`.
- Optional `serde` feature: every spec type implements `Serialize` and
  `Deserialize`, and unknown fields are refused.
- `ChartError::Unsupported` and `ChartError::MissingBubbleSizes`.
- GitHub Actions CI: fmt, clippy, tests, docs, and an MSRV build.
- Chart kinds: `BarPercentStacked`, `ColumnPercentStacked`, `Area`,
  `AreaStacked`, `AreaPercentStacked`, `Scatter`, `ScatterLines`, `Doughnut`,
  `Radar`. `ChartSpec::hole_size` and `first_slice_angle` for round charts.
- Series formatting: `with_color`, `with_line_width`, `with_smooth`,
  `with_marker` (`MarkerSymbol`), `with_cached_categories`,
  `with_cached_values`.
- Axis scaling and format: `number_format`, `min`, `max`, `major_unit`,
  `reversed`, `hidden`.
- `DataLabels` and `ChartSpec::data_labels`, with label positions validated per
  chart kind.
- `Anchor` (`TwoCell`, `OneCell`, `Absolute`), `anchor_xml_for` and
  `drawing_part_with`.
- `ChartError::InvalidColor`, `InvalidAxisRange`, `InvalidDataLabelPosition`
  and `OutOfRange`.
- `examples/gallery.rs`, and `ROADMAP.md` listing what is still open.

- `ChartSpec` for declaring a chart: clustered and stacked bar and column, line
  with and without markers, and pie.
- `ChartSpec::from_template` and `TemplateChart` for extending a chart family by
  cloning one of its members and swapping the data references.
- `RowMetrics`, `UnknownHeight` and `two_cell_anchor` for converting a pixel
  size into an OOXML cell anchor with a caller-supplied row-height table.
- `drawing_part`, `anchor_xml` and `drawing_relationships` for emitting the
  drawing that hosts a chart.
- `GraphicFrame::edit_as`, choosing what an anchored chart does when the cells
  under it are resized. `None` omits the attribute, which the schema reads as
  `twoCell` and which is what real drawings carry.
- `ChartError::MixedNamespacePrefixes`, refusing a template that spells its
  chart elements both `c:`-prefixed and unprefixed. Reading one would mean
  guessing which element is the chart's own title, and guessing wrong renames
  an axis and reports success.
