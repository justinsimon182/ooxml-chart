#!/usr/bin/env python3
"""Open every gallery part in a real Excel and report what survived.

Windows with desktop Excel only. Each gallery part is wrapped in its own tiny
.xlsx (sample data in Sheet1!A1:H8) so a rejected part is attributable, then
opened read-only through COM with alerts off. A chart part passes if Excel
loads exactly one chart; a drawing part passes if it loads every shape it
holds. Excel repairs silently when alerts are off, so a part that loses its
chart or shape shows up as a count mismatch.

Excel also saves each workbook back out, and the saved chart or drawing XML is
compared with what was written. Excel rewrites a lot (it adds defaults and
extension elements), so only what the original had and the saved part lacks is
reported: an element or attribute value that Excel dropped or changed. Those
are listed but do not fail the run unless --strict is given. Cached values and
quoting in references are ignored, since Excel refills and rewrites them.

Expected, harmless differences on the current gallery: `valAx/axPos` flipped
where the axis crosses at the maximum; `lineChart/marker` normalised;
`view3D/perspective` dropped where it is Excel's default of 30; a scatter or
bubble `xVal` turned from a number reference to a string one, because the
sample data's first column is text; and `dLbls` formatting that Excel moves
from the chart group onto each series. Anything else is worth a look.

    cargo run --example gallery -- out
    python scripts/excel_roundtrip.py out [--strict] [--verbose]

Only generated sample data is used; nothing leaves the machine.
"""
import base64
import collections
import glob
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG = "http://schemas.openxmlformats.org/package/2006/relationships"
HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
# A 1x1 transparent PNG.
PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="
)
GRAPHIC_FRAME = (
    '<xdr:twoCellAnchor><xdr:from><xdr:col>9</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>'
    '<xdr:to><xdr:col>18</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>20</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>'
    '<xdr:graphicFrame macro=""><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>'
    '<xdr:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></xdr:xfrm><a:graphic>'
    '<a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
    f'<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/>'
    '</a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor>'
)
WS_DRAWING = (
    f'{HEAD}<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" '
    f'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="{R}">%s</xdr:wsDr>'
)


def sheet_xml():
    rows = []
    for r in range(1, 9):
        cells = []
        for c, col in enumerate("ABCDEFGH"):
            if r == 1:
                cells.append(f'<c r="{col}1" t="inlineStr"><is><t>H{col}</t></is></c>')
            elif c == 0:
                cells.append(f'<c r="{col}{r}" t="inlineStr"><is><t>Cat{r}</t></is></c>')
            else:
                cells.append(f'<c r="{col}{r}"><v>{(r * 7 + c * 3) % 23 + 2}</v></c>')
        rows.append(f'<row r="{r}">{"".join(cells)}</row>')
    return (
        f'{HEAD}<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
        f'xmlns:r="{R}"><sheetData>{"".join(rows)}</sheetData><drawing r:id="rId1"/></worksheet>'
    )


def rels(*entries):
    body = "".join(f'<Relationship Id="{i}" Type="{R}/{t}" Target="{g}"/>' for i, t, g in entries)
    return f'{HEAD}<Relationships xmlns="{PKG}">{body}</Relationships>'


def build(path, drawing, drawing_rels, extra):
    """One workbook: Sheet1 + the given drawing part + its relationship targets."""
    overrides = {
        "/xl/workbook.xml": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
        "/xl/worksheets/sheet1.xml": "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml",
        "/xl/drawings/drawing1.xml": "application/vnd.openxmlformats-officedocument.drawing+xml",
    }
    if "xl/charts/chart1.xml" in extra:
        overrides["/xl/charts/chart1.xml"] = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
    types = "".join(f'<Override PartName="{p}" ContentType="{t}"/>' for p, t in overrides.items())
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr(
            "[Content_Types].xml",
            f'{HEAD}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Default Extension="xml" ContentType="application/xml"/>'
            f'<Default Extension="png" ContentType="image/png"/>{types}</Types>',
        )
        z.writestr("_rels/.rels", rels(("rId1", "officeDocument", "xl/workbook.xml")))
        z.writestr(
            "xl/workbook.xml",
            f'{HEAD}<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{R}">'
            '<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>',
        )
        z.writestr("xl/_rels/workbook.xml.rels", rels(("rId1", "worksheet", "worksheets/sheet1.xml")))
        z.writestr("xl/worksheets/sheet1.xml", sheet_xml())
        z.writestr("xl/worksheets/_rels/sheet1.xml.rels", rels(("rId1", "drawing", "../drawings/drawing1.xml")))
        z.writestr("xl/drawings/drawing1.xml", drawing)
        z.writestr("xl/drawings/_rels/drawing1.xml.rels", drawing_rels)
        for name, data in extra.items():
            z.writestr(name, data)


POWERSHELL = r"""
param($dir)
$xl = New-Object -ComObject Excel.Application
$xl.Visible = $false; $xl.DisplayAlerts = $false; $xl.AutomationSecurity = 3
foreach ($f in Get-ChildItem "$dir\*.xlsx") {
  try {
    $wb = $xl.Workbooks.Open($f.FullName, 0, $true)
    $ws = $wb.Worksheets.Item(1)
    "$($f.BaseName)|$($ws.ChartObjects().Count)|$($ws.Shapes.Count)"
    $wb.SaveAs("$dir\saved\$($f.BaseName).xlsx", 51)
    $wb.Close($false)
  } catch { "$($f.BaseName)|ERROR|$($_.Exception.Message)" }
}
$xl.Quit()
"""


# Excel refills these from the sheet, so what was cached is not compared.
CACHES = ("strCache", "numCache", "multiLvlStrCache")


def local(tag):
    return tag.rsplit("}", 1)[-1]


