# ooxml-chart

Author OOXML (Excel) chart parts and their drawing anchors from a declarative
specification.

This crate emits **XML**. It does not open, read, write, or zip `.xlsx`
packages — bring your own package layer. Keeping that out is what makes this
crate small enough to depend on: the only dependency is `thiserror`.

## Install

```toml
[dependencies]
ooxml-chart = "0.2"
```

## A chart in ten lines

```rust
use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

let part = ChartSpec::new(ChartKind::ColumnClustered)
    .title("Monthly totals")
    .series(
        Series::new(
            SeriesName::Literal("Series A".to_string()),
            "'Sheet1'!$C$3:$C$38",
        )
        .with_categories("'Sheet1'!$A$3:$A$38"),
    )
    .render()
    .expect("a chart");

// Store `part.xml` at `xl/charts/chart1.xml` and register
// `part.content_type` in `[Content_Types].xml`.
```

## Adding a chart to a family that already exists

Prefer template mode. Cloning a chart that already renders correctly and
swapping only its data references inherits every colour, font, axis format and
legend setting — none of which you then have to get right.

```rust
# use ooxml_chart::ChartSpec;
# let sibling: &[u8] = br#"<c:chartSpace><c:f>Old!$A$1</c:f></c:chartSpace>"#;
let part = ChartSpec::from_template(sibling)?
    .with_references(vec!["Sheet1!$J$3:$J$38".to_string()])
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

The chart title, each axis title and each literal series name can be swapped
too. Axes and series are addressed by their position in the template
(`axis_count`, `series_count`); a swap that has nothing to land on — an axis
with no title, a series named by a cell reference — is an error, never a guess.

```rust
# use ooxml_chart::ChartSpec;
# let sibling: &[u8] = br#"<c:chartSpace><c:plotArea><c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Old</c:v></c:tx></c:ser></c:plotArea></c:chartSpace>"#;
let part = ChartSpec::from_template(sibling)?
    .with_series_name(0, "Findings")
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

## Placing a chart

A `twoCellAnchor` chart has no stored width or height — its size is the
distance between two cell positions. So keeping a chart the same size across an
edit means recomputing where it ends:

```rust
use ooxml_chart::{two_cell_anchor, CellAnchor, RowMetrics, UnknownHeight};

let metrics = RowMetrics::excel_default()
    .measured(42, 50)          // your workbook's measured (points, pixels)
    .unknown(UnknownHeight::Refuse);

let anchor = two_cell_anchor(
    CellAnchor::new(12, 1),
    (2300, 920),               // pixels
    &metrics,
    |row| if row > 2 { Some(42) } else { None },
    |_| Some(64),
)?;
# Ok::<(), ooxml_chart::ChartError>(())
```

`RowMetrics` is configuration rather than a constant because how many pixels a
row occupies is a property of *your workbook*, not of the format. The obvious
rule — `floor(points * 1.2)` — is wrong often enough to matter, so
`UnknownHeight::Refuse` exists for callers who would rather fail than guess.

## Supported chart kinds

Clustered, stacked and percent-stacked bar and column; line with and without
markers; area, stacked area and percent-stacked area; scatter (markers, or
lines with markers); bubble; pie and doughnut; pie-of-pie and bar-of-pie; radar
and filled radar; stock (high-low-close, open-high-low-close, and with volume);
surface and contour, solid or wireframe. Bar, column, line, area and pie kinds
also draw in 3-D with `view_3d`. Column, line, area, scatter and bubble kinds
combine on one chart, optionally with a second value axis.
`ChartKind` is `#[non_exhaustive]`; more can be added without a breaking
change.

## Formatting

```rust
use ooxml_chart::{
    Axis, ChartKind, ChartSpec, DataLabelPosition, DataLabels, MarkerSymbol, Series, SeriesName,
};

let part = ChartSpec::new(ChartKind::LineMarkers)
    .value_axis(Axis::default().title("Revenue").number_format("#,##0").min(0.0))
    .data_labels(DataLabels::values().at(DataLabelPosition::Above))
    .series(
        Series::new(SeriesName::Literal("North".to_string()), "'Sheet1'!$B$2:$B$5")
            .with_categories("'Sheet1'!$A$2:$A$5")
            .with_color("8E0DD1")
            .with_line_width(2.25)
            .with_marker(MarkerSymbol::Circle, 7)
            // Lets previews that do not recalculate draw the plot.
            .with_cached_values(vec![4.0, 7.5, 6.0, 9.25]),
    )
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

Settings Excel would repair away â€” a malformed colour, an inverted axis range,
a label position the chart kind does not allow â€” are refused with a typed
`ChartError` rather than written.

`Series`, `Axis` and `DataLabels` are `#[non_exhaustive]`: build them with
`Series::new` / `Axis::default()` and the builder methods.

