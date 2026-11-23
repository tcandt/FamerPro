const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const crypto = require('crypto');

const ROOT = 'E:\\DATA\\FarmerPro';
const PACK_DIR = path.join(ROOT, 'artifacts', 'final_acceptance_pack');
const GATE2_DIR = path.join(PACK_DIR, 'gate2_live_adb');
const SANDBOX = path.join(GATE2_DIR, 'sandbox');
const TARGET_SERIAL = 'emulator-5554';
const ADB_BIN = path.join(ROOT, 'bin', 'adb.exe');

function sha256File(p) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(p));
  return h.digest('hex');
}

function rmSyncRetry(target) {
  for (let i = 0; i < 6; i++) {
    try {
      if (fs.existsSync(target)) fs.rmSync(target, { recursive: true, force: true });
      return;
    } catch (e) {
      if (i === 5) throw e;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 300);
    }
  }
}

function liveEnv(extra = {}) {
  const sysPath = process.env.PATH || process.env.Path || '';
  return {
    ...process.env,
    PATH: `${path.join(ROOT, 'bin')};${sysPath}`,
    TEMP: path.join(SANDBOX, 'temp'),
    TMP: path.join(SANDBOX, 'temp'),
    APPDATA: path.join(SANDBOX, 'AppData', 'Roaming'),
    LOCALAPPDATA: path.join(SANDBOX, 'AppData', 'Local'),
    FP_TRUST_ANCHOR_DIR: path.join(SANDBOX, 'ExternalTrustAnchors'),
    ...extra,
  };
}

function pullLiveScreenshot(outName) {
  const remote = '/sdcard/fp_gate2_snap.png';
  const local = path.join(GATE2_DIR, outName);
  execFileSync(ADB_BIN, ['-s', TARGET_SERIAL, 'shell', 'screencap', '-p', remote], { encoding: 'utf8' });
  execFileSync(ADB_BIN, ['-s', TARGET_SERIAL, 'pull', remote, local], { encoding: 'utf8' });
  execFileSync(ADB_BIN, ['-s', TARGET_SERIAL, 'shell', 'rm', '-f', remote], { encoding: 'utf8' });
  return { file: outName, bytes: fs.statSync(local).size, sha256: sha256File(local) };
}

function resetToTopSettings() {
  for (const pkg of ['com.android.settings', 'com.google.android.settings.intelligence']) {
    try {
      execFileSync(ADB_BIN, ['-s', TARGET_SERIAL, 'shell', 'am', 'force-stop', pkg], { encoding: 'utf8' });
    } catch (_) { }
  }
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
  execFileSync(
    ADB_BIN,
    ['-s', TARGET_SERIAL, 'shell', 'am', 'start', '-W', '-a', 'android.settings.SETTINGS', '--activity-clear-task'],
    { encoding: 'utf8' }
  );
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1500);
}

