param(
  [string]$RoDir = 'C:\gate1_ro',
  [string]$OutDir = 'C:\gate1_out'
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'
. "$RoDir\scripts\gate1_common.ps1"

$shotsDir = Join-Path $OutDir 'screenshots'
$dbSnapsDir = Join-Path $OutDir 'db_snapshots'
New-Item -ItemType Directory -Force -Path $shotsDir,$dbSnapsDir | Out-Null
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = ''

function Stop-FarmerApps {
  Get-Process -Name 'FarmerPro','Auto_P6Alpha','Auto_P6Beta','fp-mini-runtime','fp-updater' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 1000
}

function Copy-DbGroup {
  param([string]$SrcDb, [string]$DstPrefix)
  foreach ($suf in @('', '-wal', '-shm')) {
    $s = "$SrcDb$suf"
    if (Test-Path $s) { Copy-Item $s "$DstPrefix$suf" -Force }
  }
}

$rep = [ordered]@{
  A_createAndPersist = [ordered]@{}
  F_updateRollback = [ordered]@{}
  sandboxVcrtCheck = [ordered]@{}
  errors = @()
}

try {
  # Check VC++ runtime DLL presence on Clean Windows Sandbox
  $rep.sandboxVcrtCheck = [ordered]@{
    vcruntime140Exists = Test-Path "$env:SystemRoot\System32\VCRUNTIME140.dll"
    vcruntime140_1Exists = Test-Path "$env:SystemRoot\System32\VCRUNTIME140_1.dll"
    msvcp140Exists = Test-Path "$env:SystemRoot\System32\MSVCP140.dll"
    ucrtbaseExists = Test-Path "$env:SystemRoot\System32\ucrtbase.dll"
    vcruntimeFilesInSystem32 = @(Get-ChildItem "$env:SystemRoot\System32\vcruntime*" -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
  }

  # -----------------------------------------------------------------------
  # A2. Install Studio, Create Project via visible "Tao Project dau tien", Workflow & Reopen
  # -----------------------------------------------------------------------
  Stop-FarmerApps
  $instExe = Join-Path $RoDir 'FarmerPro_0.1.0_x64-setup.exe'
  Start-Process -FilePath $instExe -ArgumentList '/S' -Wait | Out-Null
  $studioInstallDir = Join-Path $env:LOCALAPPDATA 'FarmerPro'
  $studioExe = Join-Path $studioInstallDir 'FarmerPro.exe'
  $env:FARMERPRO_DATA_DIR = ''
  $env:FARMERPRO_PROJECTS_DIR = ''

  $pS = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
  $wS = Wait-AppWindowReady -Proc $pS -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40

  $scr = [System.Windows.Forms.SystemInformation]::VirtualScreen
  # Exact screen-proportional coordinates measured from 03c_studio_project_created_real.png (2030x1230)
  [Win32Ui]::ClickAt([int]($scr.Width * 0.396), [int]($scr.Height * 0.579)) # "+ Tao Project dau tien"
  Start-Sleep -Milliseconds 1000
  [Win32Ui]::ClickAt([int]($scr.Width * 0.400), [int]($scr.Height * 0.375)) # "Ten Project *" input
  Start-Sleep -Milliseconds 250
  [System.Windows.Forms.SendKeys]::SendWait('Gate1 Sandbox RPG')
  Start-Sleep -Milliseconds 250
  [Win32Ui]::ClickAt([int]($scr.Width * 0.400), [int]($scr.Height * 0.438)) # "Android Package ID *" input
  Start-Sleep -Milliseconds 250
  [System.Windows.Forms.SendKeys]::SendWait('com.gate1.sbxrpg')
  Start-Sleep -Milliseconds 300
  [Win32Ui]::ClickAt([int]($scr.Width * 0.638), [int]($scr.Height * 0.765)) # "Tao Project" button
  Start-Sleep -Milliseconds 2400
  $sShotCreated = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '03c_studio_project_created_real.png')

  # Switch to Workflow Studio tab (6.9% x, 16.5% y)
  [Win32Ui]::ClickAt([int]($scr.Width * 0.069), [int]($scr.Height * 0.165))
  Start-Sleep -Milliseconds 1800
  $sShotWf = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04_studio_workflow_tab_real.png')

  # Switch to System Settings tab (6.9% x, 48.8% y)
  [Win32Ui]::ClickAt([int]($scr.Width * 0.069), [int]($scr.Height * 0.488))
  Start-Sleep -Milliseconds 1200
  $sShotSet = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04b_studio_settings_tab_real.png')
  Stop-FarmerApps

  # Reopen Studio to confirm persistence across restart
  $pS2 = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
  $wS2 = Wait-AppWindowReady -Proc $pS2 -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40
  $sShotReopen = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04c_studio_reopened_real.png')
  Stop-FarmerApps

  $studioUserDb = Join-Path $studioInstallDir 'StudioData\data\farmerpro.db'
  $studioProjectsDir = Join-Path $studioInstallDir 'StudioData\projects'
  $dbShaPre = if (Test-Path $studioUserDb) { (Get-FileHash $studioUserDb -Algorithm SHA256).Hash.ToLower() } else { $null }
  if (Test-Path $studioUserDb) { Copy-DbGroup -SrcDb $studioUserDb -DstPrefix (Join-Path $dbSnapsDir 'studio_after_create_reopen.db') }

  # Uninstall & confirm data preservation
  $uninstExe = Join-Path $studioInstallDir 'uninstall.exe'
  if (-not (Test-Path $uninstExe)) { $uninstExe = Join-Path $studioInstallDir 'Uninstall.exe' }
  $unP = Start-Process -FilePath $uninstExe -ArgumentList '/S' -Wait -PassThru
  Start-Sleep -Milliseconds 1500
  $dbShaPost = if (Test-Path $studioUserDb) { (Get-FileHash $studioUserDb -Algorithm SHA256).Hash.ToLower() } else { $null }

  $rep.A_createAndPersist = [ordered]@{
    projectDirsOnDisk = @(Get-ChildItem $studioProjectsDir -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
    dbShaBeforeUninstall = $dbShaPre
    dbShaAfterUninstall = $dbShaPost
    dbPreservedExactMatch = ($dbShaPre -and ($dbShaPre -eq $dbShaPost))
    studioExeRemovedAfterUninstall = -not (Test-Path $studioExe)
    screenshots = @($sShotCreated, $sShotWf, $sShotSet, $sShotReopen)
  }

  # -----------------------------------------------------------------------
  # F. Probe fp-updater.exe inside Clean Sandbox (reproduces 0xC0000135 VCRUNTIME140.dll!)
  # -----------------------------------------------------------------------
  $betaZipDir = 'C:\Gate1Work\Portable\P6_Beta'
  New-Item -ItemType Directory -Force -Path $betaZipDir | Out-Null
  Expand-Archive (Join-Path $RoDir 'P6_Beta_v1.0.0_win-x64.zip') $betaZipDir -Force
  $betaExe0 = Get-ChildItem $betaZipDir -Recurse -Filter 'Auto_P6Beta.exe' | Select-Object -First 1
  $betaRoot = $betaExe0.Directory.FullName
  $updaterExe = Join-Path $betaRoot 'updater\fp-updater.exe'
  $managedBeta = 'C:\Gate1Work\ManagedBeta'

  $pUpd = Start-Process -FilePath $updaterExe -ArgumentList 'install-initial','--install-root',$managedBeta,'--release-dir',$betaRoot -Wait -PassThru -NoNewWindow
  $rep.F_updateRollback = [ordered]@{
    updaterExePath = $updaterExe
    updaterExeExists = Test-Path $updaterExe
    installInitialExitCode = $pUpd.ExitCode
    installInitialHex = ('0x{0:X8}' -f ($pUpd.ExitCode -band 0xFFFFFFFF))
    status = if ($pUpd.ExitCode -eq 0) { 'PASS' } else { 'FAIL_CLEAN_WINDOWS_VCRUNTIME140_MISSING' }
    v100Installed = Test-Path (Join-Path $managedBeta 'versions\v1.0.0\Auto_P6Beta.exe')
  }
} catch {
  $rep.errors += ($_.Exception.ToString() + "`n" + $_.InvocationInfo.PositionMessage)
}

$rep | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $OutDir 'test_f_and_create_report.json') -Encoding UTF8
Write-Output 'TEST_F_AND_CREATE_DONE'
