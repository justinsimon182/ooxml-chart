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

Last, Excel formats a base chart through its Format object model, once per
value of each formatting option (fills, colours with brightness and
transparency, lines, dashes, compounds, arrowheads, shadows, glow, soft edges,
bevels, fonts, number formats). What a format adds to the shape and text
properties must appear in some gallery part, matched from the `spPr`/`txPr`
root so a format counts wherever the gallery shows it. Numbers, hex colours and
font names are free when the gallery has that attribute; defaults Excel spells
out, and exact equivalents (preset black for sRGB 000000, the alignment of an
unscaled shadow) are not differences.

The same is done for the chartEx kinds (all but the region map): Excel formats
each element (series, point, chart and plot area, title, legend, labels, axes,
gridlines, axis titles), and each element's shape properties must appear in
some gallery part. Their content is matched from the `spPr` root against every
gallery part, as both chart forms share one writer.

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

FORMATS_POWERSHELL = r"""param($out, $only = '.')
Add-Type -AssemblyName office
$xl = New-Object -ComObject Excel.Application
$xl.DisplayAlerts = $false
# Each format: base chart type (51 column, 4 line) and what Excel's Format object model does to it.
$formats = [ordered]@{
  'base_51'            = @(51, { })
  'base_4'             = @(4, { })
  'base_labels'        = @(51, { $c.SeriesCollection(1).HasDataLabels = $true })
  'fill_solid'         = @(51, { $c.SeriesCollection(1).Format.Fill.ForeColor.RGB = 255; $c.SeriesCollection(1).Format.Fill.Solid() })
  'fill_transparent'   = @(51, { $f = $c.SeriesCollection(1).Format.Fill; $f.Solid(); $f.ForeColor.RGB = 255; $f.Transparency = 0.5 })
  'fill_theme'         = @(51, { $f = $c.SeriesCollection(1).Format.Fill; $f.Solid(); $f.ForeColor.ObjectThemeColor = 6 })
  'fill_gradient'      = @(51, { $c.SeriesCollection(1).Format.Fill.TwoColorGradient(1, 1) })
  'fill_gradient_rad'  = @(51, { $c.SeriesCollection(1).Format.Fill.TwoColorGradient(7, 1) })
  'fill_preset'        = @(51, { $c.SeriesCollection(1).Format.Fill.PresetGradient(1, 1, 2) })
  'fill_pattern'       = @(51, { $c.SeriesCollection(1).Format.Fill.Patterned(5) })
  'fill_none'          = @(51, { $c.SeriesCollection(1).Format.Fill.Visible = 0 })
  'border_solid'       = @(51, { $l = $c.SeriesCollection(1).Format.Line; $l.Visible = -1; $l.ForeColor.RGB = 255; $l.Weight = 2.5 })
  'line_dash'          = @(4,  { $c.SeriesCollection(1).Format.Line.DashStyle = 4 })
  'line_round_dot'     = @(4,  { $c.SeriesCollection(1).Format.Line.DashStyle = 3 })
  'line_long_dash_dot' = @(4,  { $c.SeriesCollection(1).Format.Line.DashStyle = 8 })
  'line_compound'      = @(4,  { $l = $c.SeriesCollection(1).Format.Line; $l.Weight = 6; $l.Style = 3 })
  'line_arrow'         = @(4,  { $c.SeriesCollection(1).Format.Line.EndArrowheadStyle = 2 })
  'line_none'          = @(4,  { $c.SeriesCollection(1).Format.Line.Visible = 0 })
  'line_transparent'   = @(4,  { $l = $c.SeriesCollection(1).Format.Line; $l.ForeColor.RGB = 255; $l.Transparency = 0.4 })
  'shadow'             = @(51, { $c.SeriesCollection(1).Format.Shadow.Type = 21 })
  'glow'               = @(51, { $g = $c.SeriesCollection(1).Format.Glow; $g.Radius = 8; $g.Color.RGB = 255 })
  'soft_edge'          = @(51, { $c.SeriesCollection(1).Format.SoftEdge.Radius = 5 })
  'bevel'              = @(51, { $c.SeriesCollection(1).Format.ThreeD.BevelTopType = 3 })
  'font_title'         = @(51, { $c.HasTitle = $true; $c.ChartTitle.Text = 'T'; $f = $c.ChartTitle.Format.TextFrame2.TextRange.Font; $f.Bold = -1; $f.Italic = -1; $f.Size = 20; $f.Name = 'Arial'; $f.Fill.ForeColor.RGB = 255 })
  'font_underline'     = @(51, { $c.HasTitle = $true; $c.ChartTitle.Text = 'T'; $c.ChartTitle.Font.Underline = 2 })
  'font_strike'        = @(51, { $c.HasTitle = $true; $c.ChartTitle.Text = 'T'; $c.ChartTitle.Format.TextFrame2.TextRange.Font.Strike = 1 })
  'font_axis'          = @(51, { $f = $c.Axes(2).TickLabels.Font; $f.Bold = $true; $f.Size = 12; $f.Color = 255 })
  'font_legend'        = @(51, { $f = $c.Legend.Font; $f.Italic = $true; $f.Size = 11 })
  'numfmt_axis'        = @(51, { $c.Axes(2).TickLabels.NumberFormat = '0.0%' })
  'numfmt_labels'      = @(51, { $s = $c.SeriesCollection(1); $s.HasDataLabels = $true; $s.DataLabels().NumberFormat = '#,##0.00' })
  'chartarea_fill'     = @(51, { $c.ChartArea.Format.Fill.ForeColor.RGB = 15790320 })
  'chartarea_border'   = @(51, { $l = $c.ChartArea.Format.Line; $l.Visible = -1; $l.ForeColor.RGB = 255; $l.Weight = 2 })
  'chartarea_rounded'  = @(51, { $c.ChartArea.RoundedCorners = $true })
  'plotarea_fill'      = @(51, { $c.PlotArea.Format.Fill.ForeColor.RGB = 15790320 })
  'legend_fill'        = @(51, { $c.Legend.Format.Fill.ForeColor.RGB = 15790320; $c.Legend.Format.Line.ForeColor.RGB = 0 })
  'gridline_format'    = @(51, { $l = $c.Axes(2).MajorGridlines.Format.Line; $l.ForeColor.RGB = 255; $l.DashStyle = 4 })
  'axis_line'          = @(51, { $l = $c.Axes(1).Format.Line; $l.Visible = -1; $l.ForeColor.RGB = 255; $l.Weight = 2 })
  'point_fill'         = @(51, { $c.SeriesCollection(1).Points(2).Format.Fill.ForeColor.RGB = 65280 })
  'marker_fill'        = @(4,  { $s = $c.SeriesCollection(1); $s.MarkerStyle = 8; $s.MarkerSize = 9; $s.MarkerBackgroundColor = 255; $s.MarkerForegroundColor = 0 })
  'labels_fill'        = @(51, { $s = $c.SeriesCollection(1); $s.HasDataLabels = $true; $s.DataLabels().Format.Fill.ForeColor.RGB = 15790320 })
}
# Every value of each enumerated format.
function Add($name, $type, $code) { $formats[$name] = @($type, [scriptblock]::Create($code)) }
foreach ($i in 1..12) { Add "line_dash_$i" 4 "`$c.SeriesCollection(1).Format.Line.DashStyle = $i" }
foreach ($i in 1..5) { Add "line_style_$i" 4 "`$l = `$c.SeriesCollection(1).Format.Line; `$l.Weight = 6; `$l.Style = $i" }
foreach ($i in 2..6) { Add "line_arrow_end_$i" 4 "`$c.SeriesCollection(1).Format.Line.EndArrowheadStyle = $i" }
foreach ($i in 2..6) { Add "line_arrow_begin_$i" 4 "`$c.SeriesCollection(1).Format.Line.BeginArrowheadStyle = $i" }
foreach ($i in 2..13) { Add "bevel_$i" 51 "`$c.SeriesCollection(1).Format.ThreeD.BevelTopType = $i" }
foreach ($i in 1..62) { Add "fill_pattern_$i" 51 "`$c.SeriesCollection(1).Format.Fill.Patterned($i)" }
foreach ($i in 1..7) { Add "fill_gradient_style_$i" 51 "`$c.SeriesCollection(1).Format.Fill.TwoColorGradient($i, 1)" }
foreach ($i in 1..16) { Add "fill_theme_$i" 51 "`$f = `$c.SeriesCollection(1).Format.Fill; `$f.Solid(); `$f.ForeColor.ObjectThemeColor = $i; `$f.ForeColor.Brightness = -0.25" }
foreach ($i in 1..16) { Add "fill_theme_light_$i" 51 "`$f = `$c.SeriesCollection(1).Format.Fill; `$f.Solid(); `$f.ForeColor.ObjectThemeColor = $i; `$f.ForeColor.Brightness = 0.4" }
foreach ($i in 1..43) { Add "shadow_type_$i" 51 "`$c.SeriesCollection(1).Format.Shadow.Type = $i" }
foreach ($u in @(-4119, 4, 5)) { Add "font_underline_$([math]::Abs($u))" 51 "`$c.HasTitle = `$true; `$c.ChartTitle.Text = 'T'; `$c.ChartTitle.Font.Underline = $u" }
Add "font_superscript" 51 "`$c.HasTitle = `$true; `$c.ChartTitle.Text = 'T'; `$c.ChartTitle.Font.Superscript = `$true"
Add "font_subscript" 51 "`$c.HasTitle = `$true; `$c.ChartTitle.Text = 'T'; `$c.ChartTitle.Font.Subscript = `$true"
Add "font_spacing" 51 "`$c.HasTitle = `$true; `$c.ChartTitle.Text = 'T'; `$c.ChartTitle.Format.TextFrame2.TextRange.Font.Spacing = 3"
Add "font_caps" 51 "`$c.HasTitle = `$true; `$c.ChartTitle.Text = 'T'; `$c.ChartTitle.Format.TextFrame2.TextRange.Font.Allcaps = -1"
foreach ($name in $formats.Keys) {
  if ($name -notmatch $only) { continue }
  $type, $act = $formats[$name]
  try {
    $wb = $xl.Workbooks.Add(); $ws = $wb.Worksheets.Item(1)
    $ws.Range("A1:C1").Value2 = @("Cat", "North", "South")
    for ($r = 2; $r -le 6; $r++) { $ws.Cells.Item($r, 1).Value2 = "Q$($r-1)"; $ws.Cells.Item($r, 2).Value2 = 10 * $r; $ws.Cells.Item($r, 3).Value2 = 7 * $r + 3 }
    $ws.Range("A1:C6").Select() | Out-Null
    $c = $ws.Shapes.AddChart2(-1, $type, 200, 10, 400, 300).Chart
    & $act
    $wb.SaveAs("$out\$name.xlsx", 51); $wb.Close($false)
  } catch { "ERR $name $($_.Exception.Message)"; try { $wb.Close($false) } catch {} }
}
$xl.Quit()
"""

