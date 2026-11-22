const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const crypto = require('crypto');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p5_4_evidence');
const SANDBOX = path.join(EV_DIR, 'sandbox');
const DIST_RELEASES = path.join(EV_DIR, 'dist_releases');

function sha256File(p) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(p));
  return h.digest('hex');
}

function copyDirSync(src, dst) {
  if (!fs.existsSync(src)) return;
  fs.mkdirSync(dst, { recursive: true });
  for (const ent of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, ent.name);
    const d = path.join(dst, ent.name);
    if (ent.isDirectory()) copyDirSync(s, d);
    else fs.copyFileSync(s, d);
  }
}

function scrubbedEnv(extra = {}) {
  return {
    SystemRoot: process.env.SystemRoot || 'C:\\Windows',
    WINDIR: process.env.WINDIR || 'C:\\Windows',
    COMSPEC: process.env.COMSPEC || 'C:\\Windows\\System32\\cmd.exe',
    PATHEXT: '.COM;.EXE;.BAT;.CMD',
    PATH: 'C:\\Windows\\System32;C:\\Windows;C:\\Windows\\System32\\WindowsPowerShell\\v1.0',
    TEMP: SANDBOX,
    TMP: SANDBOX,
    USERPROFILE: process.env.USERPROFILE,
    APPDATA: path.join(SANDBOX, 'AppData', 'Roaming'),
    LOCALAPPDATA: path.join(SANDBOX, 'AppData', 'Local'),
    ...extra,
  };
}

function runCommandChecked(exe, args, cwd, envOverride = null) {
  return execFileSync(exe, args, {
    cwd,
    encoding: 'utf8',
    env: envOverride || process.env,
  }).trim();
}

function runCommandExpectFail(exe, args, cwd, envOverride = null) {
  let out = '';
  try {
    out = execFileSync(exe, args, {
      cwd,
      encoding: 'utf8',
      stdio: 'pipe',
      env: envOverride || process.env,
    });
  } catch (e) {
    return `${e.stdout || ''} ${e.stderr || ''} ${e.message || ''}`.trim();
  }
  throw new Error(`Expected command to fail, but succeeded: ${out}`);
}

function runPsJson(script, envOverride = null) {
  const out = execFileSync(
    'powershell',
    ['-NoProfile', '-NonInteractive', '-Command', script],
    { encoding: 'utf8', cwd: ROOT, env: envOverride || process.env }
  ).trim();
  return JSON.parse(out);
}

async function waitForCdp(port, timeoutMs = 26000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const targets = await new Promise((resolve, reject) => {
        http
          .get(`http://127.0.0.1:${port}/json`, (res) => {
            let data = '';
            res.on('data', (c) => (data += c));
            res.on('end', () => {
              try {
                resolve(JSON.parse(data));
              } catch (e) {
                reject(e);
              }
            });
          })
          .on('error', reject);
      });
      const page = targets.find((t) => t.type === 'page' && t.url.startsWith('http://tauri.localhost'));
      if (page) return page;
    } catch (_) {}
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error(`Timed out waiting for WebView2 CDP on port ${port}`);
}

async function connectCdp(page, waitSelector = null) {
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let idCounter = 1;
  const pending = new Map();
  const consoleErrors = [];

  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = idCounter++;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });

  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(msg.error);
      else resolve(msg.result);
    } else if (msg.method === 'Runtime.exceptionThrown') {
      consoleErrors.push(msg.params?.exceptionDetails?.text || 'Runtime exception');
    }
  };

  await new Promise((r) => (ws.onopen = r));
  await send('Page.enable');
  await send('Runtime.enable');

  const evalJs = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(`Eval error: ${JSON.stringify(r.exceptionDetails)}`);
    return r.result.value;
  };

  const snap = async (filename, width, height) => {
    await send('Emulation.setDeviceMetricsOverride', {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await new Promise((r) => setTimeout(r, 400));
    const res = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(res.data, 'base64'));
    return { filename, width, height };
  };

  for (let i = 0; i < 65; i++) {
    const checkExpr = waitSelector
      ? `Boolean(window.__TAURI_INTERNALS__ && document.querySelector(${JSON.stringify(waitSelector)}))`
      : 'Boolean(window.__TAURI_INTERNALS__)';
    const ready = await evalJs(checkExpr);
    if (ready) break;
    await new Promise((r) => setTimeout(r, 200));
  }

  return { ws, evalJs, snap, consoleErrors };
}

