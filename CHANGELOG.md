# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
