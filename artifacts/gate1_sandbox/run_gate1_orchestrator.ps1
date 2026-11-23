$ErrorActionPreference = 'Stop'
$root = 'E:\DATA\FarmerPro\artifacts\gate1_sandbox'
$roStage = Join-Path $root 'rc4_ro_stage'
$evOff = Join-Path $root 'evidence_offline'
$evOn = Join-Path $root 'evidence_online'
New-Item -ItemType Directory -Force -Path $evOff,$evOn | Out-Null

# Clean up any leftover sandboxes from earlier diagnostics
$oldSbx = (& wsb.exe list --raw | ConvertFrom-Json).WindowsSandboxEnvironments
foreach ($e in $oldSbx) { & wsb.exe stop --id $e --raw | Out-Null }

# Refresh stage manifest SHA-256 cleanly
python -c "import os,json,hashlib; s=r'$roStage'; h=lambda p: hashlib.sha256(open(p,'rb').read()).hexdigest(); man={os.path.relpath(os.path.join(r,fn),s).replace('\\','/'):{'bytes':os.path.getsize(os.path.join(r,fn)),'sha256':h(os.path.join(r,fn))} for r,_,fs in os.walk(s) for fn in sorted(fs) if fn!='stage_manifest_sha256.json'}; open(os.path.join(s,'stage_manifest_sha256.json'),'w',encoding='utf-8').write(json.dumps(man,indent=2)); print('MANIFEST_REFRESHED', len(man))"

