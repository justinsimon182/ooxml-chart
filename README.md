# ooxml-chart

Author OOXML (Excel) chart parts and their drawing anchors from a declarative
specification.

This crate emits **XML**. It does not open, read, write, or zip `.xlsx`
packages — bring your own package layer. Keeping that out is what makes this
crate small enough to depend on: the only dependency is `thiserror`.

## Install

```toml
[dependencies]
ooxml-chart = "0.1"
```

## A chart in ten lines

```rust
use ooxml_chart::{ChartKind, ChartSpec, Series, SeriesName};

let part = ChartSpec::new(ChartKind::ColumnClustered)
    .title("Monthly totals")
    .series(Series {
        name: SeriesName::Literal("Series A".to_string()),
        categories: Some("'Sheet1'!$A$3:$A$38".to_string()),
        values: "'Sheet1'!$C$3:$C$38".to_string(),
    })
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
    CellAnchor { col: 12, col_offset_emu: 0, row: 1, row_offset_emu: 0 },
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

Clustered and stacked bar and column, line with and without markers, and pie.
`ChartKind` is `#[non_exhaustive]`; more can be added without a breaking
change.

## What this deliberately does not do

- Read or write `.xlsx` files.
- Register parts in `[Content_Types].xml` or the workbook relationships. It
  hands you the content type and the relationship XML; wiring them in is yours.
- Reproduce every chart feature Excel has. It covers the kinds above, well.

## License

MIT OR Apache-2.0, at your option.
