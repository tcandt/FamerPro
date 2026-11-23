Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
public static class Win32Ui {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, UIntPtr dwExtraInfo);
  public static void ClickAt(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(80);
    mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
    System.Threading.Thread.Sleep(40);
    mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
  }
}
"@
[Win32Ui]::SetProcessDPIAware() | Out-Null

function Get-WebView2Entries {
  $keys = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
  )
  $entries = @()
  foreach ($k in $keys) {
    if (Test-Path $k) {
      $p = Get-ItemProperty $k -ErrorAction SilentlyContinue
      if ($p.pv) { $entries += [ordered]@{ key = $k; pv = [string]$p.pv } }
    }
  }
  return ,$entries
}

function Save-DesktopScreenshot {
  param([string]$OutPath)
  $dir = Split-Path -Parent $OutPath
  if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
  $bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
  $gfx = [System.Drawing.Graphics]::FromImage($bmp)
  $gfx.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
  $bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $gfx.Dispose()
  $bmp.Dispose()
  return [ordered]@{ path = $OutPath; width = $bounds.Width; height = $bounds.Height; bytes = (Get-Item $OutPath).Length }
}

function Wait-AppWindowReady {
  param(
    [System.Diagnostics.Process]$Proc,
    [string]$TitleRegex = '',
    [string]$WebViewExeName = '',
    [int]$TimeoutSec = 35
  )
  $dl = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $dl) {
    $Proc.Refresh()
    if ($Proc.HasExited) { throw "Process $($Proc.ProcessName) exited early with code $($Proc.ExitCode)" }
    $hwnd = $Proc.MainWindowHandle
    $title = $Proc.MainWindowTitle
    $wvOk = $true
    if ($WebViewExeName) {
      $r = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -match $WebViewExeName -and $_.CommandLine -match '--type=renderer' }
      $wvOk = ($null -ne $r)
    }
    if ($hwnd -ne [IntPtr]::Zero -and $wvOk) {
      if (-not $TitleRegex -or ($title -match $TitleRegex)) {
        [Win32Ui]::ShowWindow($hwnd, 9) | Out-Null
        [Win32Ui]::SetForegroundWindow($hwnd) | Out-Null
        Start-Sleep -Milliseconds 500
        $rect = New-Object RECT
        [Win32Ui]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
        $w = $rect.Right - $rect.Left
        $h = $rect.Bottom - $rect.Top
        # Wait until WebView2 finishes first non-white paint (soft-rendered Sandbox takes ~3-5s)
        $paintDl = (Get-Date).AddSeconds(16)
        $painted = $false
        while ((Get-Date) -lt $paintDl) {
          $bmp1 = New-Object System.Drawing.Bitmap(1, 1)
          $g1 = [System.Drawing.Graphics]::FromImage($bmp1)
          $sx = [Math]::Max(0, $rect.Left + [int]($w * 0.5))
          $sy = [Math]::Max(0, $rect.Top + [int]($h * 0.45))
          $g1.CopyFromScreen($sx, $sy, 0, 0, (New-Object System.Drawing.Size(1, 1)))
          $px = $bmp1.GetPixel(0, 0)
          $g1.Dispose()
          $bmp1.Dispose()
          if ($px.R -lt 235 -or $px.G -lt 235 -or $px.B -lt 235) {
            $painted = $true
            break
          }
          Start-Sleep -Milliseconds 350
        }
        Start-Sleep -Milliseconds 900
        [Win32Ui]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
        return [ordered]@{
          pid = $Proc.Id
          hwnd = [int64]$hwnd
          title = $title
          painted = $painted
          left = $rect.Left
          top = $rect.Top
          width = ($rect.Right - $rect.Left)
          height = ($rect.Bottom - $rect.Top)
        }
      }
    }
    Start-Sleep -Milliseconds 500
  }
  throw "Timed out waiting for window '$TitleRegex' ($WebViewExeName). Last title='$($Proc.MainWindowTitle)'"
}
