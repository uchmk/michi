# Takes screenshots of a running michi for a session without a screen of its own:
#   scripts/shot.ps1 -Exe target\debug\michi.exe -Arg C:\path\to\repo -Out $env:TEMP\shot\name -Width 1100 -Keys "@500,167|sleep1;{ENTER}"
# Groups in -Keys are split by ';' (one picture after each), keys in a group by '|':
# '@x,y' clicks at a point of the window, '@@x,y' double-clicks, 'r@x,y' right-clicks, '@x,y>x2,y2' drags, 'sleepN' waits N seconds, anything else goes to SendKeys.
# It stops when michi has gone or another window is in front, so no key lands elsewhere;
# what michi wrote to its standard error is in <Out>-stderr.txt.
param([string]$Exe, [string]$Arg, [string]$Out, [int]$Wait = 8, [string]$Do = "", [int]$After = 3, [string]$Keys = "", [int]$Width = 1400, [switch]$Keep)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int L, T, R, B; }
public static class W {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int x, int y, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
}
"@
[W]::SetProcessDPIAware() | Out-Null
$p = Start-Process -FilePath $Exe -ArgumentList $Arg -PassThru -RedirectStandardError "$Out-stderr.txt"
Start-Sleep -Seconds $Wait
$p.Refresh()
$h = $p.MainWindowHandle
[W]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [W]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
[W]::SetForegroundWindow($h) | Out-Null
$r = New-Object RECT
[W]::GetWindowRect($h, [ref]$r) | Out-Null
if ($Width -gt 0) { [W]::MoveWindow($h, $r.L, $r.T, $Width, $r.B - $r.T, $true) | Out-Null; Start-Sleep -Milliseconds 700; [W]::GetWindowRect($h, [ref]$r) | Out-Null }
[W]::SetCursorPos([int](($r.L + $r.R) / 2), [int]($r.T + 200)) | Out-Null
[W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 500
function Shoot($name) {
  Start-Sleep -Milliseconds 300
  [W]::GetWindowRect($h, [ref]$r) | Out-Null
  $w = $r.R - $r.L; $hh = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
  $bmp.Save("$Out-$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
}
if ($Do -ne "") { Invoke-Expression $Do; Start-Sleep -Seconds $After }
Shoot "a"
$i = 0
:groups foreach ($group in ($Keys -split ';')) {
  if ($group -eq "") { continue }
  foreach ($k in ($group -split '\|')) {
    if ($k -notmatch '^sleep[0-9]+$') {
      if ($p.HasExited) { "michi exited ($($p.ExitCode)) before '$k': see $Out-stderr.txt"; break groups }
      if ([W]::GetForegroundWindow() -ne $h) { "another window came in front before '$k': stopped"; break groups }
    }
    if ($k -match '^r@([0-9]+),([0-9]+)$') {
      [W]::SetCursorPos($r.L + [int]$Matches[1], $r.T + [int]$Matches[2]) | Out-Null
      Start-Sleep -Milliseconds 150
      [W]::mouse_event(8, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60; [W]::mouse_event(16, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 500
    } elseif ($k -match '^@@([0-9]+),([0-9]+)$') {
      [W]::SetCursorPos($r.L + [int]$Matches[1], $r.T + [int]$Matches[2]) | Out-Null
      Start-Sleep -Milliseconds 150
      foreach ($n in 1..2) { [W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60 }
      Start-Sleep -Milliseconds 500
    } elseif ($k -match '^@([0-9]+),([0-9]+)>([0-9]+),([0-9]+)$') {
      [W]::SetCursorPos($r.L + [int]$Matches[1], $r.T + [int]$Matches[2]) | Out-Null
      Start-Sleep -Milliseconds 150
      [W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 100
      [W]::SetCursorPos($r.L + [int]$Matches[3], $r.T + [int]$Matches[4]) | Out-Null
      Start-Sleep -Milliseconds 200
      [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 500
    } elseif ($k -match '^@([0-9]+),([0-9]+)$') {
      [W]::SetCursorPos($r.L + [int]$Matches[1], $r.T + [int]$Matches[2]) | Out-Null
      Start-Sleep -Milliseconds 150
      [W]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 60; [W]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
      Start-Sleep -Milliseconds 500
    } elseif ($k -match '^sleep([0-9]+)$') { Start-Sleep -Seconds ([int]$Matches[1]) }
    elseif ($k -ne "") { [System.Windows.Forms.SendKeys]::SendWait($k); Start-Sleep -Milliseconds 500 }
  }
  Shoot ([string][char](98 + $i))
  $i++
}
if (-not $Keep -and -not $p.HasExited) { Stop-Process -Id $p.Id }
"$($r.R - $r.L) x $($r.B - $r.T)"
