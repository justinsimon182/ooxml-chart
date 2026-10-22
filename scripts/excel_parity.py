"""Check the gallery against every chart type Excel offers.

Excel (through COM, Windows only) draws one chart of each `XlChartType` over
sample data and saves it. Each saved chart, and each gallery part, is reduced
to a signature: the plot element and the attributes that tell the types apart
(direction, grouping, bar shape, markers, smoothing, explosion, 3-D bubbles,
the chartEx layout). A type is covered when some gallery part has its
signature. Excel's region map sends the sample category names (Q1-Q5) to Bing.

Then Excel applies each of its chart elements (`Chart.SetElement`, every
`MsoChartElementType`) to a fitting base chart. What the element adds over the
bare chart, by element path and `val`, must all appear in the gallery (copies
Excel writes on each series aside, and free numbers matched by path); what it
removes must be absent from some gallery part.

    cargo run --example gallery -- out
    python scripts/excel_parity.py out
"""
import glob
import os
import re
import subprocess
import sys
import tempfile
import zipfile
import xml.etree.ElementTree as ET
C = "{http://schemas.openxmlformats.org/drawingml/2006/chart}"
A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
CX = "{http://schemas.microsoft.com/office/drawing/2014/chartex}"
def val(e, tag):
    x = e.find(C + tag)
    return None if x is None else x.get("val", "1")
def sig(data):
    root = ET.fromstring(data)
    if root.tag == CX + "chartSpace":
        return ("cx",) + tuple(sorted({s.get("layoutId") for s in root.iter(CX + "series")}))
    pa = root.find(f"{C}chart/{C}plotArea")
    out = []
    for plot in pa:
        name = plot.tag[len(C):]
        if not name.endswith("Chart"):
            continue
        attrs = [name]
        for t in ("barDir", "grouping", "shape", "scatterStyle", "radarStyle", "ofPieType", "wireframe"):
            v = val(plot, t)
            if t == "grouping" and v == "standard" and name in ("lineChart", "line3DChart", "area3DChart", "areaChart"):
                v = None  # standard is the plain kind
            if t == "shape" and v is None and name == "bar3DChart":
                v = "box"  # the spec default
            if v is not None:
                attrs.append(f"{t}={v}")
        sers = plot.findall(C + "ser")
        if name in ("lineChart", "scatterChart", "radarChart"):
            marks = any((s.find(f"{C}marker/{C}symbol") is None) or val(s.find(C + "marker"), "symbol") != "none" for s in sers)
            if name == "radarChart" and val(plot, "radarStyle") == "filled":
                marks = False
            attrs.append("markers" if marks else "nomarkers")
        if name == "scatterChart":
            nolines = all(s.find(f"{C}spPr/{A}ln/{A}noFill") is not None for s in sers)
            smooth = any(val(s, "smooth") == "1" for s in sers)
            attrs.append("nolines" if nolines else ("smooth" if smooth else "lines"))
        if name == "bubbleChart" and (val(plot, "bubble3D") == "1" or any(val(s, "bubble3D") == "1" for s in sers)):
            attrs.append("bubble3D")
        if name in ("pieChart", "pie3DChart", "doughnutChart") and any(int(s.find(C+"explosion").get("val")) > 0 for s in sers if s.find(C+"explosion") is not None):
            attrs.append("exploded")
        if name == "bar3DChart" and attrs.count("grouping=standard"):
            pass
        out.append(" ".join(attrs))
    return tuple(sorted(out))

