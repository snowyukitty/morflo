param(
  [Parameter(Mandatory)][int]$ProcessId,
  [Parameter(Mandatory)][string]$Output,
  [int]$Width = 1280,
  [int]$Height = 800
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class MorfloWindowCapture {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr w, out Rect r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr w, out Rect r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr w, IntPtr a, int x,int y,int cx,int cy,uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr w, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr w, int a, out Rect r, int size);
}
'@
[void][MorfloWindowCapture]::SetProcessDpiAwarenessContext([IntPtr](-4))
$app = Get-Process -Id $ProcessId
if ($app.ProcessName -ne 'morflo' -or $app.MainWindowHandle -eq 0) { throw 'Expected a live Morflo native window.' }
$handle = $app.MainWindowHandle
$rect = New-Object MorfloWindowCapture+Rect
$client = New-Object MorfloWindowCapture+Rect
[void][MorfloWindowCapture]::GetWindowRect($handle,[ref]$rect)
[void][MorfloWindowCapture]::GetClientRect($handle,[ref]$client)
[void][MorfloWindowCapture]::SetWindowPos($handle,[IntPtr]::Zero,0,0,($Width+$rect.R-$rect.L-$client.R),($Height+$rect.B-$rect.T-$client.B),0x0016)
Start-Sleep -Milliseconds 450
[void][MorfloWindowCapture]::GetWindowRect($handle,[ref]$rect)
$bitmap = New-Object Drawing.Bitmap(($rect.R-$rect.L),($rect.B-$rect.T))
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$dc = $graphics.GetHdc()
try { if (-not [MorfloWindowCapture]::PrintWindow($handle,$dc,2)) { throw 'PrintWindow failed.' } }
finally { $graphics.ReleaseHdc($dc); $graphics.Dispose() }
$visible = New-Object MorfloWindowCapture+Rect
[void][MorfloWindowCapture]::DwmGetWindowAttribute($handle,9,[ref]$visible,16)
$crop = New-Object Drawing.Rectangle(($visible.L-$rect.L),($visible.T-$rect.T),($visible.R-$visible.L),($visible.B-$visible.T))
$image = $bitmap.Clone($crop,$bitmap.PixelFormat)
try { $image.Save($Output,[Drawing.Imaging.ImageFormat]::Png) }
finally { $image.Dispose(); $bitmap.Dispose() }