Beyond the above: `TextStyle` for fonts, `AreaStyle` for chart and plot area
fill and border, `PointFormat` for one slice, bar or marker, `PointLabel` to
hide, reword, move or restyle one data label, `Trendline`, `ErrorBars`, `Layout` for the plot area and legend, log and
date axes,
tick and label options.

## Combination charts

```rust
use ooxml_chart::{Axis, ChartKind, ChartSpec, Plot, Series, SeriesName};

let part = ChartSpec::new(ChartKind::ColumnClustered)
    .series(
        Series::new(SeriesName::Literal("Revenue".into()), "'Sheet1'!$B$2:$B$5")
            .with_categories("'Sheet1'!$A$2:$A$5"),
    )
    .plot(
        Plot::new(ChartKind::LineMarkers)
            .series(
                Series::new(SeriesName::Literal("Margin".into()), "'Sheet1'!$C$2:$C$5")
                    .with_categories("'Sheet1'!$A$2:$A$5"),
            )
            .on_secondary_axis(),
    )
    .secondary_value_axis(Axis::default().title("Margin").number_format("0%"))
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

## Stock, 3-D, pie-of-pie and surface charts

Stock charts take their series in a fixed order, and a volume column can share
the chart through a second plot:

```rust
use ooxml_chart::{ChartKind, ChartSpec, Plot, Series, SeriesName};

let s = |name: &str, column: &str| {
    Series::new(SeriesName::Literal(name.into()), format!("'Sheet1'!${column}$2:${column}$30"))
        .with_categories("'Sheet1'!$A$2:$A$30")
};
let part = ChartSpec::new(ChartKind::ColumnClustered)
    .series(s("Volume", "F"))
    .plot(
        Plot::new(ChartKind::StockOpenHighLowClose)
            .series(s("Open", "B"))
            .series(s("High", "C"))
            .series(s("Low", "D"))
            .series(s("Close", "E"))
            .on_secondary_axis(),
    )
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

A 3-D chart is an ordinary chart plus a view. Anything unset takes Excel's own
default for the kind:

```rust
use ooxml_chart::{BarShape, ChartKind, ChartSpec, Series, SeriesName, View3D};

let part = ChartSpec::new(ChartKind::ColumnClustered)
    .view_3d(View3D::new().rotation_x(20).rotation_y(30).bar_shape(BarShape::Cylinder))
    .series(
        Series::new(SeriesName::Literal("Sales".into()), "'Sheet1'!$B$2:$B$5")
            .with_categories("'Sheet1'!$A$2:$A$5"),
    )
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

Pie-of-pie and bar-of-pie move some points into a second plot:

```rust
use ooxml_chart::{ChartKind, ChartSpec, OfPie, OfPieSplit, Series, SeriesName};

let part = ChartSpec::new(ChartKind::PieOfPie)
    .of_pie(OfPie::new().split(OfPieSplit::LastPoints(2)).second_size(60))
    .series(
        Series::new(SeriesName::Literal("Share".into()), "'Sheet1'!$B$2:$B$8")
            .with_categories("'Sheet1'!$A$2:$A$8"),
    )
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

A surface colours its value bands rather than its series, so its series carry
no colour of their own:

```rust
use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

let s = |name: &str, column: &str| {
    Series::new(SeriesName::Literal(name.into()), format!("'Sheet1'!${column}$2:${column}$6"))
        .with_categories("'Sheet1'!$A$2:$A$6")
};
let part = ChartSpec::new(ChartKind::Surface)
    .surface_bands(["0090B2", "28EAE4", "8E0DD1"])
    .series(s("North", "B"))
    .series(s("South", "C"))
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

An axis can show its values in thousands or millions, with a caption of your
own wording:

```rust
use ooxml_chart::{Axis, DisplayUnit};

let axis = Axis::default()
    .display_units(DisplayUnit::Millions)
    .display_units_caption("USD millions");
```

## Pictures and text boxes beside a chart

A drawing can host a picture or a text box as well as charts. The image bytes
are yours to put in the package; the drawing only points at them:

```rust
use ooxml_chart::{
    drawing_part_objects, drawing_relationships_typed, Anchor, DrawingObject, GraphicFrame,
    Picture, TextBox, CHART_RELATIONSHIP_TYPE, IMAGE_RELATIONSHIP_TYPE,
};