POWERSHELL = r"""param($out)
$types = [ordered]@{
 xlColumnClustered=51; xlColumnStacked=52; xlColumnStacked100=53; xl3DColumnClustered=54; xl3DColumnStacked=55; xl3DColumnStacked100=56; xl3DColumn=-4100
 xlBarClustered=57; xlBarStacked=58; xlBarStacked100=59; xl3DBarClustered=60; xl3DBarStacked=61; xl3DBarStacked100=62
 xlLine=4; xlLineStacked=63; xlLineStacked100=64; xlLineMarkers=65; xlLineMarkersStacked=66; xlLineMarkersStacked100=67; xl3DLine=-4101
 xlPie=5; xl3DPie=-4102; xlPieExploded=69; xl3DPieExploded=70; xlPieOfPie=68; xlBarOfPie=71
 xlXYScatter=-4169; xlXYScatterSmooth=72; xlXYScatterSmoothNoMarkers=73; xlXYScatterLines=74; xlXYScatterLinesNoMarkers=75
 xlArea=1; xlAreaStacked=76; xlAreaStacked100=77; xl3DArea=-4098; xl3DAreaStacked=78; xl3DAreaStacked100=79
 xlDoughnut=-4120; xlDoughnutExploded=80; xlRadar=-4151; xlRadarMarkers=81; xlRadarFilled=82
 xlSurface=83; xlSurfaceWireframe=84; xlSurfaceTopView=85; xlSurfaceTopViewWireframe=86
 xlBubble=15; xlBubble3DEffect=87; xlStockHLC=88; xlStockOHLC=89; xlStockVHLC=90; xlStockVOHLC=91
 xlCylinderColClustered=92; xlCylinderColStacked=93; xlCylinderColStacked100=94; xlCylinderBarClustered=95; xlCylinderBarStacked=96; xlCylinderBarStacked100=97; xlCylinderCol=98
 xlConeColClustered=99; xlConeColStacked=100; xlConeColStacked100=101; xlConeBarClustered=102; xlConeBarStacked=103; xlConeBarStacked100=104; xlConeCol=105
 xlPyramidColClustered=106; xlPyramidColStacked=107; xlPyramidColStacked100=108; xlPyramidBarClustered=109; xlPyramidBarStacked=110; xlPyramidBarStacked100=111; xlPyramidCol=112
 xlTreemap=117; xlHistogram=118; xlWaterfall=119; xlSunburst=120; xlBoxwhisker=121; xlPareto=122; xlFunnel=123; xlRegionMap=140
}
$xl = New-Object -ComObject Excel.Application
$xl.DisplayAlerts = $false
foreach ($k in $types.Keys) {
  try {
    $wb = $xl.Workbooks.Add(); $ws = $wb.Worksheets.Item(1)
    $ws.Range("A1:F1").Value2 = @("Cat","Vol","Open","High","Low","Close")
    for ($r = 2; $r -le 6; $r++) { $ws.Cells.Item($r,1).Value2 = "Q$($r-1)"; $ws.Cells.Item($r,2).Value2 = 100*$r; $ws.Cells.Item($r,3).Value2 = 10+$r; $ws.Cells.Item($r,4).Value2 = 20+$r; $ws.Cells.Item($r,5).Value2 = 5+$r; $ws.Cells.Item($r,6).Value2 = 12+$r }
    $src = if ($k -eq "xlStockVHLC") { "A1:B6,D1:F6" } elseif ($k -like 'xlStockV*') { "A1:F6" } elseif ($k -like 'xlStockO*') { "A1:A6,C1:F6" } elseif ($k -like 'xlStock*') { "A1:A6,D1:F6" } else { "A1:C6" }
    $ws.Range($src).Select() | Out-Null
    $shape = $ws.Shapes.AddChart2(-1, $types[$k], 200, 10, 400, 300)
    $shape.Chart.SetSourceData($ws.Range($src))
    $wb.SaveAs("$out\$k.xlsx", 51); $wb.Close($false)
    "OK $k"
  } catch { "ERR $k $($_.Exception.Message)"; try { $wb.Close($false) } catch {} }
}
$xl.Quit()
"""


ELEMENTS_POWERSHELL = r"""param($out, $only = '.')
Add-Type -AssemblyName office
$xl = New-Object -ComObject Excel.Application
$xl.DisplayAlerts = $false
# Base chart per element family: 51 column, 65 line with markers, 58 stacked bar, 54 3-D column.
function Base($name) {
  if ($name -match 'Trendline') { return 5100 }
  if ($name -match 'UpDownBars|HiLo|DropLine|DropHiLo|LineNone') { return 65 }
  if ($name -match 'SeriesLine') { return 58 }
  if ($name -match 'DataLabel(Left|Right|Top|Bottom)$') { return 65 }
  if ($name -match 'DataLabelBestFit') { return 5 }
  if ($name -match 'CategoryAxis(Thousands|Millions|Billions|LogScale)') { return -4169 }
  if ($name -match 'SeriesAxis|Wall|Floor') { return -4100 }
  return 51
}
$names = [Enum]::GetNames([Microsoft.Office.Core.MsoChartElementType]) | Select-Object -Unique | Where-Object { $_ -match $only }
$bases = @{}
foreach ($n in $names) { $bases[(Base $n)] = 1 }
$jobs = @()
foreach ($b in $bases.Keys) { $jobs += ,@("base_$b", $b, $null) }
foreach ($n in $names) { $jobs += ,@($n, (Base $n), $n) }
foreach ($j in $jobs) {
  $name, $type, $el = $j
  try {
    $wb = $xl.Workbooks.Add(); $ws = $wb.Worksheets.Item(1)
    $ws.Range("A1:C1").Value2 = @("Cat", "North", "South")
    for ($r = 2; $r -le 6; $r++) { $ws.Cells.Item($r, 1).Value2 = "Q$($r-1)"; $ws.Cells.Item($r, 2).Value2 = 10 * $r; $ws.Cells.Item($r, 3).Value2 = 7 * $r + 3 }
    # 5100: a column chart of one series, which a trendline element needs.
    $src = if ($type -eq 5100) { "A1:B6" } else { "A1:C6" }
    if ($type -eq 5100) { $type = 51 }
    $ws.Range($src).Select() | Out-Null
    $chart = $ws.Shapes.AddChart2(-1, $type, 200, 10, 400, 300).Chart
    if ($el -and $el -match 'Secondary') { $chart.SeriesCollection(2).AxisGroup = 2 }
    if ($el) { $chart.SetElement([int][Microsoft.Office.Core.MsoChartElementType]::$el) }
    $wb.SaveAs("$out\$name.xlsx", 51); $wb.Close($false)
  } catch { "ERR $name $($_.Exception.Message)"; try { $wb.Close($false) } catch {} }
}
$xl.Quit()
"""

