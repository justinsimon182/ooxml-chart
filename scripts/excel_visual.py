"""Compare how Excel draws this crate's charts with how it draws its own.

Usage: excel_visual.py <dir from `cargo run --example visual -- <dir>`> <gallery dir> [--keep]

The gallery dir (from `cargo run --example gallery`) holds the chartEx drawing.

For each chart type the example writes (a plain spec with `excel_look`), this
builds a workbook holding the part over Sheet1!A1:C5, opens it in Excel, and
has Excel insert its own chart of that type over the same cells at the same
size. Both are exported as PNG and compared pixel by pixel: a pixel differs
when any channel is off by more than 48 of 255 (anti-aliasing stays under
that). A type passes when under 1% of its pixels differ.

Windows with Excel only. Exit status 1 when any type fails.
"""

import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import excel_roundtrip as rt  # noqa: E402

LIMIT = 1.0  # percent of pixels

POWERSHELL = r"""
param($dir)
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System; using System.Drawing; using System.Drawing.Imaging; using System.Runtime.InteropServices;
public static class PixelDiff {
  public static double Percent(string a, string b) {
    using (var x = new Bitmap(a)) using (var y = new Bitmap(b)) {
      if (x.Width != y.Width || x.Height != y.Height) return 100;
      var r = new Rectangle(0, 0, x.Width, x.Height);
      var dx = x.LockBits(r, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
      var dy = y.LockBits(r, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
      int n = dx.Stride * x.Height; var px = new byte[n]; var py = new byte[n];
      Marshal.Copy(dx.Scan0, px, 0, n); Marshal.Copy(dy.Scan0, py, 0, n);
      x.UnlockBits(dx); y.UnlockBits(dy);
      long bad = 0;
      for (int i = 0; i < n; i += 4)
        if (Math.Abs(px[i] - py[i]) > 48 || Math.Abs(px[i+1] - py[i+1]) > 48 || Math.Abs(px[i+2] - py[i+2]) > 48) bad++;
      return 100.0 * bad / (n / 4);
    }
  }
}
'@
$xl = New-Object -ComObject Excel.Application
$xl.Visible = $false; $xl.DisplayAlerts = $false; $xl.AutomationSecurity = 3
foreach ($line in Get-Content "$dir\kinds.txt") {
  $name, $type, $range, $post = $line -split ' '
  try {
    $wb = $xl.Workbooks.Open("$dir\$name.xlsx")
    $ws = $wb.Worksheets.Item(1)
    $o = $ws.ChartObjects(1)
    $o.Chart.Export("$dir\$name.ours.png") | Out-Null
    $ws.Range($range).Select() | Out-Null
    $e = $ws.Shapes.AddChart2(-1, [int]$type, $o.Left, $o.Top, $o.Width, $o.Height).Chart
    # A square range would otherwise be charted by rows.
    if ($name -notlike 'cx_*') { $e.SetSourceData($ws.Range($range), 2) }
    # `s2=<type>[,sec]` turns the second series into another type, on the right axis with `sec`.
    if ($post) {
      $opts = ($post -replace '^s2=', '') -split ','
      $s = $e.SeriesCollection(2)
      $s.ChartType = [int]$opts[0]
      if ($opts -contains 'sec') { $s.AxisGroup = 2 }
    }
    $e.Export("$dir\$name.excel.png") | Out-Null
    $wb.Close($false)
    "$name|$([PixelDiff]::Percent("$dir\$name.ours.png", "$dir\$name.excel.png"))"
  } catch { "$name|ERROR $($_.Exception.Message)" }
}
$xl.Quit()
"""


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        sys.exit("usage: excel_visual.py <parts dir> <gallery dir> [--keep]")
    parts, gallery = args[0], args[1]
    work = tempfile.mkdtemp(prefix="ooxml-chart-visual-")
    shutil.copy(os.path.join(parts, "kinds.txt"), work)
    for line in open(os.path.join(parts, "kinds.txt")):
        name = line.split()[0]
        data = open(os.path.join(parts, name + ".xml"), "rb").read()
        if name.startswith("cx_"):
            read = lambda suffix: open(os.path.join(parts, name + suffix), "rb").read()
            rt.build_chart_ex(
                os.path.join(work, name + ".xlsx"), data, gallery,
                read(".style.xml"), read(".colors.xml"),
            )
            continue
        rt.build(
            os.path.join(work, name + ".xlsx"),
            rt.WS_DRAWING % rt.GRAPHIC_FRAME,
            rt.rels(("rId1", "chart", "../charts/chart1.xml")),
            {"xl/charts/chart1.xml": data},
        )
    script = os.path.join(work, "visual.ps1")
    open(script, "w").write(POWERSHELL)
    run = subprocess.run(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script, work],
        capture_output=True, text=True,
    )
    passed = failed = 0
    for line in run.stdout.splitlines():
        name, _, result = line.partition("|")
        if not result:
            continue
        try:
            ok = float(result) < LIMIT
        except ValueError:
            ok = False
        passed += ok
        failed += not ok
        print(f"{'OK  ' if ok else 'DIFF'} {name}: {result}{'' if result.startswith('ERROR') else '% of pixels differ'}")
    if run.returncode or not passed + failed:
        print(run.stderr[-2000:])
    print(f"{passed} of {passed + failed} chart types drawn as Excel draws its own")
    if "--keep" in sys.argv:
        print("images kept in", work)
    else:
        shutil.rmtree(work, ignore_errors=True)
    sys.exit(1 if failed or not passed else 0)


if __name__ == "__main__":
    main()