function getAdbStatus() {
  let adbOut = '';
  try {
    adbOut = execFileSync(path.join(ROOT, 'bin', 'adb.exe'), ['devices', '-l'], { encoding: 'utf8' }).trim();
  } catch (e) {
    adbOut = String(e.message || e);
  }
  let ldRunning = 'unknown';
  try {
    ldRunning = execFileSync('C:\\LDPlayer\\LDPlayer14\\ldconsole.exe', ['isrunning', '--index', '0'], {
      encoding: 'utf8',
    }).trim();
  } catch (_) {}
  return { adbOut, ldRunning };
}

function findNsisInstaller() {
  const nsisDir = path.join(ROOT, 'target', 'release', 'bundle', 'nsis');
  if (!fs.existsSync(nsisDir)) throw new Error(`Missing NSIS bundle dir: ${nsisDir}`);
  const exes = fs.readdirSync(nsisDir).filter((f) => f.endsWith('-setup.exe'));
  if (exes.length === 0) throw new Error(`No *-setup.exe found in ${nsisDir}`);
  return path.join(nsisDir, exes[0]);
}

function createForwardSlashZip(srcDir, zipOut) {
  execFileSync(
    'python',
    [
      '-c',
      `import os, zipfile, sys
src, dst = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as zf:
  for r, _, files in os.walk(src):
    for fn in sorted(files):
      fp = os.path.join(r, fn)
      arc = os.path.relpath(fp, os.path.dirname(src)).replace('\\\\', '/')
      zf.write(fp, arc)
print('ZIP OK', os.path.getsize(dst))`,
      srcDir,
      zipOut,
    ],
    { encoding: 'utf8' }
  );
}