FMT_NS = {
    "http://schemas.openxmlformats.org/drawingml/2006/chart": "c",
    "http://schemas.openxmlformats.org/drawingml/2006/main": "a",
}
FMT_SKIP = {"extLst", "numCache", "strCache", "layout", "printSettings", "clrMapOvr", "userShapes", "axId", "crossAx"}
FMT_FREE = re.compile(r"-?[0-9.]+|[0-9A-Fa-f]{6}")
FMT_FREE_ATTRS = {"typeface", "formatCode", "lang", "altLang"}
# Values Excel writes out that are the schema's defaults.
FMT_DEFAULTS = {
    ("a:ln", "algn", "ctr"), ("a:ln", "cmpd", "sng"), ("a:headEnd", "type", "none"),
    ("a:tailEnd", "type", "none"), ("a:headEnd", "w", "med"), ("a:headEnd", "len", "med"),
    ("a:tailEnd", "w", "med"), ("a:tailEnd", "len", "med"), ("a:prstDash", "val", "solid"),
    ("a:gradFill", "flip", "none"), ("a:tileRect", "", ""), ("a:effectLst", "", ""),
    ("c:invertIfNegative", "val", "0"), ("c:bubble3D", "val", "0"), ("c:showLeaderLines", "val", "0"),
    ("a:round", "", ""), ("a:headEnd", "", ""), ("a:tailEnd", "", ""),
}


