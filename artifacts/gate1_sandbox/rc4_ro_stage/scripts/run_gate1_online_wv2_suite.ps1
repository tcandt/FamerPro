param(
  [string]$Phase = 'present_online',
  [string]$RoDir = 'C:\gate1_ro',
  [string]$OutDir = 'C:\gate1_out'
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'
. "$RoDir\scripts\gate1_common.ps1"
$shotsDir = Join-Path $OutDir 'screenshots'
New-Item -ItemType Directory -Force -Path $shotsDir | Out-Null
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = ''

$instExe = Join-Path $RoDir 'FarmerPro_0.1.0_x64-setup.exe'
$studioInstallDir = Join-Path $env:LOCALAPPDATA 'FarmerPro'
$studioExe = Join-Path $studioInstallDir 'FarmerPro.exe'

$p1 = Start-Process -FilePath $instExe -ArgumentList '/S' -Wait -PassThru
$s1 = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
$wS = Wait-AppWindowReady -Proc $s1 -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40
$fwShot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '07_online_firewall_adb_prompt.png')
[Win32Ui]::SetForegroundWindow([IntPtr]$wS.hwnd) | Out-Null
Start-Sleep -Milliseconds 600
$shot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '07b_online_studio_present_wv2.png')
Get-Process -Name 'FarmerPro','msedgewebview2' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

[ordered]@{
  networking = 'Enable'
  nonLoopbackIpv4 = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -ne '127.0.0.1' } | ForEach-Object { $_.IPAddress })
  webview2Initially = Get-WebView2Entries
  nsisExitCode = $p1.ExitCode
  studioWindow = $wS
  studioLaunchOk = ($wS.title -match 'FarmerPro')
  screenshot = $shot
} | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $OutDir 'online_present_wv2.json') -Encoding UTF8
