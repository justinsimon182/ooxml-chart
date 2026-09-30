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

- Horizontal bar charts wrote the category axis at `axPos="b"` and the value
  axis at `axPos="l"`; they now read sideways (`l` and `b`).
- Every generated chart now writes `<c:roundedCorners val="0"/>`. The schema
  default is true, so some readers drew rounded borders.
- `gap_width` over 500 and `overlap` outside -100..=100 are refused instead of
  written, since Excel repairs such a chart away.

### Added

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