def fmt_items(data):
    out = set()

    def walk(e, path):
        ns, _, tag = e.tag[1:].partition("}")
        if ns not in FMT_NS or tag in FMT_SKIP:
            return
        here = f"{path}/{FMT_NS[ns]}:{tag}"
        attrib = e.attrib
        if tag == "prstClr" and e.get("val") == "black":
            # The preset colour black is exactly sRGB 000000.
            here, attrib = f"{path}/a:srgbClr", {"val": "000000"}
        elif tag == "scrgbClr" and all(e.get(k) == "0" for k in "rgb"):
            # Linear-RGB black is exactly sRGB 000000.
            here, attrib = f"{path}/a:srgbClr", {"val": "000000"}
        elif tag == "outerShdw" and not any(k in e.attrib for k in ("sx", "sy", "kx", "ky")):
            # Alignment anchors only scaling and skew; with neither it changes nothing.
            attrib = {k: v for k, v in e.attrib.items() if k != "algn"}
        out.add((here, "", ""))
        for k, v in attrib.items():
            out.add((here, k, v))
        if tag in ("spPr", "txPr", "rich"):
            # DrawingML is one type wherever it sits: match it from its root.
            here = "@spPr" if tag == "spPr" else "@text"
        for child in e:
            walk(child, here)

    walk(ET.fromstring(data), "")
    return out


