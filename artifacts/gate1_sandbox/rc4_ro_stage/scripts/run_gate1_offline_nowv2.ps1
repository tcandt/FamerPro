param(
  [string]$Mode = 'uninstall_wv2',
  [string]$RoDir = 'C:\gate1_ro',
  [string]$OutDir = 'C:\gate1_out'
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'
. "$RoDir\scripts\gate1_common.ps1"
$shotsDir = Join-Path $OutDir 'screenshots'
New-Item -ItemType Directory -Force -Path $shotsDir | Out-Null

if ($Mode -eq 'uninstall_wv2') {
  Get-Process -Name 'FarmerPro','Auto_P6Alpha','Auto_P6Beta','msedgewebview2' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 600
  $wv2Setup = Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Recurse -Filter 'setup.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
  $exitCode = $null
  if ($wv2Setup) {
    $wp = Start-Process -FilePath $wv2Setup.FullName -ArgumentList '--uninstall','--msedgewebview','--system-level','--verbose-logging','--force-uninstall' -Wait -PassThru
    $exitCode = $wp.ExitCode
  }
  Start-Sleep -Seconds 1
  $entries = Get-WebView2Entries
  [ordered]@{
    setupPath = if ($wv2Setup) { $wv2Setup.FullName } else { $null }
    uninstallExitCode = $exitCode
    remainingEntries = $entries
    remainingCount = @($entries).Count
  } | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $OutDir 'wv2_uninstall_sys.json') -Encoding UTF8
  exit 0
}

# Mode = test_offline_missing_wv2
$studioInstallDir = Join-Path $env:LOCALAPPDATA 'FarmerPro'
$unExe = Join-Path $studioInstallDir 'uninstall.exe'
if (-not (Test-Path $unExe)) { $unExe = Join-Path $studioInstallDir 'Uninstall.exe' }
if (Test-Path $unExe) { Start-Process -FilePath $unExe -ArgumentList '/S' -Wait | Out-Null }

$alphaExe = Get-ChildItem 'C:\Gate1Work\Portable\P6_Alpha' -Recurse -Filter 'Auto_P6Alpha.exe' | Select-Object -First 1
$wv2Before = Get-WebView2Entries
$pAlphaNoWv = Start-Process -FilePath $alphaExe.FullName -WorkingDirectory $alphaExe.Directory.FullName -PassThru
Start-Sleep -Seconds 6
$alphaNoWvExited = $pAlphaNoWv.HasExited
$alphaNoWvCode = if ($alphaNoWvExited) { $pAlphaNoWv.ExitCode } else { $null }
$noWvShot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '06_offline_missing_webview2_mini.png')
if (-not $alphaNoWvExited) { Stop-Process -Id $pAlphaNoWv.Id -Force -ErrorAction SilentlyContinue }

$instExe = Join-Path $RoDir 'FarmerPro_0.1.0_x64-setup.exe'
$instOffNoWv = Start-Process -FilePath $instExe -ArgumentList '/S' -Wait -PassThru
$wv2AfterNsisOffline = Get-WebView2Entries
$studioExe = Join-Path $studioInstallDir 'FarmerPro.exe'
$studioNoWvExe = Test-Path $studioExe
$studioNoWvExited = $null
$studioNoWvCode = $null
if ($studioNoWvExe) {
  $pStNoWv = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
  Start-Sleep -Seconds 6
  $studioNoWvExited = $pStNoWv.HasExited
  $studioNoWvCode = if ($studioNoWvExited) { $pStNoWv.ExitCode } else { $null }
  if (-not $studioNoWvExited) { Stop-Process -Id $pStNoWv.Id -Force -ErrorAction SilentlyContinue }
}

[ordered]@{
  wv2CountBeforeTest = @($wv2Before).Count
  portableAlphaExited = $alphaNoWvExited
  portableAlphaExitCode = $alphaNoWvCode
  nsisOfflineExitCode = $instOffNoWv.ExitCode
  wv2CountAfterNsisOffline = @($wv2AfterNsisOffline).Count
  studioExeExists = $studioNoWvExe
  studioOfflineMissingWv2Exited = $studioNoWvExited
  studioOfflineMissingWv2Code = $studioNoWvCode
  screenshot = $noWvShot
} | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $OutDir 'offline_missing_wv2_report.json') -Encoding UTF8
