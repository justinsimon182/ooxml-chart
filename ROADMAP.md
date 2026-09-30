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
| Error bars | **Open** — the `errDir` rules differ by chart kind and are not yet pinned |
| Per-point marker and data-label overrides | **Open** |
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
| Manual plot-area and legend layout | **Open** |
| Per-label text, leader lines, legend-entry deletion | **Open** |
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
| Template mode: swap axis titles and series names | **Open** |
| Validate reference syntax (`Sheet!$A$1:$B$2`) | **Open** — must not reject valid quoting |
| Round-trip check against a real Excel install | **Open** — no Excel in CI; see below |

## Verification gap

Output is checked by unit tests that pin the emitted XML, a well-formedness
check on every rendered part, and a load through `openpyxl`'s chart reader for
the gallery. None of that is Excel. The schema element order was written from
ECMA-376; the first time a chart is opened in a real Excel and repaired is the
signal to add a test for what it complained about.

## Ground rules for new features

- Emit elements in schema order. Excel refuses a reordered chart silently.
- Validate what Excel would repair away (colours, ranges, label positions,
  illegal combinations) and return a typed `ChartError` rather than emit a
  file that opens damaged.
- Every option gets a test that pins the emitted XML.
- Default dependencies stay at `thiserror`; anything more is an optional
  feature.
