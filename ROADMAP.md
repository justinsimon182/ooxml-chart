# Roadmap

What `ooxml-chart` does not do yet, why it matters to consumers such as
Project Helios, and where each item stands. Items marked **Done** shipped in
`[Unreleased]` (see `CHANGELOG.md`). Items marked **Open** are not started.

The scope line does not move: this crate emits chart and drawing **XML**. It
does not read, write, or zip `.xlsx` packages.

## Gaps found in the 0.1.0 audit

### Chart kinds

| Gap | Status |
| --- | --- |
| Percent-stacked bar and column | Done — `BarPercentStacked`, `ColumnPercentStacked` |
| Area, stacked area, percent-stacked area | Done — `Area`, `AreaStacked`, `AreaPercentStacked` |
| Scatter (markers, and lines with markers) | Done — `Scatter`, `ScatterLines` |
| Doughnut | Done — `Doughnut`, with `hole_size` and `first_slice_angle` |
| Radar | Done — `Radar` |
| Combination charts and secondary axes | **Open** — needs a multi-plot model in `ChartSpec` |
| Bubble, stock, surface, 3-D, filled radar, pie-of-pie | **Open** |

### Defects in what already shipped

| Gap | Status |
| --- | --- |
| Horizontal bar charts put the category axis at the bottom (`axPos="b"`) and the value axis at the left. Excel tolerates it; other readers draw the axes swapped. | Done |
| `<c:roundedCorners>` was never written. The schema default is *true*, so some readers draw rounded chart borders. | Done |
| Charts carried no cached values, so viewers that do not recalculate (previews, some importers) drew an empty plot. | Done — `Series::with_cached_values` and `with_cached_categories` |

### Series formatting

| Gap | Status |
| --- | --- |
| Series colour | Done — `Series::with_color` (validated `RRGGBB`) |
| Line width | Done — `Series::with_line_width` |
| Smoothed lines | Done — `Series::with_smooth` |
| Marker symbol and size | Done — `Series::with_marker` |
| Per-point colours (`<c:dPt>`) | **Open** |
| Trendlines, error bars | **Open** |

### Axes

| Gap | Status |
| --- | --- |
| Number format | Done — `Axis::number_format` |
| Minimum, maximum, major unit | Done — `Axis::min`, `max`, `major_unit` (range validated) |
| Reversed order | Done — `Axis::reversed` |
| Logarithmic scale | **Open** |
| Tick mark and label position, label rotation, minor gridlines | **Open** |
| Date axes (`<c:dateAx>`) | **Open** |

### Labels, text and layout

| Gap | Status |
| --- | --- |
| Data labels (value, category, series name, percent) | Done — `DataLabels` and `ChartSpec::data_labels`. Positions are validated per chart kind because Excel *repairs away* the whole chart on an illegal position. |
| Title, axis and legend font size, bold, colour | **Open** |
| Manual plot-area and legend layout | **Open** |
| Legend overlay, legend entry deletion | **Open** |
| Chart-area fill and border | **Open** |

### Drawing

| Gap | Status |
| --- | --- |
| Only `twoCellAnchor` | Done — `Anchor` adds `oneCellAnchor` and `absoluteAnchor`; `drawing_part_with` emits them |
| Drawings hosting things other than charts | **Open** — out of scope unless a consumer asks |

### Template mode

| Gap | Status |
| --- | --- |
| Only data references and the chart title can be swapped | **Open** — axis titles and series names are the likely next swaps |

## Ground rules for new features

- Emit elements in schema order. Excel refuses a reordered chart silently.
- Validate what Excel would repair away (colours, ranges, label positions)
  and return a typed `ChartError` rather than emit a file that opens damaged.
- Every option gets a test that pins the emitted XML.
- Keep the dependency list at `thiserror`.