def fmt_default(p, a, v):
    last = p.rsplit("/", 1)[-1]
    if not a and any(d[0] == last for d in FMT_DEFAULTS if d[1]):
        return True  # the bare element of a default-valued attribute
    if a in ("charset", "panose", "pitchFamily"):
        return True  # font metadata Excel looks up, not a choice
    if last == "a:spAutoFit" or (last == "a:bodyPr" and a):
        return True  # Excel's default text box settings
    return (last, a, v) in FMT_DEFAULTS


def format_parity(work, gallery_dir):
    script = os.path.join(work, "formats.ps1")
    open(script, "w").write(FORMATS_POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    # Values Excel itself refuses (patterns past its last, accounting underlines on a title).
    refused = [line for line in run.stdout.splitlines() if line.startswith("ERR")]
    print(f"{len(refused)} format values refused by Excel itself")

    gallery = []
    for f in glob.glob(gallery_dir + "/*.xml"):
        data = open(f, "rb").read()
        if b"<c:chartSpace" in data[:400]:
            gallery.append(fmt_items(data))
    union = set().union(*gallery)
    free_keys = {(p, a) for p, a, v in union if a and (a in FMT_FREE_ATTRS or FMT_FREE.fullmatch(v))}

    def chart(path):
        with zipfile.ZipFile(path) as z:
            return fmt_items(z.read("xl/charts/chart1.xml"))

    bases = {n: chart(f"{work}/base_{n}.xlsx") for n in ("51", "4", "labels")}
    matched = missed = 0
    for f in sorted(glob.glob(work + "/*.xlsx")):
        name = os.path.basename(f)[:-5]
        if name.startswith("base_"):
            continue
        labels = name.startswith(("numfmt_labels", "labels_"))
        base = bases["labels" if labels else "4" if name.startswith(("line_", "marker_")) else "51"]
        mine = chart(f)
        # Excel writes a series' label settings on the series; chart-wide ones match them.
        added = {
            (p.replace("/c:ser/c:dLbls", "/c:dLbls") if labels else p, a, v)
            for p, a, v in mine - base
            if labels or "/c:ser/c:dLbls" not in p
        }
        lacking = sorted(
            (p, a, v) for p, a, v in added - union
            if not fmt_default(p, a, v)
            and not (a and (a in FMT_FREE_ATTRS or FMT_FREE.fullmatch(v)) and (p, a) in free_keys)
        )
        if lacking:
            missed += 1
            print("MISS", name, "| needs", lacking[:6])
        else:
            matched += 1
    print(f"{matched} of {matched + missed} Excel formats matched by the gallery")
    return missed == 0 and matched >= 200


CHART_EX_FORMATS_POWERSHELL = r"""param($out, $only = '.')
$xl = New-Object -ComObject Excel.Application
$xl.DisplayAlerts = $false
# chartEx kinds: 117 treemap, 118 histogram, 119 waterfall, 120 sunburst, 121 box and whisker, 122 pareto, 123 funnel.
$types = @{ treemap = 117; histogram = 118; waterfall = 119; sunburst = 120; boxwhisker = 121; pareto = 122; funnel = 123 }
$formats = [ordered]@{
  'base'            = { }
  'series_fill'     = { $s = $c.FullSeriesCollection(1); $s.Format.Fill.ForeColor.RGB = 255; $s.Format.Line.ForeColor.RGB = 65280 }
  'point_fill'      = { $c.FullSeriesCollection(1).Points(2).Format.Fill.ForeColor.RGB = 255 }
  'chartarea_fill'  = { $c.ChartArea.Format.Fill.ForeColor.RGB = 15790320; $c.ChartArea.Format.Line.ForeColor.RGB = 0 }
  'plotarea_fill'   = { $c.PlotArea.Format.Fill.ForeColor.RGB = 15790320 }
  'title_text'      = { $c.HasTitle = $true; $c.ChartTitle.Text = 'T'; $f = $c.ChartTitle.Format.TextFrame2.TextRange.Font; $f.Bold = -1; $f.Size = 18; $f.Fill.ForeColor.RGB = 255 }
  'title_fill'      = { $c.HasTitle = $true; $c.ChartTitle.Text = 'T'; $c.ChartTitle.Format.Fill.ForeColor.RGB = 15790320 }
  'legend_fill'     = { $c.HasLegend = $true; $c.Legend.Format.Fill.ForeColor.RGB = 15790320; $c.Legend.Format.TextFrame2.TextRange.Font.Italic = -1 }
  'labels_format'   = { $s = $c.FullSeriesCollection(1); $s.HasDataLabels = $true; $d = $s.DataLabels(); $d.Format.Fill.ForeColor.RGB = 15790320; $d.Format.TextFrame2.TextRange.Font.Bold = -1 }
  'axis_line'       = { $l = $c.Axes(2).Format.Line; $l.Visible = -1; $l.ForeColor.RGB = 255; $l.Weight = 2 }
  'axis_text'       = { $f = $c.Axes(1).TickLabels.Font; $f.Bold = $true; $f.Color = 255 }
  'gridlines'       = { $l = $c.Axes(2).MajorGridlines.Format.Line; $l.ForeColor.RGB = 255; $l.DashStyle = 4 }
  'axis_title'      = { $a = $c.Axes(2); $a.HasTitle = $true; $a.AxisTitle.Text = 'V'; $a.AxisTitle.Format.TextFrame2.TextRange.Font.Bold = -1; $a.AxisTitle.Format.Fill.ForeColor.RGB = 15790320 }
}
foreach ($t in $types.Keys) {
  foreach ($name in $formats.Keys) {
    if ("$t`_$name" -notmatch $only) { continue }
    try {
      $wb = $xl.Workbooks.Add(); $ws = $wb.Worksheets.Item(1)
      $ws.Range("A1:B1").Value2 = @("Cat", "North")
      for ($r = 2; $r -le 6; $r++) { $ws.Cells.Item($r, 1).Value2 = "Q$($r-1)"; $ws.Cells.Item($r, 2).Value2 = 10 * $r - 25 }
      $ws.Range("A1:B6").Select() | Out-Null
      $c = $ws.Shapes.AddChart2(-1, $types[$t], 200, 10, 400, 300).Chart
      & $formats[$name]
      $wb.SaveAs("$out\$t`_$name.xlsx", 51); $wb.Close($false)
    } catch { "ERR $t`_$name $($_.Exception.Message)"; try { $wb.Close($false) } catch {} }
  }
}
$xl.Quit()
"""


def cx_items(data):
    out = set()
    names = dict(FMT_NS)
    names["http://schemas.microsoft.com/office/drawing/2014/chartex"] = "cx"
    skip = {"extLst", "numDim", "strDim", "data", "printSettings", "clrMapOvr", "layoutPr", "numFmt", "numCache", "strCache", "layout", "axId", "crossAx"}

    def walk(e, path):
        ns, _, tag = e.tag[1:].partition("}")
        if ns not in names or tag in skip:
            return
        here = f"{path}/{names[ns]}:{tag}"
        out.add((here, "", ""))
        for k, v in e.attrib.items():
            if k != "uniqueId":  # random on each save
                out.add((here, k, v))
        if tag in ("spPr", "txPr", "rich"):
            here = "@spPr" if tag == "spPr" else "@text"
        for child in e:
            walk(child, here)

    walk(ET.fromstring(data), "")
    return out


def chart_ex_format_parity(work, gallery_dir):
    script = os.path.join(work, "cx_formats.ps1")
    open(script, "w").write(CHART_EX_FORMATS_POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    # Elements Excel has no object for on a kind (axes on a treemap, say).
    refused = [line for line in run.stdout.splitlines() if line.startswith("ERR")]
    print(f"{len(refused)} chartEx formats refused by Excel itself")

    gallery = []
    for f in glob.glob(gallery_dir + "/*.xml"):
        data = open(f, "rb").read()
        if b"<cx:chartSpace" in data[:600] or b"<c:chartSpace" in data[:400]:
            gallery.append(cx_items(data))
    union = set().union(*gallery)
    free_keys = {(p, a) for p, a, v in union if a and (a in FMT_FREE_ATTRS or FMT_FREE.fullmatch(v))}

    def chart(path):
        with zipfile.ZipFile(path) as z:
            name = next(n for n in z.namelist() if n.startswith("xl/charts/chartEx") and n.endswith(".xml"))
            return cx_items(z.read(name))

    matched = missed = 0
    for f in sorted(glob.glob(work + "/*.xlsx")):
        name = os.path.basename(f)[:-5]
        kind, _, fmt = name.partition("_")
        if fmt == "base":
            continue
        lacking = sorted(
            (p, a, v) for p, a, v in chart(f) - chart(f"{work}/{kind}_base.xlsx") - union
            if not fmt_default(p, a, v)
            and not (a and (a in FMT_FREE_ATTRS or FMT_FREE.fullmatch(v)) and (p, a) in free_keys)
        )
        if lacking:
            missed += 1
            print("MISS", name, "| needs", lacking[:6])
        else:
            matched += 1
    print(f"{matched} of {matched + missed} Excel chartEx formats matched by the gallery")
    return missed == 0 and matched >= 60


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
    formats_ok = format_parity(work, sys.argv[1])
    for f in os.listdir(work):
        os.remove(os.path.join(work, f))
    chart_ex_ok = chart_ex_format_parity(work, sys.argv[1])
    for f in os.listdir(work):
        os.remove(os.path.join(work, f))
    os.rmdir(work)
    sys.exit(1 if missing or len(excel) < 81 or not elements_ok or not formats_ok or not chart_ex_ok else 0)


if __name__ == "__main__":
    main()