async function main() {
  fs.mkdirSync(EV_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) fs.rmSync(SANDBOX, { recursive: true, force: true });
  if (fs.existsSync(DIST_RELEASES)) fs.rmSync(DIST_RELEASES, { recursive: true, force: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Local'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Roaming'), { recursive: true });
  fs.mkdirSync(DIST_RELEASES, { recursive: true });

  const { adbOut, ldRunning } = getAdbStatus();
  const hostInfo = runPsJson(`
    $os = Get-CimInstance Win32_OperatingSystem;
    $wv = (Get-ItemProperty 'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv;
    [PSCustomObject]@{
      osCaption = $os.Caption
      osBuild = $os.BuildNumber
      arch = $env:PROCESSOR_ARCHITECTURE
      webview2Version = $wv
      windowsSandboxExists = (Test-Path "$env:SystemRoot\\System32\\WindowsSandbox.exe")
    } | ConvertTo-Json -Compress
  `);

  const nsisSetupSrc = findNsisInstaller();
  const nsisSetupDist = path.join(DIST_RELEASES, path.basename(nsisSetupSrc));
  fs.copyFileSync(nsisSetupSrc, nsisSetupDist);

  // 1. Install FarmerPro Studio silently using real NSIS Setup EXE into isolated per-user install folder
  // with sandboxed LOCALAPPDATA/APPDATA so host production data is completely untouched!
  const studioInstallDir = path.join(SANDBOX, 'InstalledStudio');
  const cleanEnv = scrubbedEnv({
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9284',
  });
  runCommandChecked(nsisSetupDist, ['/S', `/D=${studioInstallDir}`], SANDBOX, cleanEnv);
  await new Promise((r) => setTimeout(r, 1200));

  const installedStudioExe = path.join(studioInstallDir, 'FarmerPro.exe');
  if (!fs.existsSync(installedStudioExe)) {
    throw new Error(`NSIS install failed: missing ${installedStudioExe}`);
  }
  const installedFilesList = [];
  const walkInst = (d) => {
    for (const ent of fs.readdirSync(d, { withFileTypes: true })) {
      const fp = path.join(d, ent.name);
      if (ent.isDirectory()) walkInst(fp);
      else installedFilesList.push(path.relative(studioInstallDir, fp).replace(/\\/g, '/'));
    }
  };
  walkInst(studioInstallDir);
  if (installedFilesList.some((f) => f.toLowerCase().endsWith('signtool.exe'))) {
    throw new Error('POLICY VIOLATION: Windows SDK signtool.exe must NOT be bundled inside installer!');
  }

  // 2. Launch NSIS-installed FarmerPro.exe under Scrubbed PATH (NO Cargo/Rust/Node/Python/SDK on PATH!),
  // create TWO complete Mini Tools (Alpha UnsignedInternal vs Beta SelfSignedQa) via Studio!
  let qaThumb = null;
  let factoryData = null;
  let studioConsoleErrors = [];

  const studioChild = spawn(installedStudioExe, [], {
    cwd: studioInstallDir,
    env: cleanEnv,
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9284, 26000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors = consoleErrors;

    factoryData = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const appStatus = await inv('app_get_status');
      const toolchain = await inv('studio_discover_signing_toolchain');

      const pubA = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-zenith-alpha',
          displayName: 'Zenith Studio Alpha',
          organization: 'Alpha Zenith Systems Ltd.',
          copyright: 'Copyright (C) 2026 Alpha Zenith Systems Ltd.',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });

      const qaCert = await inv('studio_create_self_signed_qa_cert', {
        subjectCn: 'Beta Orion Games Studio'
      });

      const pubB = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-orion-beta',
          displayName: 'Beta Orion Games Studio',
          organization: 'Beta Orion Games Studio',
          copyright: 'Copyright (C) 2026 Beta Orion Games Studio',
          authenticodeCertRef: qaCert.certRef,
          releaseChannel: 'qa',
          signingPolicy: 'self_signed_qa'
        }
      });

      // Mini Tool 1: Portable Alpha (UnsignedInternal + ADB bundled)
      const p1 = await inv('studio_create_project', {
        req: {
          slug: 'portable-alpha',
          name: 'Zenith Daily Harvester',
          gamePackageId: 'com.zenith.daily',
          version: '1.0.0',
          exeName: 'Auto_PortableAlpha.exe',
          windowTitle: 'Zenith Daily Harvester v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: p1.id, accentHex: '#10B981', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: p1.id,
        workflowId: 'wf-zenith-1',
        name: 'Zenith Resource Sweep',
        category: 'Daily',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-zenith-1', name: 'Zenith Resource Sweep', status: 'published',
          graph: {
            nodes: [
              { id: 'n1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'n2', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'zenith_quota', valueType: 'number', value: 1200 } },
              { id: 'n3', type: 'flow.end', position: { x: 360, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'n1', sourcePort: 'flow', target: 'n2', targetPort: 'flow' },
              { id: 'e2', source: 'n2', sourcePort: 'flow', target: 'n3', targetPort: 'flow' }
            ]
          }
        })
      });
      await inv('studio_upsert_project_task', {
        projectId: p1.id,
        task: { id: 'tsk-z1', title: 'Zenith Quarry Collect', categoryId: 'cat-zn', workflowId: 'wf-zenith-1', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [] },
        categoryMeta: { id: 'cat-zn', shortTag: 'ZN', displayName: 'Zenith', colorHex: '#10B981' }
      });
      const vi1 = {
        companyName: pubA.organization,
        fileDescription: 'Zenith Daily Harvester Portable Runtime',
        fileVersion: '1.0.0.100',
        internalName: 'portable-alpha',
        legalCopyright: pubA.copyright,
        originalFilename: 'Auto_PortableAlpha.exe',
        productName: 'Zenith Daily Harvester',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: { id: p1.id, publisherProfileId: pubA.id, signingMode: 'unsigned_internal', versionInfoJson: JSON.stringify(vi1) }
      });
      const relOut1 = ${JSON.stringify(path.join(DIST_RELEASES, 'Portable_Alpha_v1.0.0_win-x64'))};
      const exp1 = await inv('studio_builder_export_release', {
        projectId: p1.id,
        outputDir: relOut1,
        options: { signingMode: 'unsigned_internal', publisherProfileId: pubA.id, versionInfo: vi1 }
      });
      const fv1 = await inv('studio_verify_final_release', { releaseDir: relOut1, requirePublicChain: false });

      // Mini Tool 2: Portable Beta (SelfSignedQa + Distinct Purple/Crimson Branding + ADB bundled)
      const p2 = await inv('studio_create_project', {
        req: {
          slug: 'portable-beta',
          name: 'Orion Citadel Manager',
          gamePackageId: 'com.orion.citadel',
          version: '1.0.0',
          exeName: 'Auto_PortableBeta.exe',
          windowTitle: 'Orion Citadel Manager v1.0.0',
          targetWidthPx: 1920,
          targetHeightPx: 1080,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: p2.id, accentHex: '#E11D48', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: p2.id,
        workflowId: 'wf-orion-1',
        name: 'Orion Citadel Patrol',
        category: 'Raid',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-orion-1', name: 'Orion Citadel Patrol', status: 'published',
          graph: {
            nodes: [
              { id: 'm1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'm2', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'orion_rank', valueType: 'string', value: 'Commander-V' } },
              { id: 'm3', type: 'flow.end', position: { x: 360, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'm1', sourcePort: 'flow', target: 'm2', targetPort: 'flow' },
              { id: 'e2', source: 'm2', sourcePort: 'flow', target: 'm3', targetPort: 'flow' }
            ]
          }
        })
      });
      await inv('studio_upsert_project_task', {
        projectId: p2.id,
        task: { id: 'tsk-o1', title: 'Orion Arena Blitz', categoryId: 'cat-or', workflowId: 'wf-orion-1', defaultEnabled: true, priority: 1, estimateSec: 25, settingsSchema: [] },
        categoryMeta: { id: 'cat-or', shortTag: 'OR', displayName: 'Orion', colorHex: '#E11D48' }
      });
      const ui2 = await inv('studio_get_ui_schema', { projectId: p2.id });
      ui2.themeAccentHex = '#E11D48';
      ui2.headerBgHex = '#2A1024';
      await inv('studio_save_ui_schema', { projectId: p2.id, schema: ui2 });

      const vi2 = {
        companyName: pubB.organization,
        fileDescription: 'Orion Citadel Manager QA Signed Portable',
        fileVersion: '1.0.0.200',
        internalName: 'portable-beta',
        legalCopyright: pubB.copyright,
        originalFilename: 'Auto_PortableBeta.exe',
        productName: 'Orion Citadel Manager',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: { id: p2.id, publisherProfileId: pubB.id, signingMode: 'self_signed_qa', versionInfoJson: JSON.stringify(vi2) }
      });
      const relOut2 = ${JSON.stringify(path.join(DIST_RELEASES, 'Portable_Beta_v1.0.0_win-x64'))};
      const exp2 = await inv('studio_builder_export_release', {
        projectId: p2.id,
        outputDir: relOut2,
        options: { signingMode: 'self_signed_qa', publisherProfileId: pubB.id, authenticodeCertRef: qaCert.certRef, versionInfo: vi2 }
      });
      const fv2 = await inv('studio_verify_final_release', { releaseDir: relOut2, requirePublicChain: false });

      // Key rotation v1.1.0 for Beta!
      const rotRes = await inv('studio_rotate_project_ed25519_key_with_delegation', {
        projectId: p2.id, publisherId: pubB.publisherId, validDays: 90
      });
      const vi2_11 = { ...vi2, fileVersion: '1.1.0.201', productVersion: '1.1.0' };
      await inv('studio_update_project', {
        req: { id: p2.id, version: '1.1.0', windowTitle: 'Orion Citadel Manager v1.1.0', versionInfoJson: JSON.stringify(vi2_11) }
      });
      const relOut2_11 = ${JSON.stringify(path.join(DIST_RELEASES, 'Portable_Beta_v1.1.0_win-x64'))};
      const exp2_11 = await inv('studio_builder_export_release', {
        projectId: p2.id,
        outputDir: relOut2_11,
        options: {
          signingMode: 'self_signed_qa',
          publisherProfileId: pubB.id,
          authenticodeCertRef: qaCert.certRef,
          versionInfo: vi2_11,
          keyRotationDelegation: rotRes.delegation
        }
      });

      return { appStatus, toolchain, qaCert, pubA, pubB, p1, p2, exp1, fv1, exp2, fv2, rotRes, exp2_11, relOut1, relOut2, relOut2_11 };
    })()`);

    qaThumb = factoryData?.qaCert?.thumbprint || null;

    // Reload Studio to select Orion Citadel & render Builder P5.4 UI cleanly
    await evalJs(`location.reload()`);
    await new Promise((r) => setTimeout(r, 1500));
    for (let i = 0; i < 40; i++) {
      const ready = await evalJs(`Boolean(window.__TAURI_INTERNALS__ && document.querySelectorAll('header select option').length > 1)`);
      if (ready) break;
      await new Promise((r) => setTimeout(r, 200));
    }
    await evalJs(`(() => {
      const sel = document.querySelector('header select');
      if (sel) {
        const opt = Array.from(sel.options).find(o => o.value === ${JSON.stringify(factoryData.p2.id)});
        if (opt) { sel.value = opt.value; sel.dispatchEvent(new Event('change', { bubbles: true })); }
      }
      const btn = Array.from(document.querySelectorAll('nav button, aside button')).find(b => b.textContent && b.textContent.includes('Mini Tool Builder'));
      if (btn) btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 900));
    await evalJs(`(() => {
      const sp = document.querySelector('[data-testid="p53-signing-preflight"]');
      if (sp) sp.click();
    })()`);
    await new Promise((r) => setTimeout(r, 700));
    await snap('p5_4_studio_factory_installed.png', 1600, 900);
    ws.close();
  } finally {
    try { studioChild.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 800));

  // Verify NSIS Uninstall preserves StudioData DB & GameProjects!
  const studioDataDb = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'StudioData', 'data', 'farmerpro.db');
  const studioDataProjects = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'StudioData', 'projects');
  const dbExistsBeforeUninstall = fs.existsSync(studioDataDb);
  const uninstallExe = path.join(studioInstallDir, 'uninstall.exe');
  if (fs.existsSync(uninstallExe)) {
    runCommandChecked(uninstallExe, ['/S'], SANDBOX, cleanEnv);
    await new Promise((r) => setTimeout(r, 1500));
  }
  const dbPreservedAfterUninstall = fs.existsSync(studioDataDb) && fs.existsSync(studioDataProjects);
  const studioExeRemovedAfterUninstall = !fs.existsSync(installedStudioExe);

  // 3. Create Portable ZIPs with forward-slash entries & extract to isolated "OtherMachine/" (ZERO Studio, Scrubbed PATH!)
  const zipAlpha = path.join(DIST_RELEASES, 'Portable_Alpha_v1.0.0_win-x64.zip');
  const zipBeta = path.join(DIST_RELEASES, 'Portable_Beta_v1.0.0_win-x64.zip');
  createForwardSlashZip(factoryData.relOut1, zipAlpha);
  createForwardSlashZip(factoryData.relOut2, zipBeta);

  const otherMachine = path.join(SANDBOX, 'OtherMachine');
  fs.mkdirSync(otherMachine, { recursive: true });
  execFileSync('powershell', ['-NoProfile', '-Command', `Expand-Archive -Path '${zipAlpha}' -DestinationPath '${otherMachine}' -Force`]);
  execFileSync('powershell', ['-NoProfile', '-Command', `Expand-Archive -Path '${zipBeta}' -DestinationPath '${otherMachine}' -Force`]);

  const alphaExtDir = path.join(otherMachine, 'Portable_Alpha_v1.0.0_win-x64');
  const betaExtDir = path.join(otherMachine, 'Portable_Beta_v1.0.0_win-x64');
  const alphaExtExe = path.join(alphaExtDir, 'Auto_PortableAlpha.exe');
  const betaExtExe = path.join(betaExtDir, 'Auto_PortableBeta.exe');

  // Check --check-deps under scrubbed PATH (no Rust/Node/Python/SDK)
  const depsAlpha = JSON.parse(runCommandChecked(alphaExtExe, ['--check-deps'], alphaExtDir, scrubbedEnv()));
  const depsBeta = JSON.parse(runCommandChecked(betaExtExe, ['--check-deps'], betaExtDir, scrubbedEnv()));

  // Launch BOTH independent Portable Mini Tools with Studio completely uninstalled & scrubbed PATH!
  let alphaRuntimeSnap = null;
  const childAlpha = spawn(alphaExtExe, [], {
    cwd: alphaExtDir,
    env: scrubbedEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9285' }),
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9285, 22000);
    const { ws, evalJs, snap } = await connectCdp(page, '#device-tbody td');
    await new Promise((r) => setTimeout(r, 450));
    alphaRuntimeSnap = await evalJs(`({ title: document.title, badge: document.querySelector('.badge-ver')?.textContent || '' })`);
    await snap('p5_4_portable_alpha_running.png', 740, 500);
    ws.close();
  } finally {
    try { childAlpha.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 500));

  let betaRuntimeSnap = null;
  const childBeta = spawn(betaExtExe, [], {
    cwd: betaExtDir,
    env: scrubbedEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9286' }),
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9286, 22000);
    const { ws, evalJs, snap } = await connectCdp(page, '#device-tbody td');
    await new Promise((r) => setTimeout(r, 450));
    betaRuntimeSnap = await evalJs(`({ title: document.title, badge: document.querySelector('.badge-ver')?.textContent || '' })`);
    await snap('p5_4_portable_beta_running.png', 740, 500);
    ws.close();
  } finally {
    try { childBeta.kill('SIGTERM'); } catch (_) {}
  }

  // 4. Verify external Win32 Authenticode + signtool verify /pa /v + VERSIONINFO on distributed EXEs!
  const signtoolExe = factoryData.toolchain?.discoveredPath;
  let signtoolVerifyBeta = 'SKIPPED';
  if (signtoolExe && fs.existsSync(signtoolExe)) {
    // Expect untrusted root on self-signed cert unless /pa uses default policy -> /v reports chain explicitly!
    try {
      signtoolVerifyBeta = execFileSync(signtoolExe, ['verify', '/pa', '/all', '/v', betaExtExe], {
        cwd: otherMachine,
        encoding: 'utf8',
        stdio: 'pipe',
      }).trim();
    } catch (e) {
      signtoolVerifyBeta = `${e.stdout || ''} ${e.stderr || ''}`.trim();
    }
  }

  const osVerify = runPsJson(`
    $a = Get-Item '${alphaExtExe.replace(/'/g, "''")}';
    $b = Get-Item '${betaExtExe.replace(/'/g, "''")}';
    $sa = Get-AuthenticodeSignature $a.FullName;
    $sb = Get-AuthenticodeSignature $b.FullName;
    [PSCustomObject]@{
      alphaCompany = $a.VersionInfo.CompanyName
      alphaAuth = $sa.Status.ToString()
      betaCompany = $b.VersionInfo.CompanyName
      betaAuth = $sb.Status.ToString()
      betaAuthSubject = if ($sb.SignerCertificate) { $sb.SignerCertificate.Subject } else { $null }
    } | ConvertTo-Json -Compress
  `);

  // 5. Read-Only Folder ACL test + Updater Rotation + Trust State Anti-Rollback CLI Check
  const roDir = path.join(SANDBOX, 'ReadOnlyProbe');
  copyDirSync(alphaExtDir, roDir);
  // Create file named `data` or lock ACL -> fallback to LOCALAPPDATA!
  fs.rmSync(path.join(roDir, 'data'), { recursive: true, force: true });
  fs.writeFileSync(path.join(roDir, 'data'), 'readonly_dir_blocker');
  const roDeps = JSON.parse(runCommandChecked(path.join(roDir, 'Auto_PortableAlpha.exe'), ['--check-deps'], roDir, scrubbedEnv()));

  const betaManagedInstall = path.join(SANDBOX, 'ManagedBeta');
  const updaterInBeta = path.join(factoryData.relOut2, 'updater', 'fp-updater.exe');
  // Use pristine release folder factoryData.relOut2 (not betaExtDir which already executed live and created data/)
  const instOut = JSON.parse(runCommandChecked(updaterInBeta, ['install-initial', '--install-root', betaManagedInstall, '--release-dir', factoryData.relOut2], otherMachine, scrubbedEnv()));
  const oldTrustJson = fs.readFileSync(path.join(betaManagedInstall, '.updater', 'trust-state.json'), 'utf8');
  const upgOut = JSON.parse(runCommandChecked(updaterInBeta, ['upgrade', '--install-root', betaManagedInstall, '--release-dir', factoryData.relOut2_11], otherMachine, scrubbedEnv()));

  // Anti-rollback attack test: restore epoch=1 trust-state.json over epoch=2 -> fp-updater reconcile/upgrade MUST REJECT!
  fs.writeFileSync(path.join(betaManagedInstall, '.updater', 'trust-state.json'), oldTrustJson, 'utf8');
  fs.writeFileSync(path.join(betaManagedInstall, '.updater', 'trust-state.bak.json'), oldTrustJson, 'utf8');
  const antiRollbackErr = runCommandExpectFail(
    updaterInBeta,
    ['rollback', '--install-root', betaManagedInstall],
    otherMachine,
    scrubbedEnv()
  );

  // Restore valid trust-state & cleanup test CNG cert
  if (qaThumb) {
    try {
      execFileSync('powershell', ['-NoProfile', '-Command', `Remove-Item 'Cert:\\CurrentUser\\My\\${qaThumb}' -ErrorAction SilentlyContinue`]);
    } catch (_) {}
  }

  const finalReport = {
    executedAtUtc: new Date().toISOString(),
    gitHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
    hostEnvironment: hostInfo,
    cleanWindowsVmOrSandboxStatus: hostInfo.windowsSandboxExists
      ? 'AVAILABLE'
      : 'BLOCKED (Windows Sandbox disabled / requires UAC Admin; no Windows VM present. Executed Scrubbed-PATH Isolated User Profile E2E without dev tools on PATH instead — NOT marked CLEAN_WINDOWS_PASS.)',
    publicSignedCommercialRelease: {
      status: 'BLOCKED (No commercial CA Authenticode cert on host; PublicSigned enforces TrustedPublicChain fail-closed)',
    },
    adbLiveHardware: {
      adbDevicesOutput: adbOut || '(0 devices)',
      ldplayer14Vm0IsRunning: ldRunning,
      status: 'BLOCKED (0 ADB online)',
    },
    nsisInstaller: {
      installerPath: nsisSetupDist,
      installerSizeBytes: fs.statSync(nsisSetupDist).size,
      installerSha256: sha256File(nsisSetupDist),
      installedFilesCount: installedFilesList.length,
      sdkBundled: installedFilesList.some((f) => f.toLowerCase().endsWith('signtool.exe')),
      dbExistsBeforeUninstall,
      dbPreservedAfterUninstall,
      studioExeRemovedAfterUninstall,
    },
    studioFactory: {
      consoleErrors: studioConsoleErrors,
      appStatus: factoryData.appStatus,
      toolchainDiscovery: factoryData.toolchain,
      finalVerifyAlpha: factoryData.fv1,
      finalVerifyBeta: factoryData.fv2,
      zipAlphaBytes: fs.statSync(zipAlpha).size,
      zipAlphaSha256: sha256File(zipAlpha),
      zipBetaBytes: fs.statSync(zipBeta).size,
      zipBetaSha256: sha256File(zipBeta),
    },
    portableCleanHostRun: {
      depsAlpha,
      depsBeta,
      roDepsFallbackUsed: roDeps.fallbackToLocalAppdataUsed,
      roDepsDbPath: roDeps.resolvedSqliteDbPath,
      alphaRuntimeSnap,
      betaRuntimeSnap,
      osVerify,
      signtoolVerifyBetaSnippet: signtoolVerifyBeta.slice(0, 600),
    },
    updaterAndAntiRollback: {
      instOut,
      upgOut,
      antiRollbackErr,
    },
  };

  fs.writeFileSync(path.join(EV_DIR, 'p5_4_e2e_report.json'), JSON.stringify(finalReport, null, 2), 'utf8');
  console.log(JSON.stringify({
    status: 'P5_4_E2E_COMPLETED',
    installerBytes: finalReport.nsisInstaller.installerSizeBytes,
    dbPreservedAfterUninstall,
    studioExeRemovedAfterUninstall,
    sdkBundled: finalReport.nsisInstaller.sdkBundled,
    finalVerifyAlphaOk: factoryData.fv1.verified,
    finalVerifyBetaOk: factoryData.fv2.verified,
    betaPubCertMatch: factoryData.fv2.publisherIdentityMatchesCert,
    roDepsFallbackUsed: roDeps.fallbackToLocalAppdataUsed,
    upgVersion: upgOut.activeVersion,
    antiRollbackBlocked: antiRollbackErr.includes('TRUST_STATE_ROLLBACK_DETECTED'),
  }, null, 2));

  if (
    !dbExistsBeforeUninstall ||
    !dbPreservedAfterUninstall ||
    !studioExeRemovedAfterUninstall ||
    finalReport.nsisInstaller.sdkBundled ||
    !factoryData.fv1.verified ||
    !factoryData.fv2.verified ||
    !factoryData.fv2.publisherIdentityMatchesCert ||
    !roDeps.fallbackToLocalAppdataUsed ||
    upgOut.activeVersion !== '1.1.0' ||
    !antiRollbackErr.includes('TRUST_STATE_ROLLBACK_DETECTED')
  ) {
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