def signatures(data):
    """Every element as (path, sorted attributes, text), counted.

    Paths use local names and no positions, so a series moving from index 2 to
    index 3 is not a difference; namespaces are dropped so prefix choices do not
    matter. Attributes named `id` and relationship ids are Excel's to renumber.
    """
    counts = collections.Counter()

    def walk(element, path):
        here = f"{path}/{local(element.tag)}"
        attributes = tuple(
            sorted(
                (local(k), v)
                for k, v in element.attrib.items()
                if local(k) not in ("id", "embed") and not k.endswith("}id")
            )
        )
        text = (element.text or "").strip()
        if local(element.tag) == "f":
            # Excel writes 'Sheet1'!A1 as Sheet1!A1.
            text = text.replace("'", "")
        counts[(here, attributes, text)] += 1
        if local(element.tag) in CACHES:
            return
        for child in element:
            walk(child, here)

    walk(ET.fromstring(data), "")
    return counts


def saved_part(path, member):
    try:
        with zipfile.ZipFile(path) as z:
            return z.read(member)
    except (OSError, KeyError, zipfile.BadZipFile):
        return None


def dropped(original, saved):
    """What the original had that the saved part lacks, as readable lines."""
    saved_counts = signatures(saved)
    lost = signatures(original) - saved_counts
    # `a:ext` and friends are Excel's to fill in; a changed attribute shows as
    # one lost and one gained signature at the same path, which reads better
    # as a pair.
    gained = saved_counts - signatures(original)
    gained_at = collections.defaultdict(list)
    for (path, attributes, text), _ in gained.items():
        gained_at[path].append((attributes, text))
    lines = []
    for (path, attributes, text), n in sorted(lost.items()):
        shown = " ".join(f'{k}="{v}"' for k, v in attributes)
        item = f"{path} {shown}".strip() + (f" text={text!r}" if text else "")
        if path in gained_at:
            other = gained_at[path][0]
            now = " ".join(f'{k}="{v}"' for k, v in other[0])
            item += f"  ->  {now}".rstrip()
        else:
            item += "  (dropped)"
        lines.append(item + (f" x{n}" if n > 1 else ""))
    return lines


def main():
    flags = [a for a in sys.argv[1:] if a.startswith("--")]
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1 or any(f not in ("--strict", "--verbose") for f in flags):
        sys.exit("usage: excel_roundtrip.py <gallery dir> [--strict] [--verbose]")
    gallery = args[0]
    work = tempfile.mkdtemp(prefix="ooxml-chart-excel-")
    os.mkdir(os.path.join(work, "saved"))
    expected = {}
    column = open(os.path.join(gallery, "column_clustered.xml"), "rb").read()
    for part in sorted(glob.glob(os.path.join(gallery, "*.xml"))):
        name = os.path.basename(part)[:-4]
        data = open(part, "rb").read()
        out = os.path.join(work, name + ".xlsx")
        if name.startswith("drawing_"):
            drawing = data.decode("utf-8")
            if "<xdr:pic>" in drawing:
                extra = {"xl/media/image1.png": PNG}
                drawing_rels = rels(("rId2", "image", "../media/image1.png"))
                expected[name] = (0, 1)
            elif "<xdr:sp " in drawing:
                extra, drawing_rels = {}, rels()
                expected[name] = (0, 1)
            else:
                extra = {"xl/charts/chart1.xml": column}
                drawing_rels = rels(("rId1", "chart", "../charts/chart1.xml"))
                expected[name] = (1, 1)
            build(out, drawing, drawing_rels, extra)
        else:
            drawing = WS_DRAWING % GRAPHIC_FRAME
            build(out, drawing, rels(("rId1", "chart", "../charts/chart1.xml")), {"xl/charts/chart1.xml": data})
            expected[name] = (1, 1)
    script = os.path.join(work, "check.ps1")
    open(script, "w").write(POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    seen, bad = {}, []
    for line in run.stdout.splitlines():
        name, charts, shapes = (line.split("|") + ["", ""])[:3]
        seen[name] = (charts, shapes)
        if charts == "ERROR":
            bad.append(f"{name}: {shapes}")
        elif (int(charts), int(shapes)) != expected[name]:
            bad.append(f"{name}: expected charts/shapes {expected[name]}, Excel loaded ({charts}, {shapes})")
    bad += [f"{n}: Excel never reported it" for n in expected if n not in seen]
    for item in bad:
        print("FAIL", item)
    print(f"{len(expected) - len(bad)} of {len(expected)} parts loaded in Excel")

    changed = 0
    for name in sorted(seen):
        if name not in expected or seen[name][0] == "ERROR":
            continue
        member = "xl/drawings/drawing1.xml" if name.startswith("drawing_") else "xl/charts/chart1.xml"
        original = zipfile.ZipFile(os.path.join(work, name + ".xlsx")).read(member)
        after = saved_part(os.path.join(work, "saved", name + ".xlsx"), member)
        if after is None:
            print("NOSAVE", name)
            changed += 1
            continue
        lines = dropped(original, after)
        if lines:
            changed += 1
            print(f"DIFF {name}: {len(lines)} element(s) dropped or changed by Excel")
            for line in lines if "--verbose" in flags else lines[:3]:
                print("    " + line)
    print(f"{len(seen) - changed} of {len(seen)} parts saved back by Excel without losing anything")

    for root, dirs, files in os.walk(work, topdown=False):
        for f in files:
            os.remove(os.path.join(root, f))
        for d in dirs:
            os.rmdir(os.path.join(root, d))
    os.rmdir(work)
    sys.exit(1 if bad or (changed and "--strict" in flags) else 0)


if __name__ == "__main__":
    main()