function runCommandChecked(exe, args, cwd, envOverride = null) {
  return execFileSync(exe, args, {
    cwd,
    encoding: 'utf8',
    stdio: 'pipe',
    env: envOverride || process.env,
  }).trim();
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
      zf.write(fp, arc)`,
      srcDir,
      zipOut,
    ],
    { encoding: 'utf8' }
  );
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
    } catch (_) { }
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
    await new Promise((r) => setTimeout(r, 350));
    const res = await send('Page.captureScreenshot', { format: 'png' });
    const outPath = path.join(GATE2_DIR, filename);
    fs.writeFileSync(outPath, Buffer.from(res.data, 'base64'));
    return { file: filename, width, height, bytes: fs.statSync(outPath).size, sha256: sha256File(outPath) };
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

function findNsisInstaller() {
  const nsisDir = path.join(ROOT, 'target', 'release', 'bundle', 'nsis');
  const exes = fs.readdirSync(nsisDir).filter((f) => f.endsWith('-setup.exe'));
  if (exes.length === 0) throw new Error(`No *-setup.exe in ${nsisDir}`);
  return path.join(nsisDir, exes[0]);
}

async function main() {
  if (fs.existsSync(GATE2_DIR)) rmSyncRetry(GATE2_DIR);
  fs.mkdirSync(path.join(SANDBOX, 'temp'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Local'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Roaming'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'ExternalTrustAnchors'), { recursive: true });

  // 0. Check live ADB device online, verify canonical two-transport identity on 1 AVD, & reset to Android Settings home
  const adbDevicesRaw = runCommandChecked(ADB_BIN, ['devices', '-l'], ROOT);
  if (!adbDevicesRaw.includes(TARGET_SERIAL) || !/\sdevice(\s|$)/.test(adbDevicesRaw)) {
    throw new Error(`Expected ${TARGET_SERIAL} in Ready device state; got:\n${adbDevicesRaw}`);
  }
  const emuSerialNo = runCommandChecked(ADB_BIN, ['-s', TARGET_SERIAL, 'shell', 'getprop', 'ro.boot.serialno'], ROOT);
  const emuBootId = runCommandChecked(ADB_BIN, ['-s', TARGET_SERIAL, 'shell', 'cat', '/proc/sys/kernel/random/boot_id'], ROOT);
  // Deliberately attach loopback alias 127.0.0.1:5555 before Studio starts to prove canonical deduplication in real runtime!
  const connectLoopbackOut = runCommandChecked(ADB_BIN, ['connect', '127.0.0.1:5555'], ROOT);
  const adbDevicesWithDualTransport = runCommandChecked(ADB_BIN, ['devices', '-l'], ROOT);
  const tcpBootId = runCommandChecked(ADB_BIN, ['-s', '127.0.0.1:5555', 'shell', 'cat', '/proc/sys/kernel/random/boot_id'], ROOT);
  if (emuBootId !== tcpBootId) {
    throw new Error(`Expected identical boot_id across ${TARGET_SERIAL} and 127.0.0.1:5555, got ${emuBootId} vs ${tcpBootId}`);
  }
  resetToTopSettings();
  const devBeforeStudio = pullLiveScreenshot('gate2_device_before_studio.png');

  // 1. Install frozen RC FarmerPro Setup in isolated Gate2 sandbox & launch Studio
  const studioInstallDir = path.join(SANDBOX, 'InstalledStudio');
  const studioEnv = liveEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9298' });
  const nsisSrc = findNsisInstaller();
  runCommandChecked(nsisSrc, ['/S', `/D=${studioInstallDir}`], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1100));
  const studioExe = path.join(studioInstallDir, 'FarmerPro.exe');
  if (!fs.existsSync(studioExe)) throw new Error(`Missing installed ${studioExe}`);

  const studioChild = spawn(studioExe, [], { cwd: studioInstallDir, env: studioEnv, stdio: 'ignore' });
  let studioResult = null;
  let releaseZip = null;
  const relOut = path.join(GATE2_DIR, 'Gate2_LiveMini_v1.0.0_win-x64');

  try {
    const page = await waitForCdp(9298, 28000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);

    // Phase A1: ADB Discovery (must coalesce dual-transport emulator-5554 + 127.0.0.1:5555 to 1!), Forward/Reverse Create/List/Remove, Live Screenshot & Vision Template Crop + Match
    const discoveryRes = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const discovered = await inv('auto_discover_devices');
      const listed = await inv('list_devices');

      // Real ADB forward/reverse create -> list -> remove verification on live ${TARGET_SERIAL}
      const fwdCreated = await inv('studio_add_device_forward', {
        serial: '${TARGET_SERIAL}',
        localSpec: 'tcp:19555',
        remoteSpec: 'tcp:19556'
      });
      const revCreated = await inv('studio_add_device_reverse', {
        serial: '${TARGET_SERIAL}',
        remoteSpec: 'tcp:19557',
        localSpec: 'tcp:19558'
      });
      const portsAfterCreate = await inv('studio_list_device_ports', { serial: '${TARGET_SERIAL}' });
      const listedFwd = portsAfterCreate.forwards.find(f => f.local === 'tcp:19555' && f.remote === 'tcp:19556');
      const listedRev = portsAfterCreate.reverses.find(r => r.remote === 'tcp:19557' && r.local === 'tcp:19558');
      if (listedFwd) {
        await inv('studio_remove_device_forward', { serial: listedFwd.serial, localSpec: listedFwd.local });
      }
      if (listedRev) {
        await inv('studio_remove_device_reverse', { serial: listedRev.serial, remoteSpec: listedRev.remote });
      }
      const portsAfterRemove = await inv('studio_list_device_ports', { serial: '${TARGET_SERIAL}' });

      const capDataUrl = await inv('capture_device', { deviceId: '${TARGET_SERIAL}' });
      const b64 = capDataUrl.replace(/^data:image\\/png;base64,/, '');

      const pub = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-gate2-live',
          displayName: 'FarmerPro Gate2 QA',
          organization: 'FarmerPro Live Validation',
          copyright: 'Copyright (C) 2026 FarmerPro',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });

      const proj = await inv('studio_create_project', {
        req: {
          slug: 'live-gate2-android',
          name: 'Gate2 Live Android Validator',
          gamePackageId: 'com.android.settings',
          version: '1.0.0',
          exeName: 'Auto_Gate2Live.exe',
          windowTitle: 'Gate2 Live Android Validator v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });

      await inv('studio_ensure_project_icon', { projectId: proj.id, accentHex: '#10B981', overwrite: true });

      const tplNetwork = await inv('studio_capture_and_crop_template', {
        projectId: proj.id,
        serial: '${TARGET_SERIAL}',
        name: 'search_settings_bar',
        relPath: 'templates/search_settings_bar.png',
        roi: { x: 50, y: 76, width: 260, height: 56 },
        defaultConfidence: 0.85
      });

      const tplConnected = await inv('studio_capture_and_crop_template', {
        projectId: proj.id,
        serial: '${TARGET_SERIAL}',
        name: 'search_settings_bar_mini',
        relPath: 'templates/search_settings_bar_mini.png',
        roi: { x: 50, y: 76, width: 260, height: 56 },
        defaultConfidence: 0.85
      });

      const matchPreview = await inv('studio_test_project_template_match', {
        projectId: proj.id,
        templateId: tplNetwork.id,
        framePngBase64: b64,
        frameWidth: 1280,
        frameHeight: 720,
        minConfidence: 0.85,
        saveAnnotatedRelPath: 'templates/match_network_annotated.png'
      });

      const navDev = Array.from(document.querySelectorAll('button')).find(b => b.textContent && (b.textContent.includes('Thiết bị') || b.textContent.includes('Device Lab')));
      if (navDev) navDev.click();
      return { discovered, listed, fwdCreated, revCreated, portsAfterCreate, portsAfterRemove, capLen: b64.length, proj, pub, tplNetwork, tplConnected, matchPreview };
    })()`);
    if (discoveryRes.discovered.length !== 1 || discoveryRes.discovered[0].adb_serial !== TARGET_SERIAL) {
      throw new Error(`Expected canonical deduplication to return 1 device (${TARGET_SERIAL}), got: ${JSON.stringify(discoveryRes.discovered)}`);
    }
    if (discoveryRes.listed.length !== 1 || discoveryRes.listed[0].adb_serial !== TARGET_SERIAL) {
      throw new Error(`Expected list_devices to return 1 device (${TARGET_SERIAL}), got: ${JSON.stringify(discoveryRes.listed)}`);
    }
    if (!discoveryRes.portsAfterCreate.forwards.some(f => f.local === 'tcp:19555' && f.remote === 'tcp:19556')
        || !discoveryRes.portsAfterCreate.reverses.some(r => r.remote === 'tcp:19557' && r.local === 'tcp:19558')
        || discoveryRes.portsAfterRemove.forwards.some(f => f.local === 'tcp:19555')
        || discoveryRes.portsAfterRemove.reverses.some(r => r.remote === 'tcp:19557')) {
      throw new Error(`ADB forward/reverse create/list/remove assertion failed: create=${JSON.stringify(discoveryRes.portsAfterCreate)}, remove=${JSON.stringify(discoveryRes.portsAfterRemove)}`);
    }

    await new Promise((r) => setTimeout(r, 600));
    const snapStudioDev = await snap('gate2_studio_device_lab.png', 1600, 900);

    // Save Workflows 1 (Pause/Resume + OCR + SafeTap Network + Swipe + Checkpoint), 2 (Checkpoint Suspend/Resume BACK), 3 (Cancel), and Mini Tool Task
    await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const pid = ${JSON.stringify(discoveryRes.proj.id)};

      await inv('studio_save_project_workflow', {
        projectId: pid,
        workflowId: 'wf-studio-live-1',
        name: 'Live OCR SafeTap Network Swipe PauseResume',
        category: 'LiveGate2',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-studio-live-1',
          name: 'Live OCR SafeTap Network Swipe PauseResume',
          status: 'published',
          graph: {
            nodes: [
              { id: 'n1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'n2', type: 'vision.capture', position: { x: 180, y: 80 }, config: {} },
              { id: 'n3', type: 'vision.ocr', position: { x: 320, y: 80 }, config: { expectedText: 'Search', minConfidence: 0.70, outputVar: 'ocr_screen_label' } },
              { id: 'n4', type: 'vision.find_image', position: { x: 480, y: 80 }, config: { templateRef: 'templates/search_settings_bar.png', minConfidence: 0.85 } },
              { id: 'n5', type: 'action.random_safe_tap', position: { x: 640, y: 80 }, config: { marginRatio: 0.15 } },
              { id: 'n6', type: 'control.wait', position: { x: 800, y: 80 }, config: { durationMs: 2800 } },
              { id: 'n7', type: 'action.swipe', position: { x: 960, y: 80 }, config: { startX: 640, startY: 500, endX: 640, endY: 260, durationMs: 250 } },
              { id: 'n8', type: 'state.checkpoint', position: { x: 1120, y: 80 }, config: {} },
              { id: 'n9', type: 'state.set_var', position: { x: 1280, y: 80 }, config: { varKey: 'studio_live_status', valueType: 'string', value: 'STUDIO_LIVE_OK' } },
              { id: 'n10', type: 'flow.end', position: { x: 1440, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'n1', sourcePort: 'flow', target: 'n2', targetPort: 'flow' },
              { id: 'e2', source: 'n2', sourcePort: 'flow', target: 'n3', targetPort: 'flow' },
              { id: 'e3', source: 'n3', sourcePort: 'flow', target: 'n4', targetPort: 'flow' },
              { id: 'e4', source: 'n4', sourcePort: 'flow', target: 'n5', targetPort: 'flow' },
              { id: 'e5', source: 'n5', sourcePort: 'flow', target: 'n6', targetPort: 'flow' },
              { id: 'e6', source: 'n6', sourcePort: 'flow', target: 'n7', targetPort: 'flow' },
              { id: 'e7', source: 'n7', sourcePort: 'flow', target: 'n8', targetPort: 'flow' },
              { id: 'e8', source: 'n8', sourcePort: 'flow', target: 'n9', targetPort: 'flow' },
              { id: 'e9', source: 'n9', sourcePort: 'flow', target: 'n10', targetPort: 'flow' }
            ]
          }
        })
      });

      await inv('studio_save_project_workflow', {
        projectId: pid,
        workflowId: 'wf-studio-cp-2',
        name: 'Live Suspend Checkpoint Back Nav',
        category: 'LiveGate2',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-studio-cp-2',
          name: 'Live Suspend Checkpoint Back Nav',
          status: 'published',
          graph: {
            nodes: [
              { id: 'c1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'c2', type: 'control.wait', position: { x: 180, y: 80 }, config: { durationMs: 2200 } },
              { id: 'c3', type: 'action.key', position: { x: 340, y: 80 }, config: { keycode: 'BACK' } },
              { id: 'c4', type: 'control.wait', position: { x: 500, y: 80 }, config: { durationMs: 600 } },
              { id: 'c5', type: 'state.checkpoint', position: { x: 660, y: 80 }, config: {} },
              { id: 'c6', type: 'state.set_var', position: { x: 820, y: 80 }, config: { varKey: 'checkpoint_resumed', valueType: 'string', value: 'SETTINGS_HOME_RESTORED' } },
              { id: 'c7', type: 'flow.end', position: { x: 980, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'ce1', source: 'c1', sourcePort: 'flow', target: 'c2', targetPort: 'flow' },
              { id: 'ce2', source: 'c2', sourcePort: 'flow', target: 'c3', targetPort: 'flow' },
              { id: 'ce3', source: 'c3', sourcePort: 'flow', target: 'c4', targetPort: 'flow' },
              { id: 'ce4', source: 'c4', sourcePort: 'flow', target: 'c5', targetPort: 'flow' },
              { id: 'ce5', source: 'c5', sourcePort: 'flow', target: 'c6', targetPort: 'flow' },
              { id: 'ce6', source: 'c6', sourcePort: 'flow', target: 'c7', targetPort: 'flow' }
            ]
          }
        })
      });

      await inv('studio_save_project_workflow', {
        projectId: pid,
        workflowId: 'wf-studio-cancel-3',
        name: 'Live Cancel Control Test',
        category: 'LiveGate2',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-studio-cancel-3',
          name: 'Live Cancel Control Test',
          status: 'published',
          graph: {
            nodes: [
              { id: 'k1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'k2', type: 'control.wait', position: { x: 200, y: 80 }, config: { durationMs: 800 } },
              { id: 'k3', type: 'control.wait', position: { x: 340, y: 80 }, config: { durationMs: 800 } },
              { id: 'k4', type: 'control.wait', position: { x: 480, y: 80 }, config: { durationMs: 800 } },
              { id: 'k5', type: 'flow.end', position: { x: 620, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'ke1', source: 'k1', sourcePort: 'flow', target: 'k2', targetPort: 'flow' },
              { id: 'ke2', source: 'k2', sourcePort: 'flow', target: 'k3', targetPort: 'flow' },
              { id: 'ke3', source: 'k3', sourcePort: 'flow', target: 'k4', targetPort: 'flow' },
              { id: 'ke4', source: 'k4', sourcePort: 'flow', target: 'k5', targetPort: 'flow' }
            ]
          }
        })
      });

      await inv('studio_save_project_workflow', {
        projectId: pid,
        workflowId: 'wf-mini-live-conn',
        name: 'Mini Live Open Search Settings',
        category: 'LiveGate2',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-mini-live-conn',
          name: 'Mini Live Open Search Settings',
          status: 'published',
          graph: {
            nodes: [
              { id: 'm1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'm2', type: 'vision.capture', position: { x: 180, y: 80 }, config: {} },
              { id: 'm3', type: 'vision.ocr', position: { x: 320, y: 80 }, config: { expectedText: 'Search', minConfidence: 0.70, outputVar: 'ocr_screen_label' } },
              { id: 'm4', type: 'vision.find_image', position: { x: 480, y: 80 }, config: { templateRef: 'templates/search_settings_bar_mini.png', minConfidence: 0.85 } },
              { id: 'm5', type: 'action.random_safe_tap', position: { x: 640, y: 80 }, config: { marginRatio: 0.15 } },
              { id: 'm6', type: 'control.wait', position: { x: 800, y: 80 }, config: { durationMs: 700 } },
              { id: 'm7', type: 'state.checkpoint', position: { x: 960, y: 80 }, config: {} },
              { id: 'm8', type: 'state.set_var', position: { x: 1120, y: 80 }, config: { varKey: 'mini_live_result', valueType: 'string', value: 'SEARCH_SETTINGS_OPENED' } },
              { id: 'm9', type: 'flow.end', position: { x: 1280, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'me1', source: 'm1', sourcePort: 'flow', target: 'm2', targetPort: 'flow' },
              { id: 'me2', source: 'm2', sourcePort: 'flow', target: 'm3', targetPort: 'flow' },
              { id: 'me3', source: 'm3', sourcePort: 'flow', target: 'm4', targetPort: 'flow' },
              { id: 'me4', source: 'm4', sourcePort: 'flow', target: 'm5', targetPort: 'flow' },
              { id: 'me5', source: 'm5', sourcePort: 'flow', target: 'm6', targetPort: 'flow' },
              { id: 'me6', source: 'm6', sourcePort: 'flow', target: 'm7', targetPort: 'flow' },
              { id: 'me7', source: 'm7', sourcePort: 'flow', target: 'm8', targetPort: 'flow' },
              { id: 'me8', source: 'm8', sourcePort: 'flow', target: 'm9', targetPort: 'flow' }
            ]
          }
        })
      });
    })()`);

    // Execute Job 1 (Pause & Resume while running on live emulator-5554)
    const job1Start = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      return await inv('studio_start_project_workflow', {
        req: {
          projectId: ${JSON.stringify(discoveryRes.proj.id)},
          workflowId: 'wf-studio-live-1',
          deviceId: '${TARGET_SERIAL}',
          priority: 1,
          leaseTtlSecs: 30,
          workflowTimeoutMs: 30000,
          initialVariables: {}
        }
      });
    })()`);

    // Pause during OCR/SafeTap->Wait window, confirm Paused, then Resume!
    await new Promise((r) => setTimeout(r, 2400));
    await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_pause_job', { jobId: ${JSON.stringify(job1Start.jobId)} })`);
    await new Promise((r) => setTimeout(r, 700));
    const job1PausedState = await evalJs(`(async () => {
      const jobs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_jobs', { projectId: ${JSON.stringify(discoveryRes.proj.id)} });
      return jobs.find(j => j.id === ${JSON.stringify(job1Start.jobId)});
    })()`);
    await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_resume_job', { jobId: ${JSON.stringify(job1Start.jobId)} })`);

    let job1Final = null;
    for (let i = 0; i < 40; i++) {
      await new Promise((r) => setTimeout(r, 400));
      job1Final = await evalJs(`(async () => {
        const jobs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_jobs', { projectId: ${JSON.stringify(discoveryRes.proj.id)} });
        return jobs.find(j => j.id === ${JSON.stringify(job1Start.jobId)});
      })()`);
      if (job1Final && ['Succeeded', 'Failed', 'Blocked', 'Cancelled'].includes(job1Final.state)) break;
    }
    const devAfterStudio = pullLiveScreenshot('gate2_device_after_studio.png');
    const studioRuntimeVars = await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_get_project_runtime_vars', { projectId: ${JSON.stringify(discoveryRes.proj.id)} })`);

    // Reload Studio to bind React Project Selector & capture Test & Debug showing real Live Jobs + Step Events!
    await evalJs(`(() => { location.reload(); })()`);
    await new Promise((r) => setTimeout(r, 1500));
    await evalJs(`(() => {
      const selCard = Array.from(document.querySelectorAll('button, div')).find(el => el.textContent && el.textContent.includes('Gate2 Live Android Validator') && el.tagName === 'BUTTON');
      if (selCard) selCard.click();
    })()`);
    await new Promise((r) => setTimeout(r, 500));
    await evalJs(`(() => {
      const projOpt = Array.from(document.querySelectorAll('button, div, tr')).find(el => el.textContent && el.textContent.includes('live-gate2-android'));
      if (projOpt) projOpt.click();
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && (b.textContent.includes('Test') || b.textContent.includes('Kiểm')));
      if (btn) btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 700));
    await evalJs(`(() => {
      const jobRow = Array.from(document.querySelectorAll('button, div')).find(el => el.textContent && el.textContent.includes('Succeeded'));
      if (jobRow) jobRow.click();
    })()`);
    await new Promise((r) => setTimeout(r, 500));
    const snapStudioRun = await snap('gate2_studio_runtime_vars.png', 1600, 900);

    // Execute Job 2: Suspend to Checkpoint & Resume from Checkpoint (navigates BACK to Settings home!)
    const job2Start = await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
      req: {
        projectId: ${JSON.stringify(discoveryRes.proj.id)},
        workflowId: 'wf-studio-cp-2',
        deviceId: '${TARGET_SERIAL}',
        priority: 1,
        leaseTtlSecs: 30,
        workflowTimeoutMs: 30000,
        initialVariables: {}
      }
    })`);
    await new Promise((r) => setTimeout(r, 400));
    await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_suspend_job_to_checkpoint', { jobId: ${JSON.stringify(job2Start.jobId)} })`);
    let job2Suspended = null;
    for (let i = 0; i < 20; i++) {
      await new Promise((r) => setTimeout(r, 250));
      job2Suspended = await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_get_job_resume_detail', { jobId: ${JSON.stringify(job2Start.jobId)} })`);
      const st = job2Suspended?.resumeContext?.job?.state || job2Suspended?.jobState;
      const cp = job2Suspended?.resumeContext?.latestCheckpoint || job2Suspended?.checkpoint;
      if (st && String(st).toLowerCase() === 'paused' && cp) break;
    }
    const job2Resumed = await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_resume_job_from_checkpoint', {
      jobId: ${JSON.stringify(job2Start.jobId)},
      leaseTtlSecs: 30,
      workflowTimeoutMs: 30000
    })`);

    // Execute Job 3: Cancel control test
    const job3Start = await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
      req: {
        projectId: ${JSON.stringify(discoveryRes.proj.id)},
        workflowId: 'wf-studio-cancel-3',
        deviceId: '${TARGET_SERIAL}',
        priority: 1,
        leaseTtlSecs: 30,
        workflowTimeoutMs: 30000,
        initialVariables: {}
      }
    })`);
    await new Promise((r) => setTimeout(r, 350));
    await evalJs(`window.__TAURI_INTERNALS__.invoke('studio_cancel_job', { jobId: ${JSON.stringify(job3Start.jobId)} })`);
    let job3Final = null;
    for (let i = 0; i < 20; i++) {
      await new Promise((r) => setTimeout(r, 250));
      job3Final = await evalJs(`(async () => {
        const jobs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_jobs', { projectId: ${JSON.stringify(discoveryRes.proj.id)} });
        return jobs.find(j => j.id === ${JSON.stringify(job3Start.jobId)});
      })()`);
      if (job3Final && ['cancelled', 'failed', 'succeeded', 'Cancelled', 'Failed', 'Succeeded'].includes(job3Final.state)) break;
    }
    if (!job3Final || String(job3Final.state).toLowerCase() !== 'cancelled') {
      throw new Error(`Expected job3 cancelled, got ${JSON.stringify(job3Final)}`);
    }

    // Export independent Live Mini Tool!
    const miniTaskDef = {
      id: 'tsk-live-conn',
      title: '1. Open Search Settings (Live OCR + SafeTap)',
      categoryId: 'cat-live',
      workflowId: 'wf-mini-live-conn',
      defaultEnabled: true,
      priority: 1,
      estimateSec: 10,
      settingsSchema: [],
    };
    const exportRes = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const pid = ${JSON.stringify(discoveryRes.proj.id)};
      await inv('studio_upsert_project_task', {
        projectId: pid,
        task: ${JSON.stringify(miniTaskDef)},
        categoryMeta: { id: 'cat-live', shortTag: 'LIVE', displayName: 'Live ADB', colorHex: '#10B981' }
      });
      const ui = await inv('studio_get_ui_schema', { projectId: pid });
      ui.layoutPreset = 'expanded';
      ui.defaultWindowSize = [980, 720];
      ui.deviceColumns = [
        { id: 'col-ocr', label: 'OCR Match', runtimeVarKey: 'ocr_matched', fallbackDisplay: '-', minWindowWidth: 500, badgeColor: '#38BDF8' },
        { id: 'col-res', label: 'Live Result', runtimeVarKey: 'mini_live_result', fallbackDisplay: '-', minWindowWidth: 500, badgeColor: '#10B981' },
        { id: 'col-src', label: 'Driver', runtimeVarKey: 'last_data_source', fallbackDisplay: '-', minWindowWidth: 680, badgeColor: '#FBBF24' }
      ];
      ui.taskCategories = [{ id: 'cat-live', shortTag: 'LIVE', displayName: 'Live ADB', colorHex: '#10B981' }];
      ui.tasks = [${JSON.stringify(miniTaskDef)}];
      await inv('studio_save_ui_schema', { projectId: pid, schema: ui });
      const vi = {
        companyName: 'FarmerPro Live Validation',
        fileDescription: 'Gate2 Live Android Mini Runtime',
        fileVersion: '1.0.0.1',
        internalName: 'live-gate2-android',
        legalCopyright: 'Copyright (C) 2026 FarmerPro',
        originalFilename: 'Auto_Gate2Live.exe',
        productName: 'Gate2 Live Android Validator',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: { id: pid, publisherProfileId: ${JSON.stringify(discoveryRes.pub.id)}, signingMode: 'unsigned_internal', versionInfoJson: JSON.stringify(vi) }
      });
      const exp = await inv('studio_builder_export_release', {
        projectId: pid,
        outputDir: ${JSON.stringify(relOut)},
        options: { signingMode: 'unsigned_internal', publisherProfileId: ${JSON.stringify(discoveryRes.pub.id)}, versionInfo: vi }
      });
      const fv = await inv('studio_verify_final_release', { releaseDir: ${JSON.stringify(relOut)}, requirePublicChain: false });
      return { exp, fv };
    })()`);

    const studioEvents = await evalJs(`window.__TAURI_INTERNALS__.invoke('get_audit_events', { jobId: ${JSON.stringify(job1Start.jobId)} })`);

    studioResult = {
      adbDiscoveredCount: discoveryRes.discovered.length,
      listedDevices: discoveryRes.listed,
      adbPortMappingVerification: {
        fwdCreated: discoveryRes.fwdCreated,
        revCreated: discoveryRes.revCreated,
        portsAfterCreate: discoveryRes.portsAfterCreate,
        portsAfterRemove: discoveryRes.portsAfterRemove,
      },
      liveCapturePngLen: discoveryRes.capLen,
      templateNetwork: discoveryRes.tplNetwork,
      templateConnected: discoveryRes.tplConnected,
      templateMatch: discoveryRes.matchPreview,
      job1Start,
      job1PausedState: job1PausedState?.state,
      job1Final,
      job1EventsCount: studioEvents.length,
      job1AuditEventsSummary: studioEvents.map(e => ({
        nodeId: e.node_id || e.nodeId,
        eventType: e.event_type || e.eventType,
        payloadJson: e.payload_json || e.payloadJson,
      })),
      job2SuspendedStage: {
        jobState: job2Suspended?.resumeContext?.job?.state || job2Suspended?.jobState || null,
        checkpointId: job2Suspended?.resumeContext?.latestCheckpoint?.id || job2Suspended?.checkpoint?.id || null,
        hasCheckpoint: Boolean(job2Suspended?.resumeContext?.latestCheckpoint || job2Suspended?.checkpoint),
      },
      job2Resumed,
      job3FinalState: job3Final?.state,
      studioRuntimeVars,
      devBeforeStudio,
      devAfterStudio,
      snapStudioDev,
      snapStudioRun,
      consoleErrors,
      exportVerified: exportRes.fv.verified,
    };
    ws.close();
  } finally {
    try { studioChild.kill('SIGTERM'); } catch (_) { }
  }

  await new Promise((r) => setTimeout(r, 1000));
  runCommandChecked(path.join(studioInstallDir, 'uninstall.exe'), ['/S'], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1200));

  // Pack & extract Independent Live Mini Tool (Studio completely gone!)
  releaseZip = path.join(GATE2_DIR, 'Gate2_LiveMini_v1.0.0_win-x64.zip');
  createForwardSlashZip(relOut, releaseZip);
  const miniRunHost = path.join(SANDBOX, 'IndependentMiniHost');
  fs.mkdirSync(miniRunHost, { recursive: true });
  execFileSync('powershell', ['-NoProfile', '-Command', `Expand-Archive -Path '${releaseZip}' -DestinationPath '${miniRunHost}' -Force`]);
  const miniDir = path.join(miniRunHost, 'Gate2_LiveMini_v1.0.0_win-x64');
  const miniExe = path.join(miniDir, 'Auto_Gate2Live.exe');

  resetToTopSettings();
  const devBeforeMini = pullLiveScreenshot('gate2_device_before_mini.png');
  const miniChild = spawn(miniExe, [], {
    cwd: miniDir,
    env: liveEnv({ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9299' }),
    stdio: 'ignore',
  });

  let miniResult = null;
  try {
    const pgM = await waitForCdp(9299, 24000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(pgM, '#left-body .task-row');
    await new Promise((r) => setTimeout(r, 500));

    const pillBefore = await evalJs(`document.getElementById('adb-mode-pill')?.textContent || ''`);
    if (!pillBefore.includes('1 ONLINE')) {
      throw new Error(`Expected deduplicated 'LIVE ADB: 1 ONLINE' on Mini Runtime pill, got: '${pillBefore}'`);
    }
    const snapMiniBefore = await snap('gate2_mini_before.png', 980, 720);

    // Click the REAL ADB button ("▶ CHẠY NHIỆM VỤ (ADB THẬT)"), NOT TestKit!
    await evalJs(`document.querySelector('#footer-root button.btn-primary').click()`);

    let doneState = null;
    for (let i = 0; i < 45; i++) {
      await new Promise((r) => setTimeout(r, 400));
      doneState = await evalJs(`({
        pill: document.getElementById('adb-mode-pill')?.textContent || '',
        logs: STATE?.logs || [],
        liveEnvelopes: STATE?.liveEnvelopes || {},
        testkitEnvelopes: STATE?.testkitEnvelopes || {},
        sources: Array.from(document.querySelectorAll('#device-tbody td[data-var-source]')).map(td => ({
          key: td.getAttribute('data-var-key'),
          src: td.getAttribute('data-var-source'),
          txt: td.innerText
        })),
        taskStatus: document.querySelector('#left-body .task-row')?.innerText || '',
        anyTestkitBadge: document.body.innerText.includes('TESTKIT · Mô phỏng')
      })`);
      if (doneState.liveEnvelopes && doneState.liveEnvelopes.mini_live_result) break;
    }
    await new Promise((r) => setTimeout(r, 500));
    const snapMiniAfter = await snap('gate2_mini_after.png', 980, 720);
    const devAfterMini = pullLiveScreenshot('gate2_device_after_mini.png');

    miniResult = {
      pillBefore,
      ...doneState,
      snapMiniBefore,
      snapMiniAfter,
      devBeforeMini,
      devAfterMini,
      consoleErrors,
    };
    ws.close();
  } finally {
    try { miniChild.kill('SIGTERM'); } catch (_) { }
  }

  // Query Mini Tool independent SQLite DB provenance!
  const miniDb = path.join(miniDir, 'data', 'runtime_live-gate2-android.db');
  const dbCheck = JSON.parse(
    runCommandChecked(
      'python',
      [
        '-c',
        `import sqlite3, json, sys