function Wait-ExistingLogin {
  param([string]$SbxId, [int]$TimeoutSec = 75)
  $dl = (Get-Date).AddSeconds($TimeoutSec)
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    while ((Get-Date) -lt $dl) {
      $psi = New-Object System.Diagnostics.ProcessStartInfo
      $psi.FileName = 'wsb.exe'
      $psi.Arguments = "exec --id $SbxId -r ExistingLogin -c `"cmd.exe /c echo READY`" --raw"
      $psi.RedirectStandardOutput = $true
      $psi.RedirectStandardError = $true
      $psi.UseShellExecute = $false
      $p = [System.Diagnostics.Process]::Start($psi)
      $stdout = $p.StandardOutput.ReadToEnd()
      $p.WaitForExit()
      if ($p.ExitCode -eq 0 -and $stdout -match '"ExitCode"\s*:\s*0') { return $true }
      Start-Sleep -Seconds 2
    }
  } finally {
    $ErrorActionPreference = $prevEap
  }
  return $false
}

# =========================================================================
# SESSION 1: OFFLINE WINDOWS SANDBOX (Networking Disable)
# =========================================================================
Remove-Item "$evOff\*" -Recurse -Force -ErrorAction SilentlyContinue
$cfgOff = '<Configuration><VGpu>Disable</VGpu><Networking>Disable</Networking><AudioInput>Disable</AudioInput><VideoInput>Disable</VideoInput><ProtectedClient>Enable</ProtectedClient><PrinterRedirection>Disable</PrinterRedirection><ClipboardRedirection>Disable</ClipboardRedirection><MemoryInMB>4096</MemoryInMB></Configuration>'
$startOff = (& wsb.exe start --raw -c $cfgOff) | ConvertFrom-Json
$sbxOff = $startOff.Id
Write-Output ("OFFLINE_SANDBOX_ID=" + $sbxOff)

try {
  & wsb.exe share --id $sbxOff -f $roStage -s 'C:\gate1_ro' | Out-Null
  & wsb.exe share --id $sbxOff -f $evOff -s 'C:\gate1_out' -w | Out-Null
  Start-Process -FilePath 'wsb.exe' -ArgumentList 'connect','--id',$sbxOff | Out-Null
  $loginOk1 = Wait-ExistingLogin -SbxId $sbxOff -TimeoutSec 75
  Write-Output ("OFFLINE_EXISTING_LOGIN_READY=" + $loginOk1)
  if (-not $loginOk1) { throw 'Timeout waiting for ExistingLogin in Offline Sandbox' }

  Start-Sleep -Seconds 3
  Write-Output '1/3 RUNNING_OFFLINE_SUITE...'
  & wsb.exe exec --id $sbxOff -r ExistingLogin -c "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\gate1_ro\scripts\run_gate1_offline_suite.ps1" --raw | Tee-Object -FilePath (Join-Path $evOff 'wsb_exec_offline.log')

  Write-Output '2/3 RUNNING_STANDARD_USER_PROBE...'
  & wsb.exe exec --id $sbxOff -r System -c "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\gate1_ro\scripts\run_gate1_standard_user_probe.ps1" --raw | Tee-Object -FilePath (Join-Path $evOff 'wsb_exec_stduser.log')

  Write-Output '3/3 PROBING_WEBVIEW2_UNINSTALL_OFFLINE...'
  & wsb.exe exec --id $sbxOff -r System -c "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\gate1_ro\scripts\run_gate1_offline_nowv2.ps1 -Mode uninstall_wv2" --raw | Tee-Object -FilePath (Join-Path $evOff 'wsb_exec_wv2uninst.log')
} finally {
  Write-Output ("STOPPING_OFFLINE_SANDBOX=" + $sbxOff)
  & wsb.exe stop --id $sbxOff --raw | Out-Null
}

# =========================================================================
# SESSION 2: ONLINE WINDOWS SANDBOX (Networking Enable)
# =========================================================================
Remove-Item "$evOn\*" -Recurse -Force -ErrorAction SilentlyContinue
$cfgOn = '<Configuration><VGpu>Disable</VGpu><Networking>Enable</Networking><AudioInput>Disable</AudioInput><VideoInput>Disable</VideoInput><ProtectedClient>Enable</ProtectedClient><PrinterRedirection>Disable</PrinterRedirection><ClipboardRedirection>Disable</ClipboardRedirection><MemoryInMB>4096</MemoryInMB></Configuration>'
$startOn = (& wsb.exe start --raw -c $cfgOn) | ConvertFrom-Json
$sbxOn = $startOn.Id
Write-Output ("ONLINE_SANDBOX_ID=" + $sbxOn)

try {
  & wsb.exe share --id $sbxOn -f $roStage -s 'C:\gate1_ro' | Out-Null
  & wsb.exe share --id $sbxOn -f $evOn -s 'C:\gate1_out' -w | Out-Null
  Start-Process -FilePath 'wsb.exe' -ArgumentList 'connect','--id',$sbxOn | Out-Null
  $loginOk2 = Wait-ExistingLogin -SbxId $sbxOn -TimeoutSec 75
  Write-Output ("ONLINE_EXISTING_LOGIN_READY=" + $loginOk2)
  if (-not $loginOk2) { throw 'Timeout waiting for ExistingLogin in Online Sandbox' }

  Start-Sleep -Seconds 3
  Write-Output '1/2 ONLINE_PRESENT_WEBVIEW2...'
  & wsb.exe exec --id $sbxOn -r ExistingLogin -c "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\gate1_ro\scripts\run_gate1_online_wv2_suite.ps1 -Phase present_online" --raw | Tee-Object -FilePath (Join-Path $evOn 'wsb_exec_online_present.log')

  Write-Output '2/2 PROBING_WEBVIEW2_UNINSTALL_ONLINE...'
  & wsb.exe exec --id $sbxOn -r System -c "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\gate1_ro\scripts\run_gate1_offline_nowv2.ps1 -Mode uninstall_wv2" --raw | Tee-Object -FilePath (Join-Path $evOn 'wsb_exec_online_wv2uninst.log')
} finally {
  Write-Output ("STOPPING_ONLINE_SANDBOX=" + $sbxOn)
  & wsb.exe stop --id $sbxOn --raw | Out-Null
}

Write-Output 'GATE1_ORCHESTRATOR_FINISHED'
