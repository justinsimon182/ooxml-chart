# Vendored schemas

The subset of the ECMA-376 / ISO/IEC 29500-4 (2016, transitional) XML schemas
that the chart and spreadsheet-drawing parts need. `validate_gallery.py` and
the `schema` CI job check every part the gallery example writes against them.

| File | Why it is here |
| --- | --- |
| `dml-chart.xsd`, `dml-spreadsheetDrawing.xsd` | the two part types the crate writes |
| `dml-chartDrawing.xsd`, `dml-main.xsd`, `dml-diagram.xsd`, `dml-lockedCanvas.xsd`, `dml-picture.xsd` | imported by the chart schema |
| `shared-commonSimpleTypes.xsd`, `shared-relationshipReference.xsd` | imported by the schemas above |

The files are unmodified copies of the publicly available ISO/IEC 29500-4:2016
schemas. They are for testing only and are excluded from the published crate.