SKIP = {"spPr", "txPr", "extLst", "numCache", "strCache", "layout", "printSettings", "clrMapOvr", "userShapes"}


def base_of(name):
    if "Trendline" in name: return 5100
    if re.search(r"UpDownBars|HiLo|DropLine|DropHiLo", name): return 65
    if "LineNone" in name: return 65
    if "SeriesLine" in name: return 58
    if re.search(r"DataLabel(Left|Right|Top|Bottom)$", name): return 65
    if "DataLabelBestFit" in name: return 5
    if re.search(r"CategoryAxis(Thousands|Millions|Billions|LogScale)", name): return -4169
    if re.search(r"SeriesAxis|Wall|Floor", name): return -4100
    return 51


def items(data):
    out = set()

    def walk(e, path):
        if not e.tag.startswith(C):
            return
        tag = e.tag[len(C):]
        if tag in SKIP:
            return
        here = f"{path}/{tag}"
        v = None if tag in ("axId", "crossAx") else e.get("val")
        out.add((here, v))
        if tag == "rich":
            return
        for child in e:
            walk(child, here)

    walk(ET.fromstring(data), "")
    return out




def element_parity(work, gallery_dir):
    script = os.path.join(work, "elements.ps1")
    open(script, "w").write(ELEMENTS_POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    errors = [line for line in run.stdout.splitlines() if line.startswith("ERR")]
    for line in errors:
        print(line)

    def chart(path):
        with zipfile.ZipFile(path) as z:
            return items(z.read("xl/charts/chart1.xml"))

    gallery = []
    for f in glob.glob(gallery_dir + "/*.xml"):
        data = open(f, "rb").read()
        if b"<c:chartSpace" in data[:400]:
            gallery.append(items(data))
    union = set().union(*gallery)
    numeric = re.compile(r"-?[0-9.]+")
    numeric_paths = {p for p, v in union if v is not None and numeric.fullmatch(v)}
    bases = {int(os.path.basename(f)[5:-5]): chart(f) for f in glob.glob(work + "/base_*.xlsx")}
    matched = missed = 0
    for f in sorted(glob.glob(work + "/mso*.xlsx")):
        name = os.path.basename(f)[:-5]
        mine = chart(f)
        base = bases[base_of(name)]
        added = {(p, v) for p, v in mine - base if "/ser/dLbls" not in p}
        removed = {p for p, _ in base - mine} - {p for p, _ in mine}
        lacking = sorted(
            (p, v) for p, v in added - union
            if not (v is not None and numeric.fullmatch(v) and p in numeric_paths)
        )
        gone = all(any(p not in {q for q, _ in s} for s in gallery) for p in removed)
        if lacking or not gone:
            missed += 1
            print("MISS", name, "| needs", lacking, "| removes", sorted(removed))
        else:
            matched += 1
    print(f"{matched} of {matched + missed} Excel chart elements matched by the gallery")
    # An element Excel itself refuses on every base is no feature to match.
    return missed == 0


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: excel_parity.py <gallery dir>")
    work = tempfile.mkdtemp(prefix="ooxml-chart-parity-")
    script = os.path.join(work, "types.ps1")
    open(script, "w").write(POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    for line in run.stdout.splitlines():
        if line.startswith("ERR"):
            print(line)
    excel = {}
    for f in sorted(glob.glob(work + "/*.xlsx")):
        z = zipfile.ZipFile(f)
        part = [n for n in z.namelist() if re.match(r"xl/charts/chart(Ex)?\d+\.xml$", n)][0]
        excel[os.path.basename(f)[:-5]] = sig(z.read(part))
        z.close()
    ours = {}
    for f in sorted(glob.glob(sys.argv[1] + "/*.xml")):
        data = open(f, "rb").read()
        if b"chartSpace" in data[:400]:
            ours.setdefault(sig(data), []).append(os.path.basename(f)[:-4])
    missing = [k for k, s in excel.items() if s not in ours]
    for k, s in excel.items():
        print("OK  " if s in ours else "MISS", k, "|", "; ".join(s), "|", ours.get(s, [""])[0])
    print(f"{len(excel) - len(missing)} of {len(excel)} Excel chart types matched by a gallery part")
    for f in os.listdir(work):
        os.remove(os.path.join(work, f))
    elements_ok = element_parity(work, sys.argv[1])
    for f in os.listdir(work):
        os.remove(os.path.join(work, f))
    os.rmdir(work)
    sys.exit(1 if missing or len(excel) < 81 or not elements_ok else 0)


if __name__ == "__main__":
    main()