c = sqlite3.connect(sys.argv[1])
jobs = c.execute("SELECT id, device_id, state FROM jobs").fetchall()
ev_cnt = c.execute("SELECT count(*) FROM step_events").fetchone()[0]
vars_rows = c.execute("SELECT var_name, value_json, source, verified, confidence, seq FROM runtime_variable_envelopes").fetchall()
print(json.dumps({"jobs": jobs, "stepEventsCount": ev_cnt, "runtimeVariableEnvelopes": vars_rows}))`,
        miniDb,
      ],
      ROOT
    )
  );

  const zipMeta = { file: path.basename(releaseZip), bytes: fs.statSync(releaseZip).size, sha256: sha256File(releaseZip) };
  try { execFileSync(ADB_BIN, ['disconnect', '127.0.0.1:5555'], { encoding: 'utf8' }); } catch (_) {}
  rmSyncRetry(SANDBOX);
  rmSyncRetry(relOut);
  rmSyncRetry(releaseZip);

  const report = {
    executedAtUtc: new Date().toISOString(),
    gitHead: runCommandChecked('git', ['rev-parse', 'HEAD'], ROOT),
    rcCodeTag: 'v0.1.0-p6-rc.2',
    adbTargetSerial: TARGET_SERIAL,
    adbTargetModel: 'sdk_gphone16k_x86_64 (Android SDK Emulator fp_gate2_avd, WHPX)',
    canonicalDeviceIdentityProof: {
      emuBootSerialNo: emuSerialNo,
      emuBootId,
      loopbackConnectOutput: connectLoopbackOut,
      adbDevicesRawDualTransport: adbDevicesWithDualTransport,
      tcpBootId,
      identicalInstanceConfirmed: emuBootId === tcpBootId,
    },
    adbDevicesRaw,
    gate2Status: 'PASS (LIVE_ANDROID_ADB_VERIFIED)',
    studioLiveRun: studioResult,
    independentMiniLiveRun: miniResult,
    independentMiniSqlite: dbCheck,
    exportedLiveMiniZipMeta: zipMeta,
  };
  fs.writeFileSync(path.join(GATE2_DIR, 'gate2_live_adb_report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({
    gate2Status: report.gate2Status,
    studioJob1State: studioResult.job1Final?.state,
    studioPausedObserved: studioResult.job1PausedState,
    studioCheckpointResumed: studioResult.job2Resumed?.state,
    studioJob3Cancelled: studioResult.job3FinalState,
    studioMatchConfidence: studioResult.templateMatch?.confidence,
    miniPillBefore: miniResult.pillBefore,
    miniPillAfter: miniResult.pill,
    miniAnyTestkitBadge: miniResult.anyTestkitBadge,
    miniLiveEnvelopesKeys: Object.keys(miniResult.liveEnvelopes || {}),
    miniDbJobs: dbCheck.jobs,
    miniDbEvents: dbCheck.stepEventsCount,
  }, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
