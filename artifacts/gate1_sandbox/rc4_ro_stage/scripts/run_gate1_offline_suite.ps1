param(
  [string]$RoDir = 'C:\gate1_ro',
  [string]$OutDir = 'C:\gate1_out'
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'
. "$RoDir\scripts\gate1_common.ps1"

$shotsDir = Join-Path $OutDir 'screenshots'
$dbSnapsDir = Join-Path $OutDir 'db_snapshots'
$updEvDir = Join-Path $OutDir 'updater_evidence'
New-Item -ItemType Directory -Force -Path $shotsDir,$dbSnapsDir,$updEvDir | Out-Null

$workRoot = 'C:\Gate1Work'
New-Item -ItemType Directory -Force -Path $workRoot | Out-Null
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

$report = [ordered]@{
  executedAtUtc = (Get-Date).ToUniversalTime().ToString('o')
  environment = [ordered]@{}
  stageShaChecks = @()
  B_portableBeforeStudio = [ordered]@{}
  A_studioNsisInstallAndPersist = [ordered]@{}
  UNINSTALL_dataPreservation = [ordered]@{}
  E_sqliteUpgrade = [ordered]@{}
  F_updateRollback = [ordered]@{}
  errors = @()
}

try {
  # -----------------------------------------------------------------------
  # 0. Environment & Stage Manifest SHA-256 Verification
  # -----------------------------------------------------------------------
  $os = Get-CimInstance Win32_OperatingSystem
  $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
  $report.environment = [ordered]@{
    computerName = $env:COMPUTERNAME
    username = $env:USERNAME
    isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    osCaption = $os.Caption
    osBuild = $os.BuildNumber
    arch = $env:PROCESSOR_ARCHITECTURE
    cpuName = $cpu.Name
    totalMemMb = [int]($os.TotalVisibleMemorySize / 1024)
    networking = 'Disable'
    nonLoopbackIpv4 = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -ne '127.0.0.1' } | ForEach-Object { $_.IPAddress })
    webview2BeforeTest = Get-WebView2Entries
    hasCargo = $null -ne (Get-Command cargo -ErrorAction SilentlyContinue)
    hasRustc = $null -ne (Get-Command rustc -ErrorAction SilentlyContinue)
    hasNode = $null -ne (Get-Command node -ErrorAction SilentlyContinue)
    hasAdbOnPath = $null -ne (Get-Command adb -ErrorAction SilentlyContinue)
  }

  $man = Get-Content (Join-Path $RoDir 'stage_manifest_sha256.json') -Raw | ConvertFrom-Json
  foreach ($rel in @('FarmerPro_0.1.0_x64-setup.exe','P6_Alpha_v1.0.0_win-x64.zip','P6_Beta_v1.0.0_win-x64.zip')) {
    $fp = Join-Path $RoDir $rel
    $h = (Get-FileHash $fp -Algorithm SHA256).Hash.ToLower()
    $exp = $man.$rel.sha256
    $report.stageShaChecks += [ordered]@{ file = $rel; bytes = (Get-Item $fp).Length; sha256 = $h; expectedSha256 = $exp; match = ($h -eq $exp) }
  }

  # -----------------------------------------------------------------------
  # B. Mini Tools Standalone (BEFORE Studio is installed)
  # -----------------------------------------------------------------------
  Stop-FarmerApps
  $studioExeDefault = Join-Path $env:LOCALAPPDATA 'FarmerPro\FarmerPro.exe'
  $studioPreExists = Test-Path $studioExeDefault

  $portDir = Join-Path $workRoot 'Portable'
  $alphaDir = Join-Path $portDir 'P6_Alpha'
  $betaDir = Join-Path $portDir 'P6_Beta'
  New-Item -ItemType Directory -Force -Path $alphaDir,$betaDir | Out-Null
  Expand-Archive -Path (Join-Path $RoDir 'P6_Alpha_v1.0.0_win-x64.zip') -DestinationPath $alphaDir -Force
  Expand-Archive -Path (Join-Path $RoDir 'P6_Beta_v1.0.0_win-x64.zip') -DestinationPath $betaDir -Force

  $alphaExe = Get-ChildItem $alphaDir -Recurse -Filter 'Auto_P6Alpha.exe' | Select-Object -First 1
  $betaExe = Get-ChildItem $betaDir -Recurse -Filter 'Auto_P6Beta.exe' | Select-Object -First 1
  $alphaRoot = $alphaExe.Directory.FullName
  $betaRoot = $betaExe.Directory.FullName

  $alphaDepsOut = (& $alphaExe.FullName '--check-deps' 2>&1 | Out-String).Trim()

  # Launch Portable Alpha standalone
  $pA = Start-Process -FilePath $alphaExe.FullName -WorkingDirectory $alphaRoot -PassThru
  $wA = Wait-AppWindowReady -Proc $pA -TitleRegex 'Zenith Harvester Pro' -WebViewExeName 'Auto_P6Alpha\.exe' -TimeoutSec 35
  $aShot1 = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '01_portable_alpha_standalone.png')
  # Click "CHAY TESTKIT (MO PHONG)" button at bottom footer (x = Left + 305, y = Top + Height - 26)
  [Win32Ui]::ClickAt($wA.left + 305, $wA.top + $wA.height - 26)
  Start-Sleep -Seconds 3
  $aShot2 = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '01b_portable_alpha_testkit_run.png')
  Stop-FarmerApps

  # Launch Portable Beta standalone
  $pB = Start-Process -FilePath $betaExe.FullName -WorkingDirectory $betaRoot -PassThru
  $wB = Wait-AppWindowReady -Proc $pB -TitleRegex 'Orion Citadel Automator' -WebViewExeName 'Auto_P6Beta\.exe' -TimeoutSec 35
  $bShot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '02_portable_beta_standalone.png')
  Stop-FarmerApps

  $alphaDb = Get-ChildItem (Join-Path $alphaRoot 'data') -Filter '*.db' -ErrorAction SilentlyContinue | Select-Object -First 1
  $betaDb = Get-ChildItem (Join-Path $betaRoot 'data') -Filter '*.db' -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($alphaDb) { Copy-DbGroup -SrcDb $alphaDb.FullName -DstPrefix (Join-Path $dbSnapsDir 'portable_alpha_standalone.db') }
  if ($betaDb) { Copy-DbGroup -SrcDb $betaDb.FullName -DstPrefix (Join-Path $dbSnapsDir 'portable_beta_standalone.db') }

  $report.B_portableBeforeStudio = [ordered]@{
    studioPreInstalled = $studioPreExists
    alphaExePath = $alphaExe.FullName
    betaExePath = $betaExe.FullName
    alphaCheckDeps = $alphaDepsOut
    alphaWindow = $wA
    betaWindow = $wB
    alphaDbCreated = ($null -ne $alphaDb)
    alphaDbName = if ($alphaDb) { $alphaDb.Name } else { $null }
    betaDbCreated = ($null -ne $betaDb)
    betaDbName = if ($betaDb) { $betaDb.Name } else { $null }
    screenshots = @($aShot1, $aShot2, $bShot)
  }

  # -----------------------------------------------------------------------
  # A. FarmerPro Studio NSIS Install + UI Subsystems + Persist + Uninstall
  # -----------------------------------------------------------------------
  $instExe = Join-Path $RoDir 'FarmerPro_0.1.0_x64-setup.exe'
  $instProc = Start-Process -FilePath $instExe -ArgumentList '/S' -Wait -PassThru
  Start-Sleep -Milliseconds 1200

  $studioInstallDir = Join-Path $env:LOCALAPPDATA 'FarmerPro'
  $studioExe = Join-Path $studioInstallDir 'FarmerPro.exe'
  $uninstExe = Join-Path $studioInstallDir 'uninstall.exe'
  if (-not (Test-Path $uninstExe)) { $uninstExe = Join-Path $studioInstallDir 'Uninstall.exe' }

  $installedFiles = [ordered]@{
    farmerproExe = Test-Path $studioExe
    fpMiniRuntimeExe = Test-Path (Join-Path $studioInstallDir 'fp-mini-runtime.exe')
    fpUpdaterExe = Test-Path (Join-Path $studioInstallDir 'fp-updater.exe')
    bundledAdbExe = Test-Path (Join-Path $studioInstallDir 'bin\adb.exe')
    nsisMarker = Test-Path (Join-Path $studioInstallDir '.nsis_installed')
    uninstallExe = Test-Path $uninstExe
  }

  $env:FARMERPRO_DATA_DIR = ''
  $env:FARMERPRO_PROJECTS_DIR = ''
  $pS = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
  $wS = Wait-AppWindowReady -Proc $pS -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40
  $sShotDash = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '03_studio_installed_dashboard.png')

  # Click "+ Tao Game Project" (top-right area of Projects & Dashboard: x ~ Left + Width - 110, y ~ Top + 82)
  [Win32Ui]::ClickAt($wS.left + $wS.width - 110, $wS.top + 82)
  Start-Sleep -Milliseconds 800
  # Click "Ten Project *" input inside modal (x ~ Left + 650, y ~ Top + 410), fill fields, click "Tao Project" (Left + 1275, Top + 870)
  [Win32Ui]::ClickAt($wS.left + 650, $wS.top + 410)
  Start-Sleep -Milliseconds 250
  [System.Windows.Forms.SendKeys]::SendWait('Gate1 Sandbox RPG{TAB}{TAB}com.gate1.sbxrpg')
  Start-Sleep -Milliseconds 350
  [Win32Ui]::ClickAt($wS.left + 1275, $wS.top + 870)
  Start-Sleep -Milliseconds 1800
  $sShotNewProj = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '03b_studio_after_create_project.png')

  # Click Workflow Studio tab (x = Left + 95, y = Top + 148)
  [Win32Ui]::ClickAt($wS.left + 95, $wS.top + 148)
  Start-Sleep -Milliseconds 1200
  $sShotWf = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04_studio_workflow_tab.png')

  # Click System Settings tab (x = Left + 95, y = Top + 568)
  [Win32Ui]::ClickAt($wS.left + 95, $wS.top + 568)
  Start-Sleep -Milliseconds 900
  $sShotSet = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04b_studio_settings_tab.png')

  Stop-FarmerApps

  # Reopen Studio to confirm persisted DB & project reload cleanly
  $pS2 = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
  $wS2 = Wait-AppWindowReady -Proc $pS2 -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40
  $sShotReopen = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '04c_studio_reopened.png')
  Stop-FarmerApps

  $studioUserDb = Join-Path $studioInstallDir 'StudioData\data\farmerpro.db'
  $studioProjectsDir = Join-Path $studioInstallDir 'StudioData\projects'
  $dbExistsPreUninstall = Test-Path $studioUserDb
  $dbShaPreUninstall = if ($dbExistsPreUninstall) { (Get-FileHash $studioUserDb -Algorithm SHA256).Hash.ToLower() } else { $null }
  if ($dbExistsPreUninstall) { Copy-DbGroup -SrcDb $studioUserDb -DstPrefix (Join-Path $dbSnapsDir 'studio_after_create_reopen.db') }

  $report.A_studioNsisInstallAndPersist = [ordered]@{
    nsisExitCode = $instProc.ExitCode
    installDir = $studioInstallDir
    installedFiles = $installedFiles
    window1 = $wS
    window2Reopen = $wS2
    projectDirsOnDisk = @(Get-ChildItem $studioProjectsDir -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
    studioUserDb = $studioUserDb
    dbExists = $dbExistsPreUninstall
    dbSha256 = $dbShaPreUninstall
    screenshots = @($sShotDash, $sShotNewProj, $sShotWf, $sShotSet, $sShotReopen)
  }

  # Uninstall Studio & verify user data preserved + Portable Alpha still launches!
  $unProc = Start-Process -FilePath $uninstExe -ArgumentList '/S' -Wait -PassThru
  Start-Sleep -Milliseconds 1500
  $dbExistsPostUninstall = Test-Path $studioUserDb
  $dbShaPostUninstall = if ($dbExistsPostUninstall) { (Get-FileHash $studioUserDb -Algorithm SHA256).Hash.ToLower() } else { $null }

  $pA2 = Start-Process -FilePath $alphaExe.FullName -WorkingDirectory $alphaRoot -PassThru
  $wA2 = Wait-AppWindowReady -Proc $pA2 -TitleRegex 'Zenith Harvester Pro' -WebViewExeName 'Auto_P6Alpha\.exe' -TimeoutSec 35
  Stop-FarmerApps

  $report.UNINSTALL_dataPreservation = [ordered]@{
    uninstallExitCode = $unProc.ExitCode
    farmerproExeRemoved = -not (Test-Path $studioExe)
    nsisMarkerRemoved = -not (Test-Path (Join-Path $studioInstallDir '.nsis_installed'))
    userDbPreserved = $dbExistsPostUninstall
    userDbShaBefore = $dbShaPreUninstall
    userDbShaAfter = $dbShaPostUninstall
    userDbShaExactMatch = ($dbShaPreUninstall -and ($dbShaPreUninstall -eq $dbShaPostUninstall))
    portableAlphaRunsAfterStudioUninstall = ($wA2.title -match 'Zenith Harvester Pro')
  }

  # -----------------------------------------------------------------------
  # E. SQLite Upgrade Compatibility inside Sandbox (v5 clean + partial 2/4 & 3/4)
  # -----------------------------------------------------------------------
  $instProc2 = Start-Process -FilePath $instExe -ArgumentList '/S' -Wait -PassThru
  Start-Sleep -Milliseconds 1000
  $sqlWork = Join-Path $workRoot 'SqlUpgrade'
  $eResults = [ordered]@{}
  foreach ($case in @(
    @{ name = 'rc2_v5_clean'; src = (Join-Path $RoDir 'fixtures\rc2_v5_clean.db') },
    @{ name = 'rc2_v5_partial_2cols'; src = (Join-Path $RoDir 'fixtures\rc2_v5_partial_2cols.db') },
    @{ name = 'rc2_v5_partial_3cols'; src = (Join-Path $RoDir 'fixtures\rc2_v5_partial_3cols.db') }
  )) {
    $cDir = Join-Path $sqlWork $case.name
    New-Item -ItemType Directory -Force -Path $cDir | Out-Null
    $cDb = Join-Path $cDir 'farmerpro.db'
    Copy-Item $case.src $cDb -Force
    $env:FARMERPRO_DATA_DIR = $cDir
    $psSql = Start-Process -FilePath $studioExe -WorkingDirectory $studioInstallDir -PassThru
    $wSql = Wait-AppWindowReady -Proc $psSql -TitleRegex 'FarmerPro' -WebViewExeName 'FarmerPro\.exe' -TimeoutSec 40
    Start-Sleep -Milliseconds 800
    $sqlShot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir ("05_sql_" + $case.name + ".png"))
    Stop-FarmerApps
    Copy-DbGroup -SrcDb $cDb -DstPrefix (Join-Path $dbSnapsDir ("upgraded_" + $case.name + ".db"))
    $eResults[$case.name] = [ordered]@{
      studioWindow = $wSql
      screenshot = $sqlShot
    }
  }
  $env:FARMERPRO_DATA_DIR = ''
  $report.E_sqliteUpgrade = $eResults

  # -----------------------------------------------------------------------
  # F. Update / Rollback / Re-Upgrade (P6_Beta v1.0.0 -> v1.1.0 -> Rollback -> Re-Upgrade)
  # -----------------------------------------------------------------------
  $managedBeta = Join-Path $workRoot 'ManagedBeta'
  if (Test-Path $managedBeta) { Remove-Item $managedBeta -Recurse -Force -ErrorAction SilentlyContinue }
  $updaterExe = Join-Path $betaRoot 'updater\fp-updater.exe'
  $v110StageDir = Join-Path $RoDir 'P6_Beta_v1.1.0_win-x64'

  $instInitRaw = (& $updaterExe 'install-initial' '--install-root' $managedBeta '--release-dir' $betaRoot 2>&1 | Out-String).Trim()
  $managedExeV100 = Join-Path $managedBeta 'versions\v1.0.0\Auto_P6Beta.exe'
  $managedExeV110 = Join-Path $managedBeta 'versions\v1.1.0\Auto_P6Beta.exe'
  $managedV100Dir = Join-Path $managedBeta 'versions\v1.0.0'
  $managedV110Dir = Join-Path $managedBeta 'versions\v1.1.0'

  # Launch v1.0.0 and click "CHAY TESTKIT" to write pre-upgrade runtime DB rows
  $pM1 = Start-Process -FilePath $managedExeV100 -WorkingDirectory $managedV100Dir -PassThru
  $wM1 = Wait-AppWindowReady -Proc $pM1 -TitleRegex 'Orion Citadel Automator v1\.0\.0' -WebViewExeName 'Auto_P6Beta\.exe' -TimeoutSec 35
  [Win32Ui]::ClickAt($wM1.left + 305, $wM1.top + $wM1.height - 26)
  Start-Sleep -Seconds 4
  $m1Shot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '06_beta_v100_pre_upgrade.png')
  Stop-FarmerApps
  $managedDb = Join-Path $managedBeta 'data\runtime_p6-beta-citadel.db'
  if (Test-Path $managedDb) { Copy-DbGroup -SrcDb $managedDb -DstPrefix (Join-Path $updEvDir 'beta_v100_pre_upgrade.db') }

  # Upgrade v1.0.0 -> v1.1.0
  $upgRaw = (& $updaterExe 'upgrade' '--install-root' $managedBeta '--release-dir' $v110StageDir 2>&1 | Out-String).Trim()
  $pM2 = Start-Process -FilePath $managedExeV110 -WorkingDirectory $managedV110Dir -PassThru
  $wM2 = Wait-AppWindowReady -Proc $pM2 -TitleRegex 'Orion Citadel Automator v1\.1\.0' -WebViewExeName 'Auto_P6Beta\.exe' -TimeoutSec 35
  [Win32Ui]::ClickAt($wM2.left + 305, $wM2.top + $wM2.height - 26)
  Start-Sleep -Seconds 4
  $m2Shot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '06b_beta_v110_post_upgrade.png')
  Stop-FarmerApps
  if (Test-Path $managedDb) { Copy-DbGroup -SrcDb $managedDb -DstPrefix (Join-Path $updEvDir 'beta_v110_post_upgrade.db') }

  # Rollback v1.1.0 -> v1.0.0
  $rbRaw = (& $updaterExe 'rollback' '--install-root' $managedBeta 2>&1 | Out-String).Trim()
  $pM3 = Start-Process -FilePath $managedExeV100 -WorkingDirectory $managedV100Dir -PassThru
  $wM3 = Wait-AppWindowReady -Proc $pM3 -TitleRegex 'Orion Citadel Automator v1\.0\.0' -WebViewExeName 'Auto_P6Beta\.exe' -TimeoutSec 35
  $m3Shot = Save-DesktopScreenshot -OutPath (Join-Path $shotsDir '06c_beta_v100_post_rollback.png')
  Stop-FarmerApps
  if (Test-Path $managedDb) { Copy-DbGroup -SrcDb $managedDb -DstPrefix (Join-Path $updEvDir 'beta_v100_post_rollback.db') }

  # Re-upgrade v1.0.0 -> v1.1.0 & verify-security
  $reUpgRaw = (& $updaterExe 'upgrade' '--install-root' $managedBeta '--release-dir' $v110StageDir 2>&1 | Out-String).Trim()
  $secRaw = (& $updaterExe 'verify-security' '--install-root' $managedBeta 2>&1 | Out-String).Trim()
  if (Test-Path $managedDb) { Copy-DbGroup -SrcDb $managedDb -DstPrefix (Join-Path $updEvDir 'beta_v110_post_reupgrade.db') }
  if (Test-Path (Join-Path $managedBeta '.updater')) { Copy-Item (Join-Path $managedBeta '.updater\*') $updEvDir -Force -ErrorAction SilentlyContinue }
  if (Test-Path (Join-Path $managedBeta 'active_version.json')) { Copy-Item (Join-Path $managedBeta 'active_version.json') (Join-Path $updEvDir 'active_version.json') -Force }

  $report.F_updateRollback = [ordered]@{
    installInitialOut = $instInitRaw
    v100Window = $wM1
    upgradeOut = $upgRaw
    v110Window = $wM2
    rollbackOut = $rbRaw
    postRollbackWindow = $wM3
    reUpgradeOut = $reUpgRaw
    verifySecurityOut = $secRaw
    screenshots = @($m1Shot, $m2Shot, $m3Shot)
  }
} catch {
  $report.errors += ($_.Exception.ToString())
}

$report | ConvertTo-Json -Depth 10 | Set-Content -Path (Join-Path $OutDir 'offline_report.json') -Encoding UTF8
Write-Output 'GATE1_OFFLINE_SUITE_DONE'
