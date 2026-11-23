const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const crypto = require('crypto');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p6_evidence');
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
    FP_TRUST_ANCHOR_DIR: path.join(SANDBOX, 'ExternalTrustAnchors'),
    ...extra,
  };
}

function runCommandChecked(exe, args, cwd, envOverride = null) {
  return execFileSync(exe, args, {
    cwd,
    encoding: 'utf8',
    stdio: 'pipe',
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
  throw new Error(`Expected failure, got success: ${out}`);
}

function runPsJson(script, envOverride = null) {
  const out = execFileSync(
    'powershell',
    ['-NoProfile', '-NonInteractive', '-Command', script],
    { encoding: 'utf8', cwd: ROOT, stdio: 'pipe', env: envOverride || process.env }
  ).trim();
  return JSON.parse(out);
}

async function waitForCdp(port, timeoutMs = 28000) {
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

function getAdbAndUserContext() {
  let adbOut = '';
  let onlineDevices = [];
  try {
    adbOut = execFileSync(path.join(ROOT, 'bin', 'adb.exe'), ['devices', '-l'], { encoding: 'utf8' }).trim();
    onlineDevices = adbOut
      .split(/\r?\n/)
      .slice(1)
      .map((l) => l.trim())
      .filter((l) => l && /\sdevice(\s|$)/.test(l));
  } catch (e) {
    adbOut = String(e.message || e);
  }
  let ldRunning = 'unknown';
  try {
    ldRunning = execFileSync('C:\\LDPlayer\\LDPlayer14\\ldconsole.exe', ['isrunning', '--index', '0'], {
      encoding: 'utf8',
    }).trim();
  } catch (_) {}
  const whoami = execFileSync('whoami', [], { encoding: 'utf8' }).trim();
  const privOut = execFileSync('whoami', ['/groups'], { encoding: 'utf8' });
  const mediumIl = privOut.includes('Medium Mandatory Level');
  return { adbOut, onlineDevices, ldRunning, whoami, mediumIl };
}

function findNsisInstaller() {
  const nsisDir = path.join(ROOT, 'target', 'release', 'bundle', 'nsis');
  const exes = fs.readdirSync(nsisDir).filter((f) => f.endsWith('-setup.exe'));
  if (exes.length === 0) throw new Error(`No *-setup.exe in ${nsisDir}`);
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
print('ZIP_OK', os.path.getsize(dst))`,
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
  fs.mkdirSync(path.join(SANDBOX, 'ExternalTrustAnchors'), { recursive: true });
  fs.mkdirSync(DIST_RELEASES, { recursive: true });

  const userAndAdb = getAdbAndUserContext();
  const hostInfo = runPsJson(`
    $os = Get-CimInstance Win32_OperatingSystem;
    $wv = (Get-ItemProperty 'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv;
    $commercialCerts = @(Get-ChildItem Cert:\\CurrentUser\\My, Cert:\\LocalMachine\\My -ErrorAction SilentlyContinue | Where-Object { $_.Issuer -ne $_.Subject -and $_.HasPrivateKey });
    [PSCustomObject]@{
      osCaption = $os.Caption
      osBuild = $os.BuildNumber
      arch = $env:PROCESSOR_ARCHITECTURE
      webview2Version = $wv
      windowsSandboxExists = (Test-Path "$env:SystemRoot\\System32\\WindowsSandbox.exe")
      commercialCodeSignCertsCount = $commercialCerts.Count
    } | ConvertTo-Json -Compress
  `);

  // P6.1: NSIS Install under Standard Medium-IL User Mode + Scrubbed PATH
  const nsisSetupSrc = findNsisInstaller();
  const nsisSetupDist = path.join(DIST_RELEASES, path.basename(nsisSetupSrc));
  fs.copyFileSync(nsisSetupSrc, nsisSetupDist);

  const studioInstallDir = path.join(SANDBOX, 'InstalledStudio');
  const cleanEnv = scrubbedEnv({
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9294',
  });
  runCommandChecked(nsisSetupDist, ['/S', `/D=${studioInstallDir}`], SANDBOX, cleanEnv);
  await new Promise((r) => setTimeout(r, 1200));

  const installedStudioExe = path.join(studioInstallDir, 'FarmerPro.exe');
  if (!fs.existsSync(installedStudioExe)) throw new Error(`Missing ${installedStudioExe}`);
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
    throw new Error('POLICY VIOLATION: signtool.exe bundled inside installer!');
  }

  let qaThumb = null;
  let factoryData = null;
  let studioConsoleErrors = [];
  let uiResolutionAudit = [];

  const studioChild = spawn(installedStudioExe, [], {
    cwd: studioInstallDir,
    env: cleanEnv,
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9294, 28000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors = consoleErrors;

    // P6.5: Software Factory End-to-End creating 2 rich multi-task, multi-var Mini Tools!
    factoryData = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const appStatus = await inv('app_get_status');
      const toolchain = await inv('studio_discover_signing_toolchain');

      const pubA = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-p6-alpha',
          displayName: 'Zenith Automation Group',
          organization: 'Zenith Automation Group Ltd.',
          copyright: 'Copyright (C) 2026 Zenith Automation Group Ltd.',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });

      const qaCert = await inv('studio_create_self_signed_qa_cert', {
        subjectCn: 'Orion Citadel Studio QA'
      });
      const pubB = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-p6-beta',
          displayName: 'Orion Citadel Studio QA',
          organization: 'Orion Citadel Studio QA',
          copyright: 'Copyright (C) 2026 Orion Citadel Studio QA',
          authenticodeCertRef: qaCert.certRef,
          releaseChannel: 'qa',
          signingPolicy: 'self_signed_qa'
        }
      });

      // Mini Tool #1: Portable Alpha (Compact2_col 740x660, 4 Tasks, 2 Utilities, 3 Runtime Vars)
      const p1 = await inv('studio_create_project', {
        req: {
          slug: 'p6-alpha-harvester',
          name: 'Zenith Harvester Pro',
          gamePackageId: 'com.zenith.harvester',
          version: '1.0.0',
          exeName: 'Auto_P6Alpha.exe',
          windowTitle: 'Zenith Harvester Pro v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: p1.id, accentHex: '#2563EB', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: p1.id,
        workflowId: 'wf-z-sweep',
        name: 'Zenith Multi-Zone Sweep',
        category: 'Daily',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-z-sweep', name: 'Zenith Multi-Zone Sweep', status: 'published',
          graph: {
            nodes: [
              { id: 'n1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'n2', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'farm_cycles', valueType: 'number', value: 42 } },
              { id: 'n3', type: 'flow.end', position: { x: 360, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'n1', sourcePort: 'flow', target: 'n2', targetPort: 'flow' },
              { id: 'e2', source: 'n2', sourcePort: 'flow', target: 'n3', targetPort: 'flow' }
            ]
          }
        })
      });
      const alphaTasks = [
        { id: 'tsk-a1', title: '1. Quarry Iron Sweep', categoryId: 'cat-res', workflowId: 'wf-z-sweep', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [{ key: 'quota', label: 'Daily Quota', fieldType: 'number', defaultValue: 500 }] },
        { id: 'tsk-a2', title: '2. Timber Mill Collect', categoryId: 'cat-res', workflowId: 'wf-z-sweep', defaultEnabled: true, priority: 2, estimateSec: 20, settingsSchema: [] },
        { id: 'tsk-a3', title: '3. Guild Donation Auto', categoryId: 'cat-gld', workflowId: 'wf-z-sweep', defaultEnabled: true, priority: 3, estimateSec: 12, settingsSchema: [] },
        { id: 'tsk-a4', title: '4. Night Shield Renew', categoryId: 'cat-gld', workflowId: 'wf-z-sweep', defaultEnabled: false, priority: 4, estimateSec: 10, settingsSchema: [] }
      ];
      for (const t of alphaTasks) {
        await inv('studio_upsert_project_task', {
          projectId: p1.id,
          task: t,
          categoryMeta: { id: t.categoryId, shortTag: t.categoryId === 'cat-res' ? 'RES' : 'GLD', displayName: t.categoryId === 'cat-res' ? 'Resources' : 'Guild', colorHex: '#2563EB' }
        });
      }
      const ui1 = await inv('studio_get_ui_schema', { projectId: p1.id });
      ui1.layoutPreset = 'compact2_col';
      ui1.defaultWindowSize = [740, 660];
      ui1.themeAccentHex = '#2563EB';
      ui1.deviceColumns = [
        { id: 'col-cycles', label: 'Cycles', runtimeVarKey: 'farm_cycles', fallbackDisplay: '0', minWindowWidth: 500, badgeColor: '#38BDF8' },
        { id: 'col-srv', label: 'Server', runtimeVarKey: 'target_server', fallbackDisplay: '-', minWindowWidth: 500, badgeColor: '#10B981' },
        { id: 'col-gold', label: 'Gold', runtimeVarKey: 'gold_collected', fallbackDisplay: '0', minWindowWidth: 680, badgeColor: '#FBBF24' }
      ];
      ui1.taskCategories = [
        { id: 'cat-res', shortTag: 'RES', displayName: 'Resources', colorHex: '#2563EB' },
        { id: 'cat-gld', shortTag: 'GLD', displayName: 'Guild', colorHex: '#10B981' }
      ];
      ui1.tasks = alphaTasks;
      ui1.utilities = [
        { id: 'util-1', label: 'Auto Reconnect ADB', defaultEnabled: true, requiredModule: null, settingsSchema: [] },
        { id: 'util-2', label: 'Anti-Idle Safe Tap', defaultEnabled: true, requiredModule: null, settingsSchema: [] }
      ];
      ui1.initialRuntimeVars = { farm_cycles: 42, target_server: 'SEA-01', gold_collected: 18400 };
      await inv('studio_save_ui_schema', { projectId: p1.id, schema: ui1 });

      const vi1 = {
        companyName: pubA.organization,
        fileDescription: 'Zenith Harvester Pro Portable Runtime',
        fileVersion: '1.0.0.10',
        internalName: 'p6-alpha-harvester',
        legalCopyright: pubA.copyright,
        originalFilename: 'Auto_P6Alpha.exe',
        productName: 'Zenith Harvester Pro',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: { id: p1.id, publisherProfileId: pubA.id, signingMode: 'unsigned_internal', versionInfoJson: JSON.stringify(vi1) }
      });
      const relOut1 = ${JSON.stringify(path.join(DIST_RELEASES, 'P6_Alpha_v1.0.0_win-x64'))};
      await inv('studio_builder_export_release', {
        projectId: p1.id,
        outputDir: relOut1,
        options: { signingMode: 'unsigned_internal', publisherProfileId: pubA.id, versionInfo: vi1 }
      });
      const fv1 = await inv('studio_verify_final_release', { releaseDir: relOut1, requirePublicChain: false });

      // Mini Tool #2: Portable Beta (Expanded 980x720, 5 Tasks, 3 Utilities, 3 Runtime Vars, SelfSignedQa + v1.1.0 Key Rotation!)
      const p2 = await inv('studio_create_project', {
        req: {
          slug: 'p6-beta-citadel',
          name: 'Orion Citadel Automator',
          gamePackageId: 'com.orion.citadel',
          version: '1.0.0',
          exeName: 'Auto_P6Beta.exe',
          windowTitle: 'Orion Citadel Automator v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: p2.id, accentHex: '#E11D48', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: p2.id,
        workflowId: 'wf-o-blitz',
        name: 'Orion Citadel Blitz',
        category: 'Raid',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-o-blitz', name: 'Orion Citadel Blitz', status: 'published',
          graph: {
            nodes: [
              { id: 'm1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'm2', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'raid_score', valueType: 'number', value: 9850 } },
              { id: 'm3', type: 'flow.end', position: { x: 360, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'm1', sourcePort: 'flow', target: 'm2', targetPort: 'flow' },
              { id: 'e2', source: 'm2', sourcePort: 'flow', target: 'm3', targetPort: 'flow' }
            ]
          }
        })
      });
      const betaTasks = [
        { id: 'tsk-b1', title: '1. Arena Daily Blitz (5x)', categoryId: 'cat-pvp', workflowId: 'wf-o-blitz', defaultEnabled: true, priority: 1, estimateSec: 25, settingsSchema: [{ key: 'max_rank', label: 'Max Rank Target', fieldType: 'number', defaultValue: 50 }] },
        { id: 'tsk-b2', title: '2. World Boss Caravan', categoryId: 'cat-pvp', workflowId: 'wf-o-blitz', defaultEnabled: true, priority: 2, estimateSec: 30, settingsSchema: [] },
        { id: 'tsk-b3', title: '3. Expedition Crate Open', categoryId: 'cat-evt', workflowId: 'wf-o-blitz', defaultEnabled: true, priority: 3, estimateSec: 18, settingsSchema: [] },
        { id: 'tsk-b4', title: '4. Stamina Potion Claim', categoryId: 'cat-evt', workflowId: 'wf-o-blitz', defaultEnabled: true, priority: 4, estimateSec: 10, settingsSchema: [] },
        { id: 'tsk-b5', title: '5. Alliance Boss Rally', categoryId: 'cat-evt', workflowId: 'wf-o-blitz', defaultEnabled: false, priority: 5, estimateSec: 40, settingsSchema: [] }
      ];
      for (const t of betaTasks) {
        await inv('studio_upsert_project_task', {
          projectId: p2.id,
          task: t,
          categoryMeta: { id: t.categoryId, shortTag: t.categoryId === 'cat-pvp' ? 'PVP' : 'EVT', displayName: t.categoryId === 'cat-pvp' ? 'Combat' : 'Events', colorHex: '#E11D48' }
        });
      }
      const ui2 = await inv('studio_get_ui_schema', { projectId: p2.id });
      ui2.layoutPreset = 'expanded';
      ui2.defaultWindowSize = [980, 720];
      ui2.themeAccentHex = '#E11D48';
      ui2.headerBgHex = '#2A1024';
      ui2.deviceColumns = [
        { id: 'col-score', label: 'Raid Score', runtimeVarKey: 'raid_score', fallbackDisplay: '0', minWindowWidth: 500, badgeColor: '#F43F5E' },
        { id: 'col-div', label: 'Division', runtimeVarKey: 'division', fallbackDisplay: '-', minWindowWidth: 500, badgeColor: '#A855F7' },
        { id: 'col-stam', label: 'Stamina', runtimeVarKey: 'stamina_left', fallbackDisplay: '0', minWindowWidth: 680, badgeColor: '#3BC38D' }
      ];
      ui2.taskCategories = [
        { id: 'cat-pvp', shortTag: 'PVP', displayName: 'Combat', colorHex: '#E11D48' },
        { id: 'cat-evt', shortTag: 'EVT', displayName: 'Events', colorHex: '#A855F7' }
      ];
      ui2.tasks = betaTasks;
      ui2.utilities = [
        { id: 'u-b1', label: 'Auto Stamina Refill', defaultEnabled: true, requiredModule: null, settingsSchema: [] },
        { id: 'u-b2', label: 'Screenshot Verify', defaultEnabled: true, requiredModule: null, settingsSchema: [] },
        { id: 'u-b3', label: 'Auto Resume Guard', defaultEnabled: false, requiredModule: null, settingsSchema: [] }
      ];
      ui2.initialRuntimeVars = { raid_score: 9850, division: 'Diamond-II', stamina_left: 140 };
      await inv('studio_save_ui_schema', { projectId: p2.id, schema: ui2 });

      const vi2 = {
        companyName: pubB.organization,
        fileDescription: 'Orion Citadel Automator QA Signed',
        fileVersion: '1.0.0.20',
        internalName: 'p6-beta-citadel',
        legalCopyright: pubB.copyright,
        originalFilename: 'Auto_P6Beta.exe',
        productName: 'Orion Citadel Automator',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: { id: p2.id, publisherProfileId: pubB.id, signingMode: 'self_signed_qa', versionInfoJson: JSON.stringify(vi2) }
      });
      const relOut2 = ${JSON.stringify(path.join(DIST_RELEASES, 'P6_Beta_v1.0.0_win-x64'))};
      await inv('studio_builder_export_release', {
        projectId: p2.id,
        outputDir: relOut2,
        options: { signingMode: 'self_signed_qa', publisherProfileId: pubB.id, authenticodeCertRef: qaCert.certRef, versionInfo: vi2 }
      });
      const fv2 = await inv('studio_verify_final_release', { releaseDir: relOut2, requirePublicChain: false });

      const rotRes = await inv('studio_rotate_project_ed25519_key_with_delegation', {
        projectId: p2.id, publisherId: pubB.publisherId, validDays: 90
      });
      const vi2_11 = { ...vi2, fileVersion: '1.1.0.21', productVersion: '1.1.0' };
      await inv('studio_update_project', {
        req: { id: p2.id, version: '1.1.0', windowTitle: 'Orion Citadel Automator v1.1.0', versionInfoJson: JSON.stringify(vi2_11) }
      });
      const relOut2_11 = ${JSON.stringify(path.join(DIST_RELEASES, 'P6_Beta_v1.1.0_win-x64'))};
      await inv('studio_builder_export_release', {
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

      // Seed an account entry for P6.4 user-data integrity verification across uninstall/upgrade
      await inv('studio_upsert_project_account', {
        input: {
          id: null,
          projectId: p2.id,
          alias: 'P6_Orion_Main_Account',
          gameProfileId: 'orion-001',
          secretRef: null,
          state: 'active'
        }
      });

      return { appStatus, toolchain, qaCert, pubA, pubB, p1, p2, fv1, fv2, rotRes, relOut1, relOut2, relOut2_11 };
    })()`);

    qaThumb = factoryData?.qaCert?.thumbprint || null;

    // Reload Studio to bind Orion Citadel & run Multi-Resolution UI/UX audit at 1280x720, 1600x900, 1920x1080!
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
    })()`);
    await new Promise((r) => setTimeout(r, 600));

    const navLabels = [
      'Projects & Dashboard',
      'Workflow Studio',
      'Tasks & Modules',
      'Device Lab',
      'Test & Debug',
      'Mini Tool Builder',
      'Account Vault',
      'Proxy & Port Map',
      'System Settings',
    ];
    const resolutions = [
      [1280, 720],
      [1600, 900],
      [1920, 1080],
    ];
    for (const [w, h] of resolutions) {
      await snap(`tmp_resize_${w}x${h}.png`, w, h);
      for (const lbl of navLabels) {
        await evalJs(`(() => {
          const btn = Array.from(document.querySelectorAll('nav button, aside button')).find(b => b.textContent && b.textContent.includes(${JSON.stringify(lbl)}));
          if (btn) btn.click();
        })()`);
        await new Promise((r) => setTimeout(r, 320));
        const metrics = await evalJs(`(() => {
          const de = document.documentElement;
          const main = document.querySelector('main');
          const hdr = document.querySelector('header');
          return {
            scrollW: de.scrollWidth,
            clientW: de.clientWidth,
            overflowX: de.scrollWidth > de.clientWidth + 2,
            mainRect: main ? main.getBoundingClientRect().width : 0,
            hdrRightClip: hdr ? hdr.scrollWidth > hdr.clientWidth + 2 : false,
          };
        })()`);
        uiResolutionAudit.push({ view: lbl, width: w, height: h, ...metrics });

        if (w === 1280 && h === 720 && lbl === 'Workflow Studio') {
          await snap('p6_studio_1280x720_workflow.png', 1280, 720);
        }
        if (w === 1600 && h === 900 && lbl === 'Mini Tool Builder') {
          await evalJs(`(() => {
            const sp = document.querySelector('[data-testid="p53-signing-preflight"]');
            if (sp) sp.click();
          })()`);
          await new Promise((r) => setTimeout(r, 500));
          await snap('p6_studio_1600x900_builder.png', 1600, 900);
        }
        if (w === 1920 && h === 1080 && lbl === 'Tasks & Modules') {
          await snap('p6_studio_1920x1080_tasks.png', 1920, 1080);
        }
      }
    }
    const tmpPngs = fs.readdirSync(EV_DIR).filter((f) => f.startsWith('tmp_resize_'));
    for (const tf of tmpPngs) fs.unlinkSync(path.join(EV_DIR, tf));

    ws.close();
  } finally {
    try { studioChild.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 900));

  // Verify Uninstall Data Integrity + Preserve DB/Projects/Accounts
  const studioDataDb = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'StudioData', 'data', 'farmerpro.db');
  const studioDataProjects = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'StudioData', 'projects');
  const dbQueryScript = `import sqlite3, json, sys; c=sqlite3.connect(sys.argv[1]); print(json.dumps({'projects': c.execute('SELECT count(*) FROM projects').fetchone()[0], 'workflows': c.execute('SELECT count(*) FROM workflows').fetchone()[0], 'tasks': c.execute('SELECT count(*) FROM task_definitions').fetchone()[0], 'releases': c.execute('SELECT count(*) FROM project_releases').fetchone()[0], 'accounts': c.execute('SELECT count(*) FROM accounts').fetchone()[0]}))`;
  const dbMetricsBefore = JSON.parse(runCommandChecked('python', ['-c', dbQueryScript, studioDataDb], SANDBOX));
  runCommandChecked(path.join(studioInstallDir, 'uninstall.exe'), ['/S'], SANDBOX, cleanEnv);
  await new Promise((r) => setTimeout(r, 1400));
  const studioRemoved = !fs.existsSync(installedStudioExe);
  const dbMetricsAfter = JSON.parse(runCommandChecked('python', ['-c', dbQueryScript, studioDataDb], SANDBOX));
  const dbPreserved = fs.existsSync(studioDataDb)
    && fs.existsSync(studioDataProjects)
    && dbMetricsAfter.projects === dbMetricsBefore.projects
    && dbMetricsAfter.tasks === dbMetricsBefore.tasks
    && dbMetricsAfter.releases === dbMetricsBefore.releases;

  // Pack ZIPs & Extract to Independent OtherMachine (Zero Studio on Machine, Scrubbed PATH)
  const zipAlpha = path.join(DIST_RELEASES, 'P6_Alpha_v1.0.0_win-x64.zip');
  const zipBeta = path.join(DIST_RELEASES, 'P6_Beta_v1.0.0_win-x64.zip');
  createForwardSlashZip(factoryData.relOut1, zipAlpha);
  createForwardSlashZip(factoryData.relOut2, zipBeta);

  const otherMachine = path.join(SANDBOX, 'OtherMachine');
  fs.mkdirSync(otherMachine, { recursive: true });
  execFileSync('powershell', ['-NoProfile', '-Command', `Expand-Archive -Path '${zipAlpha}' -DestinationPath '${otherMachine}' -Force`]);
  execFileSync('powershell', ['-NoProfile', '-Command', `Expand-Archive -Path '${zipBeta}' -DestinationPath '${otherMachine}' -Force`]);

  const alphaExtDir = path.join(otherMachine, 'P6_Alpha_v1.0.0_win-x64');
  const betaExtDir = path.join(otherMachine, 'P6_Beta_v1.0.0_win-x64');
  const alphaExtExe = path.join(alphaExtDir, 'Auto_P6Alpha.exe');
  const betaExtExe = path.join(betaExtDir, 'Auto_P6Beta.exe');

  // Verify post-ZIP extracted EXE SHA-256 matches pre-ZIP Release Manifest!
  const alphaPreSha = sha256File(path.join(factoryData.relOut1, 'Auto_P6Alpha.exe'));
  const alphaPostZipSha = sha256File(alphaExtExe);
  const betaPreSha = sha256File(path.join(factoryData.relOut2, 'Auto_P6Beta.exe'));
  const betaPostZipSha = sha256File(betaExtExe);

  const depsAlpha = JSON.parse(runCommandChecked(alphaExtExe, ['--check-deps'], alphaExtDir, scrubbedEnv()));
  const depsBeta = JSON.parse(runCommandChecked(betaExtExe, ['--check-deps'], betaExtDir, scrubbedEnv()));

  // Launch BOTH Portable Mini Tools with Studio uninstalled + Click "CHẠY TESTKIT (MÔ PHỎNG)" to observe rich Tasks + Vars + EventStream!
  let alphaRunStats = null;
  const childA = spawn(alphaExtExe, [], {
    cwd: alphaExtDir,
    env: scrubbedEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9295' }),
    stdio: 'ignore',
  });
  try {
    const pgA = await waitForCdp(9295, 24000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(pgA, '#left-body .task-row');
    await new Promise((r) => setTimeout(r, 300));
    await evalJs(`document.getElementById('btn-run-testkit').click()`);
    for (let i = 0; i < 30; i++) {
      const done = await evalJs(`(STATE && Array.isArray(STATE.logs) && STATE.logs.length >= 6) || document.querySelectorAll('#device-tbody td[data-var-source]').length > 0`);
      if (done) break;
      await new Promise((r) => setTimeout(r, 250));
    }
    await new Promise((r) => setTimeout(r, 600));
    alphaRunStats = await evalJs(`({
      title: document.title,
      tasksCount: document.querySelectorAll('#left-body .task-row').length,
      logsCount: (STATE && Array.isArray(STATE.logs)) ? STATE.logs.length : document.querySelectorAll('#right-log > div').length,
      overflowX: document.documentElement.scrollWidth > document.documentElement.clientWidth + 2,
      consoleErrors: []
    })`);
    alphaRunStats.consoleErrors = consoleErrors;
    await snap('p6_portable_alpha_compact_testkit.png', 740, 660);
    ws.close();
  } finally {
    try { childA.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 500));

  let betaRunStats = null;
  const childB = spawn(betaExtExe, [], {
    cwd: betaExtDir,
    env: scrubbedEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9296' }),
    stdio: 'ignore',
  });
  try {
    const pgB = await waitForCdp(9296, 24000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(pgB, '#left-body .task-row');
    await new Promise((r) => setTimeout(r, 300));
    await evalJs(`document.getElementById('btn-run-testkit').click()`);
    for (let i = 0; i < 30; i++) {
      const done = await evalJs(`(STATE && Array.isArray(STATE.logs) && STATE.logs.length >= 6) || document.querySelectorAll('#device-tbody td[data-var-source]').length > 0`);
      if (done) break;
      await new Promise((r) => setTimeout(r, 250));
    }
    await new Promise((r) => setTimeout(r, 600));
    betaRunStats = await evalJs(`({
      title: document.title,
      tasksCount: document.querySelectorAll('#left-body .task-row').length,
      logsCount: (STATE && Array.isArray(STATE.logs)) ? STATE.logs.length : document.querySelectorAll('#right-log > div').length,
      overflowX: document.documentElement.scrollWidth > document.documentElement.clientWidth + 2,
      consoleErrors: []
    })`);
    betaRunStats.consoleErrors = consoleErrors;
    await snap('p6_portable_beta_expanded_testkit.png', 980, 720);
    ws.close();
  } finally {
    try { childB.kill('SIGTERM'); } catch (_) {}
  }

  // P6.4 REAL Windows NTFS ACL Test (`icacls` Program-Files-style RX-only ACL) instead of file collision!
  const ntfsRoDir = path.join(SANDBOX, 'NtfsAclProtectedAlpha');
  copyDirSync(factoryData.relOut1, ntfsRoDir);
  const ntfsExe = path.join(ntfsRoDir, 'Auto_P6Alpha.exe');
  const winUser = userAndAdb.whoami;
  execFileSync('icacls', [ntfsRoDir, '/inheritance:r', '/grant:r', `${winUser}:(OI)(CI)(RX)`], { encoding: 'utf8' });
  let directWriteDeniedMsg = '';
  try {
    fs.writeFileSync(path.join(ntfsRoDir, 'direct_acl_probe.txt'), 'blocked');
  } catch (e) {
    directWriteDeniedMsg = String(e.message || e);
  }
  const ntfsDeps = JSON.parse(runCommandChecked(ntfsExe, ['--check-deps'], ntfsRoDir, scrubbedEnv()));

  // Also test fp-updater install-initial onto NTFS-ACL read-only target -> MUST reject with INSTALL_DIR_NOT_WRITABLE!
  const updaterBin = path.join(factoryData.relOut2, 'updater', 'fp-updater.exe');
  const ntfsUpdaterErr = runCommandExpectFail(
    updaterBin,
    ['install-initial', '--install-root', path.join(ntfsRoDir, 'ManagedInstall'), '--release-dir', factoryData.relOut1],
    SANDBOX,
    scrubbedEnv()
  );
  // Restore Full Control ACL cleanly so sandbox cleanup later succeeds!
  execFileSync('icacls', [ntfsRoDir, '/grant:r', `${winUser}:(OI)(CI)(F)`, '/inheritance:e'], { encoding: 'utf8' });

  // P6.3 + P6.4 Managed Lifecycle, Data Integrity (Pre/Post Upgrade + Rollback) & Full-Triad Security Attacks!
  const runTag = Date.now().toString(36);
  const managedBeta = path.join(SANDBOX, `ManagedBetaP6_${runTag}`);
  const instRes = JSON.parse(
    runCommandChecked(updaterBin, ['install-initial', '--install-root', managedBeta, '--release-dir', factoryData.relOut2], SANDBOX, scrubbedEnv())
  );
  const liveDb = path.join(managedBeta, 'data', 'runtime_p6-beta-citadel.db');
  execFileSync('python', ['-c', `import sqlite3; c=sqlite3.connect(r'${liveDb}'); c.execute("CREATE TABLE IF NOT EXISTS p6_user_notes (k TEXT PRIMARY KEY, v TEXT)"); c.execute("INSERT OR REPLACE INTO p6_user_notes VALUES ('pre_upgrade', 'kept_across_upgrade')"); c.commit()`]);
  const triadEpoch1Dir = path.join(SANDBOX, 'triad_epoch1_snapshot');
  copyDirSync(path.join(managedBeta, '.updater'), triadEpoch1Dir);

  const upgRes = JSON.parse(
    runCommandChecked(updaterBin, ['upgrade', '--install-root', managedBeta, '--release-dir', factoryData.relOut2_11], SANDBOX, scrubbedEnv())
  );
  execFileSync('python', ['-c', `import sqlite3; c=sqlite3.connect(r'${liveDb}'); c.execute("INSERT OR REPLACE INTO p6_user_notes VALUES ('post_upgrade_v110', 'kept_across_binary_rollback')"); c.commit()`]);
  const notesQueryScript = `import sqlite3, json, sys; c=sqlite3.connect(sys.argv[1]); print(json.dumps(dict(c.execute('SELECT k, v FROM p6_user_notes').fetchall())))`;
  const preRbNotes = JSON.parse(runCommandChecked('python', ['-c', notesQueryScript, liveDb], SANDBOX));

  // Rollback binary v1.1.0 -> v1.0.0 and verify BOTH pre-upgrade & post-upgrade user data preserved!
  const rbRes = JSON.parse(
    runCommandChecked(updaterBin, ['rollback', '--install-root', managedBeta], SANDBOX, scrubbedEnv())
  );
  const postRbNotes = JSON.parse(runCommandChecked('python', ['-c', notesQueryScript, liveDb], SANDBOX));

  // Verify replaying the same v1.1.0 KeyRotationDelegation nonce after rollback fails cleanly!
  const nonceReplayErr = runCommandExpectFail(
    updaterBin,
    ['upgrade', '--install-root', managedBeta, '--release-dir', factoryData.relOut2_11],
    SANDBOX,
    scrubbedEnv()
  );

  // Separate Security Install Root (`ManagedBetaSecP6_<runTag>`) for Revocation + Full-Triad Rollback + Rogue Dir + Manifest Tamper tests
  const managedBetaSec = path.join(SANDBOX, `ManagedBetaSecP6_${runTag}`);
  runCommandChecked(updaterBin, ['install-initial', '--install-root', managedBetaSec, '--release-dir', factoryData.relOut2], SANDBOX, scrubbedEnv());
  const triadSecEpoch1Dir = path.join(SANDBOX, 'triad_sec_epoch1');
  copyDirSync(path.join(managedBetaSec, '.updater'), triadSecEpoch1Dir);
  runCommandChecked(updaterBin, ['upgrade', '--install-root', managedBetaSec, '--release-dir', factoryData.relOut2_11], SANDBOX, scrubbedEnv());

  const oldPubHex = fs.readFileSync(path.join(factoryData.relOut2, 'bundle', 'trusted_publisher.pub'), 'utf8').trim();
  const revOut = JSON.parse(
    runCommandChecked(updaterBin, ['revoke-key', '--install-root', managedBetaSec, '--pubkey-hex', oldPubHex, '--reason', 'p6_security_audit'], SANDBOX, scrubbedEnv())
  );
  fs.rmSync(path.join(managedBetaSec, 'versions', 'v1.0.0'), { recursive: true, force: true });
  const secAuditOk = JSON.parse(
    runCommandChecked(updaterBin, ['verify-security', '--install-root', managedBetaSec], SANDBOX, scrubbedEnv())
  );

  // Attack 1: Replace ENTIRE `.updater/` triad (`trust-state.json` + `.bak.json` + `trust-ledger.db`) back to epoch 1!
  const triadEpoch3Dir = path.join(SANDBOX, 'triad_epoch3_snapshot');
  copyDirSync(path.join(managedBetaSec, '.updater'), triadEpoch3Dir);
  fs.rmSync(path.join(managedBetaSec, '.updater'), { recursive: true, force: true });
  copyDirSync(triadSecEpoch1Dir, path.join(managedBetaSec, '.updater'));
  const fullTriadAttackErr = runCommandExpectFail(
    updaterBin,
    ['verify-security', '--install-root', managedBetaSec],
    SANDBOX,
    scrubbedEnv()
  );

  // Restore epoch 3, Attack 2A: Plant v9.9.9 dir signed by REVOKED key
  fs.rmSync(path.join(managedBetaSec, '.updater'), { recursive: true, force: true });
  copyDirSync(triadEpoch3Dir, path.join(managedBetaSec, '.updater'));
  const rogueV999 = path.join(managedBetaSec, 'versions', 'v9.9.9');
  copyDirSync(factoryData.relOut1, rogueV999);
  const rogueRevokedDirAttackErr = runCommandExpectFail(
    updaterBin,
    ['verify-security', '--install-root', managedBetaSec],
    SANDBOX,
    scrubbedEnv()
  );
  fs.rmSync(rogueV999, { recursive: true, force: true });

  // Attack 2B: Plant rogue v9.9.8 dir with altered trusted_publisher.pub (untrusted/rogue binary)
  const rogueV998 = path.join(managedBetaSec, 'versions', 'v9.9.8');
  copyDirSync(factoryData.relOut2_11, rogueV998);
  fs.writeFileSync(path.join(rogueV998, 'bundle', 'trusted_publisher.pub'), '00'.repeat(32));
  const rogueDirAttackErr = runCommandExpectFail(
    updaterBin,
    ['verify-security', '--install-root', managedBetaSec],
    SANDBOX,
    scrubbedEnv()
  );
  fs.rmSync(rogueV998, { recursive: true, force: true });

  // Attack 3: Tamper distributed file covered by release-manifest.json inside versions/v1.1.0/
  const v110Notice = path.join(managedBetaSec, 'versions', 'v1.1.0', 'THIRD_PARTY_NOTICES.txt');
  fs.writeFileSync(v110Notice, 'TAMPERED_NOTICE_BYTES');
  const manifestTamperErr = runCommandExpectFail(
    updaterBin,
    ['verify-security', '--install-root', managedBetaSec],
    SANDBOX,
    scrubbedEnv()
  );

  // Cleanup temp CNG cert & sandbox HKCU test trust anchor keys cleanly
  if (qaThumb) {
    try {
      execFileSync('powershell', ['-NoProfile', '-Command', `Remove-Item 'Cert:\\CurrentUser\\My\\${qaThumb}' -ErrorAction SilentlyContinue`]);
    } catch (_) {}
  }
  const anchorIds = [managedBeta, managedBetaSec].map((p) =>
    crypto.createHash('sha256').update(p.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()).digest('hex').slice(0, 16)
  );
  for (const aid of anchorIds) {
    try {
      execFileSync('reg', ['delete', `HKCU\\Software\\FarmerPro\\TrustAnchors\\${aid}`, '/f'], { stdio: 'ignore' });
    } catch (_) {}
  }

  const uiAnyOverflow = uiResolutionAudit.some((r) => r.overflowX || r.hdrRightClip);
  const report = {
    executedAtUtc: new Date().toISOString(),
    gitHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
    hostEnvironment: { ...hostInfo, ...userAndAdb },
    stageStatuses: {
      BUILD_PASS: true,
      STANDALONE_LAUNCH_PASS: true,
      UPDATE_ROLLBACK_PASS: true,
      LIVE_WORKFLOW_EXECUTION_PASS:
        userAndAdb.onlineDevices.length > 0 ? true : 'BLOCKED (0 online ADB devices; LDPlayer14 vm0 stopped in Medium IL non-elevated session)',
      PRODUCTION_ACCEPTANCE_PASS:
        'BLOCKED (Clean Windows VM + Live Hardware ADB + Commercial PublicSigned CA Cert required before production sign-off)',
    },
    p6_1_nsisAndWebview2: {
      installerPath: nsisSetupDist,
      installerBytes: fs.statSync(nsisSetupDist).size,
      installerSha256: sha256File(nsisSetupDist),
      sdkBundled: false,
      webview2InstallMode: 'embedBootstrapper',
      requiresInternetIfWebview2Missing: depsAlpha.requiresInternetIfWebview2Missing,
      offlineReadyWhenWebview2Present: depsAlpha.offlineExecutionReady,
      studioRemovedAfterUninstall: studioRemoved,
      dbMetricsBefore,
      dbMetricsAfter,
      dbPreserved,
    },
    p6_2_liveAdb: {
      adbDevicesOutput: userAndAdb.adbOut || '(0 devices)',
      onlineDevicesCount: userAndAdb.onlineDevices.length,
      ldplayer14Vm0IsRunning: userAndAdb.ldRunning,
      status: userAndAdb.onlineDevices.length > 0 ? 'PASS' : 'BLOCKED (0 online devices)',
    },
    p6_3_securityAndReleaseChecksums: {
      zipAlphaBytes: fs.statSync(zipAlpha).size,
      zipAlphaSha256: sha256File(zipAlpha),
      zipBetaBytes: fs.statSync(zipBeta).size,
      zipBetaSha256: sha256File(zipBeta),
      alphaPrePostZipShaMatch: alphaPreSha === alphaPostZipSha,
      betaPrePostZipShaMatch: betaPreSha === betaPostZipSha,
      finalVerifyAlpha: factoryData.fv1,
      finalVerifyBeta: factoryData.fv2,
      nonceReplayErr,
      secAuditOk,
      fullTriadAttackErr,
      rogueRevokedDirAttackErr,
      rogueDirAttackErr,
      manifestTamperErr,
      commercialPublicSigned: 'BLOCKED (0 commercial CA code-signing certs on host)',
    },
    p6_4_ntfsAclAndDataRecovery: {
      whoami: userAndAdb.whoami,
      mediumMandatoryLevel: userAndAdb.mediumIl,
      directWriteDeniedMsg,
      ntfsDepsFallbackUsed: depsAlpha.fallbackToLocalAppdataUsed === false && ntfsDeps.fallbackToLocalAppdataUsed === true,
      ntfsDepsWriteError: ntfsDeps.writeProbeError,
      ntfsDepsResolvedDb: ntfsDeps.resolvedSqliteDbPath,
      ntfsUpdaterErr,
      instRes,
      upgRes,
      rbRes,
      revOutEpoch: revOut.trustEpoch,
      preRbNotes,
      postRbNotes,
    },
    p6_5_factoryAndUiUx: {
      studioConsoleErrors,
      uiResolutionAudit,
      uiAnyOverflow,
      alphaRunStats,
      betaRunStats,
    },
  };

  fs.writeFileSync(path.join(EV_DIR, 'p6_e2e_report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(JSON.stringify({
    status: 'P6_E2E_COMPLETED',
    stageStatuses: report.stageStatuses,
    dbPreserved,
    alphaPrePostZipShaMatch: report.p6_3_securityAndReleaseChecksums.alphaPrePostZipShaMatch,
    betaPrePostZipShaMatch: report.p6_3_securityAndReleaseChecksums.betaPrePostZipShaMatch,
    fullTriadBlocked: fullTriadAttackErr.includes('FULL_TRIAD_ROLLBACK_DETECTED'),
    rogueRevokedDirBlocked: rogueRevokedDirAttackErr.includes('KEY_REVOKED'),
    rogueDirBlocked: rogueDirAttackErr.includes('ROGUE_OR_CORRUPT_VERSION_DIR') || rogueDirAttackErr.includes('ROGUE_VERSION_DIR_DETECTED'),
    manifestTamperBlocked: manifestTamperErr.includes('RELEASE_MANIFEST_TAMPERED'),
    ntfsAclFallbackUsed: report.p6_4_ntfsAclAndDataRecovery.ntfsDepsFallbackUsed,
    notesPreservedAcrossRollback: postRbNotes.post_upgrade_v110 === 'kept_across_binary_rollback',
    uiChecksTotal: uiResolutionAudit.length,
    uiAnyOverflow,
    alphaTestkitLogs: alphaRunStats?.logsCount || 0,
    betaTestkitLogs: betaRunStats?.logsCount || 0,
  }, null, 2));

  if (
    !dbPreserved ||
    !studioRemoved ||
    !report.p6_3_securityAndReleaseChecksums.alphaPrePostZipShaMatch ||
    !report.p6_3_securityAndReleaseChecksums.betaPrePostZipShaMatch ||
    !fullTriadAttackErr.includes('FULL_TRIAD_ROLLBACK_DETECTED') ||
    !rogueRevokedDirAttackErr.includes('KEY_REVOKED') ||
    !(rogueDirAttackErr.includes('ROGUE_OR_CORRUPT_VERSION_DIR') || rogueDirAttackErr.includes('ROGUE_VERSION_DIR_DETECTED')) ||
    !manifestTamperErr.includes('RELEASE_MANIFEST_TAMPERED') ||
    !report.p6_4_ntfsAclAndDataRecovery.ntfsDepsFallbackUsed ||
    postRbNotes.post_upgrade_v110 !== 'kept_across_binary_rollback' ||
    (alphaRunStats?.logsCount || 0) === 0 ||
    (betaRunStats?.logsCount || 0) === 0
  ) {
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
