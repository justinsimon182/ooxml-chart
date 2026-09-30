"""Validate every part the gallery example writes against the ECMA-376 schemas.

    cargo run --example gallery -- out
    python scripts/validate_gallery.py out scripts/schemas

The schema directory must hold `dml-chart.xsd` and `dml-spreadsheetDrawing.xsd`
and what they import. `scripts/schemas` vendors exactly that subset of the
transitional (ISO/IEC 29500-4) set; CI runs this script against it.
Requires `lxml`. Exits non-zero if any part is invalid, which is the signal
that Excel would repair or refuse it.
"""
import glob
import os
import sys

from lxml import etree

CHART = "http://schemas.openxmlformats.org/drawingml/2006/chart"
DRAWING = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"


def flatten_extensions(doc):
    """Keep only the first child of each `c:ext`.

    The schema lets an extension hold one foreign element, but Excel writes two
    in a data label (`c15:dlblFieldTable` then `c15:showDataLabelsRange`). The
    extra children are not schema content, so they are dropped before checking.
    """
    for ext in doc.iter("{%s}ext" % CHART):
        for extra in list(ext)[1:]:
            ext.remove(extra)


def main(parts_dir, schema_dir):
    schemas = {
        "{%s}chartSpace" % CHART: "dml-chart.xsd",
        "{%s}wsDr" % DRAWING: "dml-spreadsheetDrawing.xsd",
    }
    loaded = {}
    invalid = checked = 0
    for path in sorted(glob.glob(os.path.join(parts_dir, "*.xml"))):
        doc = etree.parse(path)
        flatten_extensions(doc)
        xsd = schemas.get(doc.getroot().tag)
        if xsd is None:
            print("SKIP   ", os.path.basename(path), doc.getroot().tag)
            continue
        if xsd not in loaded:
            loaded[xsd] = etree.XMLSchema(etree.parse(os.path.join(schema_dir, xsd)))
        schema = loaded[xsd]
        checked += 1
        if not schema.validate(doc):
            invalid += 1
            print("INVALID", os.path.basename(path))
            for error in list(schema.error_log)[:3]:
                print("   ", error.message[:240])
    print(f"{checked - invalid} of {checked} parts valid")
    return 1 if invalid or not checked else 0


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    sys.exit(main(sys.argv[1], sys.argv[2]))
