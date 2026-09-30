# Roadmap

What `ooxml-chart` does not do yet, and where each item stands. **Done** items
shipped in `[Unreleased]` (see `CHANGELOG.md`); **Open** items are not started.

The scope line does not move: this crate emits chart and drawing **XML**. It
does not read, write, or zip `.xlsx` packages.

## Chart kinds

| Feature | Status |
| --- | --- |
| Clustered, stacked and percent-stacked bar and column | Done |
| Line, line with markers | Done |
| Area, stacked area, percent-stacked area | Done |
| Scatter, scatter with lines | Done |
| Bubble | Done — `Series::with_bubble_sizes`, `ChartSpec::bubble_scale` |
| Pie, doughnut (`hole_size`, `first_slice_angle`) | Done |
| Radar, filled radar | Done |
| Combination charts and a secondary value axis | Done — `Plot`, `ChartSpec::plot`; column, line and area share one category axis |
| Stock, surface, 3-D, pie-of-pie, bar-of-pie | **Open** |
| Combining scatter or bubble with other kinds | **Open** — needs a shared x axis model |

## Series

| Feature | Status |
| --- | --- |
| Colour, line width, smooth lines, marker shape and size | Done |
| Per-point colour and pie explosion | Done — `PointFormat` |
| Trendlines (linear, exponential, logarithmic, polynomial, power, moving average) | Done — refused where Excel forbids them |
| Cached values and categories | Done |
| Error bars | Done — `ErrorBars`; fixed, percentage, std dev, std error, custom; x and y on scatter and bubble. `errDir` is left out on bar and column, written elsewhere |
| Per-point marker and data-label overrides | Done — `PointFormat::marker`, `PointLabel` (hide, custom text, position, font) |
| Series-name cache for referenced names | **Open** — needs a `SeriesName` change |
| Gradient and pattern fills | **Open** |

## Axes

| Feature | Status |
| --- | --- |
| Title, gridlines, visibility, number format | Done |
| Min, max, major and minor unit, reversed order, log scale | Done |
| Tick marks, tick label position, label rotation, minor gridlines | Done |
| Date axis | Done — `Axis::dates`; base unit only |
| Date axis major and minor time units | **Open** |
| Axis line styling, custom crossing value (`crossesAt`) | **Open** |
| Display units (thousands, millions) | **Open** |

## Text, colour and layout

| Feature | Status |
| --- | --- |
| Font size, bold, italic, colour and typeface for title, axes, legend, labels and chart-wide | Done — `TextStyle` |
| Chart-area and plot-area fill and border | Done — `AreaStyle` |
| Legend position, overlay | Done |
| Data labels: value, category, series name, percent, number format, position | Done — positions validated per kind |
| Manual plot-area and legend layout | Done — `Layout`, `ChartSpec::plot_area_layout`, `legend_layout` |
| Manual title and axis-title layout | **Open** |
| Leader lines, legend-entry deletion, per-label text from cells or fields | **Open** |
| Data table under the plot | **Open** |

## Drawing and package helpers

| Feature | Status |
| --- | --- |
| `twoCellAnchor`, `oneCellAnchor`, `absoluteAnchor` | Done |
| `[Content_Types].xml` entry for a chart part | Done — `ChartPart::content_types_override` |
| Drawing content-type and worksheet-relationship constants | **Open** |
| Drawings hosting things other than charts | **Open** — out of scope unless asked for |

## Crate

| Feature | Status |
| --- | --- |
| Declare a chart as JSON, TOML or YAML | Done — `serde` feature, unknown fields refused |
| Illegal XML 1.0 characters stripped from text | Done |
| CI: fmt, clippy, tests, docs, MSRV | Done — `.github/workflows/ci.yml` |
| Template mode: swap axis titles and series names | Done — `with_axis_title`, `with_series_name`; literal names and plain-text titles only |
| Validate reference syntax (`Sheet!$A$1:$B$2`) | **Open** — must not reject valid quoting |
| Round-trip check against a real Excel install | **Open** — no Excel in CI; see below |

## Verification

Every gallery part (all kinds, a fully-optioned variant of each, combo, bubble,
date axis, and one drawing per anchor type â€” 49 in all) validates against the
ECMA-376 transitional schemas, including element order:

```sh
cargo run --example gallery -- out
python scripts/validate_gallery.py out path/to/ISO-IEC29500-4_2016
```

The schemas are not vendored, so this is a manual step and not in CI.

Schema-valid is not the same as Excel-accepted. Excel also enforces rules the
XSD cannot express â€” which label positions suit which chart kind, which
combinations exist â€” and those are encoded and tested here from documented
behaviour, not from a live Excel. The first time a chart is repaired by a real
Excel is the signal to add a test for what it complained about.

| Item | Status |
| --- | --- |
| Schema validation of the gallery | Done â€” manual script |
| Schema validation in CI | **Open** â€” needs the schemas vendored or fetched |
| Round-trip through a real Excel install | **Open** |

## Ground rules for new features

- Emit elements in schema order. Excel refuses a reordered chart silently.
- Validate what Excel would repair away (colours, ranges, label positions,
  illegal combinations) and return a typed `ChartError` rather than emit a
  file that opens damaged.
- Every option gets a test that pins the emitted XML.
- Default dependencies stay at `thiserror`; anything more is an optional
  feature.