let frame = GraphicFrame::new(2, "Chart 1", "rId1");
let at = |x_emu| Anchor::absolute(x_emu, 0, 3_000_000, 2_000_000);
let drawing = drawing_part_objects(&[
    (at(0), DrawingObject::Chart(frame)),
    (at(3_000_000), DrawingObject::Picture(Picture::new(3, "Logo", "rId2").description("Company logo"))),
    (at(6_000_000), DrawingObject::TextBox(TextBox::new(4, "Source", "Source: field survey"))),
]);
let rels = drawing_relationships_typed(&[
    ("rId1".into(), CHART_RELATIONSHIP_TYPE, "../charts/chart1.xml".into()),
    ("rId2".into(), IMAGE_RELATIONSHIP_TYPE, "../media/image1.png".into()),
]);
assert!(!drawing.is_empty() && !rels.is_empty());
```

## Lines, bars and flags on a chart

Drop lines, high-low lines, up/down bars and series lines are chart-wide
switches, refused on a kind that cannot carry them. So are the chart's style,
language, rounded corners and date system:

```rust
use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

let s = |name: &str, column: &str| {
    Series::new(SeriesName::Literal(name.into()), format!("'Sheet1'!${column}$2:${column}$30"))
        .with_categories("'Sheet1'!$A$2:$A$30")
};
let part = ChartSpec::new(ChartKind::Line)
    .series(s("Open", "B"))
    .series(s("Close", "C"))
    .high_low_lines(true)
    .up_down_bars("0090B2", "8E0DD1")
    .language("en-US")
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

Categories that nest, such as a year column beside a month column, take a
multi-column range. Excel reads the labels from the sheet, so no cache goes with
them:

```rust
use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

let part = ChartSpec::new(ChartKind::ColumnClustered)
    .series(
        Series::new(SeriesName::Literal("Sales".into()), "'Sheet1'!$C$2:$C$13")
            .with_multi_level_categories("'Sheet1'!$A$2:$B$13"),
    )
    .render()?;
# Ok::<(), ooxml_chart::ChartError>(())
```

## A chart on its own sheet

A chartsheet holds one chart and no cells. The chart part is the one above; the
rest is a wrapper and the lines that register it:

```rust
use ooxml_chart::{
    chartsheet_content_types_override, chartsheet_drawing_part, chartsheet_part,
    workbook_chartsheet_relationship, workbook_sheet_element, GraphicFrame,
};

let frame = GraphicFrame::new(2, "Chart 1", "rId1");
let drawing = chartsheet_drawing_part(&frame);      // xl/drawings/drawing1.xml
let sheet = chartsheet_part("rId1");                // xl/chartsheets/sheet1.xml
let content_type = chartsheet_content_types_override("/xl/chartsheets/sheet1.xml");
let workbook_rel = workbook_chartsheet_relationship("rId2", "chartsheets/sheet1.xml");
let sheet_entry = workbook_sheet_element("Sales chart", 2, "rId2")?;
# Ok::<(), ooxml_chart::ChartError>(())
```

## Declaring a chart as data

With the `serde` feature every spec type implements `Serialize` and
`Deserialize`, so a chart can live in JSON, TOML or YAML instead of code. Only
`kind` and each series' `name` and `values` are required; everything else
takes the Excel default. A misspelt field is an error, not silently ignored.

```toml
[dependencies]
ooxml-chart = { version = "0.2", features = ["serde"] }
```

```json
{
  "kind": "column_clustered",
  "title": "Revenue",
  "series": [
    { "name": { "literal": "North" }, "values": "'Sheet1'!$B$2:$B$5",
      "categories": "'Sheet1'!$A$2:$A$5", "color": "8E0DD1" }
  ]
}
```

A parsed spec is validated when rendered, exactly like one built in code.

Anchors other than `twoCellAnchor` are available through `Anchor` and
`drawing_part_with`. Pictures and text boxes can share a drawing with charts
through `DrawingObject` and `drawing_part_objects`.

## What this deliberately does not do

- Read or write `.xlsx` files.
- Register parts in `[Content_Types].xml` or the workbook relationships. It
  hands you the content type and the relationship XML; wiring them in is yours.
- Reproduce every chart feature Excel has. See [ROADMAP.md](ROADMAP.md) for
  what is covered and how it is verified.

## License

MIT OR Apache-2.0, at your option.
