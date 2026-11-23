param(
  [string]$RoDir = 'C:\gate1_ro',
  [string]$OutDir = 'C:\gate1_out'
)

$ErrorActionPreference = 'Continue'
$ProgressPreference = 'SilentlyContinue'

$u = 'fpstandard'
$pwPlain = 'FpStd!2026#9'
$secPw = ConvertTo-SecureString $pwPlain -AsPlainText -Force
$cred = New-Object System.Management.Automation.PSCredential("$env:COMPUTERNAME\$u", $secPw)

net user $u /delete 2>&1 | Out-Null
$createOut = (net user $u $pwPlain /add 2>&1 | Out-String).Trim()
$adminsOut = (net localgroup Administrators 2>&1 | Out-String).Trim()
$isStdInAdmins = $adminsOut -match $u

$stdWork = 'C:\Gate1StdWork'
if (Test-Path $stdWork) { Remove-Item $stdWork -Recurse -Force -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path "$stdWork\Portable","$stdWork\Logs" | Out-Null
Copy-Item "$RoDir\FarmerPro_0.1.0_x64-setup.exe" "$stdWork\FarmerPro_0.1.0_x64-setup.exe" -Force
Expand-Archive "$RoDir\P6_Alpha_v1.0.0_win-x64.zip" "$stdWork\Portable\P6_Alpha" -Force
$alphaExe = (Get-ChildItem "$stdWork\Portable\P6_Alpha" -Recurse -Filter 'Auto_P6Alpha.exe' | Select-Object -First 1).FullName

$childScript = @"
`$ErrorActionPreference = 'Continue'
`$log = 'C:\Gate1StdWork\Logs\std_run.json'
`$id = [Security.Principal.WindowsIdentity]::GetCurrent()
`$pr = New-Object Security.Principal.WindowsPrincipal(`$id)
`$whoGroups = (whoami /groups | Out-String).Trim()
`$hasAdminSid = `$whoGroups -match 'S-1-5-32-544'
`$depsOut = (& '$alphaExe' '--check-deps' 2>&1 | Out-String).Trim()
`$instP = Start-Process -FilePath '$stdWork\FarmerPro_0.1.0_x64-setup.exe' -ArgumentList '/S' -Wait -PassThru
`$stExe = Join-Path `$env:LOCALAPPDATA 'FarmerPro\FarmerPro.exe'
`$stExists = Test-Path `$stExe
[ordered]@{
  whoami = (whoami).Trim()
  isAdminRole = `$pr.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  hasBuiltinAdminSid = `$hasAdminSid
  localAppData = `$env:LOCALAPPDATA
  alphaCheckDeps = `$depsOut
  nsisInstallExitCode = `$instP.ExitCode
  studioExeInstalled = `$stExists
} | ConvertTo-Json -Depth 4 | Set-Content -Path `$log -Encoding UTF8
"@
$childPath = "$stdWork\run_as_std.ps1"
$childScript | Set-Content -Path $childPath -Encoding UTF8
icacls $stdWork /grant "Users:(OI)(CI)(F)" | Out-Null

# 1. Try Start-Process -Credential (tests real non-admin user token directly without ScheduledTask batch policy)
$spErr = $null
try {
  $sp = Start-Process -FilePath 'powershell.exe' -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File $childPath" -Credential $cred -WorkingDirectory $stdWork -Wait -PassThru -ErrorAction Stop
  $spExit = $sp.ExitCode
} catch {
  $spErr = $_.Exception.Message
  $spExit = $null
}

# 2. If Start-Process -Credential failed, record ScheduledTask diagnostics
$taskName = 'Gate1StandardUserCheck'
$taskErr = $null
$taskInfo = $null
if (-not (Test-Path "$stdWork\Logs\std_run.json")) {
  try {
    schtasks /Delete /TN $taskName /F 2>&1 | Out-Null
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -File $childPath"
    Register-ScheduledTask -TaskName $taskName -Action $action -User "$env:COMPUTERNAME\$u" -Password $pwPlain -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    Start-Sleep -Seconds 8
    $info = Get-ScheduledTaskInfo -TaskName $taskName -ErrorAction SilentlyContinue
    $taskInfo = [ordered]@{ lastTaskResult = $info.LastTaskResult; lastRunTime = [string]$info.LastRunTime }
  } catch {
    $taskErr = $_.Exception.Message
  }
}

$stdJson = $null
if (Test-Path "$stdWork\Logs\std_run.json") {
  $stdJson = Get-Content "$stdWork\Logs\std_run.json" -Raw | ConvertFrom-Json
}

[ordered]@{
  standardUsername = $u
  createOut = $createOut
  isInAdministratorsGroup = $isStdInAdmins
  startProcessCredExit = $spExit
  startProcessCredErr = $spErr
  scheduledTaskInfo = $taskInfo
  scheduledTaskErr = $taskErr
  stdRunResult = $stdJson
} | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $OutDir 'standard_user_report.json') -Encoding UTF8
Write-Output 'GATE1_STD_USER_DONE'
