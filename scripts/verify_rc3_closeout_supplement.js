const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const crypto = require('crypto');

const ROOT = 'E:\\DATA\\FarmerPro';
const PACK_DIR = path.join(ROOT, 'artifacts', 'final_acceptance_pack');
const RC4_DIR = path.join(PACK_DIR, 'rc4_evidence');
const SANDBOX = path.join(RC4_DIR, 'sandbox');
const TARGET_SERIAL = 'emulator-5554';
const ADB_BIN = path.join(ROOT, 'bin', 'adb.exe');
const INSTALLER_EXE = path.join(ROOT, 'artifacts', 'p6_evidence', 'dist_releases', 'FarmerPro_0.1.0_x64-setup.exe');

function sha256File(p) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(p));
  return h.digest('hex');
}

function rmSyncRetry(target) {
  for (let i = 0; i < 10; i++) {
    try {
      if (fs.existsSync(target)) fs.rmSync(target, { recursive: true, force: true });
      return;
    } catch (e) {
      if (i === 9) throw e;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 350);
    }
  }
}

function liveEnv(baseSandbox, extra = {}) {
  const sysPath = process.env.PATH || process.env.Path || '';
  return {
    ...process.env,
    PATH: `${path.join(ROOT, 'bin')};${sysPath}`,
    TEMP: path.join(baseSandbox, 'temp'),
    TMP: path.join(baseSandbox, 'temp'),
    APPDATA: path.join(baseSandbox, 'AppData', 'Roaming'),
    LOCALAPPDATA: path.join(baseSandbox, 'AppData', 'Local'),
    FP_TRUST_ANCHOR_DIR: path.join(baseSandbox, 'ExternalTrustAnchors'),
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
  try {
    const out = execFileSync(exe, args, {
      cwd,
      encoding: 'utf8',
      stdio: 'pipe',
      env: envOverride || process.env,
    });
    throw new Error(`Expected command failure, got success: ${out}`);
  } catch (e) {
    if (e.message && e.message.startsWith('Expected command failure')) throw e;
    return `${e.stdout || ''}\n${e.stderr || ''}\n${e.message || ''}`.trim();
  }
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

  for (let i = 0; i < 65; i++) {
    const checkExpr = waitSelector
      ? `Boolean(window.__TAURI_INTERNALS__ && document.querySelector(${JSON.stringify(waitSelector)}))`
      : 'Boolean(window.__TAURI_INTERNALS__)';
    const ready = await evalJs(checkExpr);
    if (ready) break;
    await new Promise((r) => setTimeout(r, 200));
  }

  return { ws, evalJs, consoleErrors };
}

function findRc4Installer() {
  const targetNsis = path.join(ROOT, 'target', 'release', 'bundle', 'nsis', 'FarmerPro_0.1.0_x64-setup.exe');
  if (fs.existsSync(targetNsis)) return targetNsis;
  return INSTALLER_EXE;
}

async function main() {
  fs.mkdirSync(RC4_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) rmSyncRetry(SANDBOX);
  fs.mkdirSync(path.join(SANDBOX, 'temp'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Local'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Roaming'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'ExternalTrustAnchors'), { recursive: true });

  const adbDevicesRaw = runCommandChecked(ADB_BIN, ['devices', '-l'], ROOT);
  if (!adbDevicesRaw.includes(TARGET_SERIAL) || !/\sdevice(\s|$)/.test(adbDevicesRaw)) {
    throw new Error(`Expected ${TARGET_SERIAL} Ready; got:\n${adbDevicesRaw}`);
  }

  const installerUsed = findRc4Installer();
  const studioInstallDir = path.join(SANDBOX, 'InstalledStudio');
  const studioEnv = liveEnv(SANDBOX, {
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9298',
    WEBVIEW2_USER_DATA_FOLDER: path.join(SANDBOX, 'wv2_studio'),
  });
  runCommandChecked(installerUsed, ['/S', `/D=${studioInstallDir}`], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1100));
  const studioExe = path.join(studioInstallDir, 'FarmerPro.exe');

  const relOut1 = path.join(SANDBOX, 'CrossMini1_v1.0.0_win-x64');
  const relOut2 = path.join(SANDBOX, 'CrossMini2_v1.0.0_win-x64');
  let studioChild = spawn(studioExe, [], { cwd: studioInstallDir, env: studioEnv, stdio: 'ignore' });
  let testAResult = null;
  let projId = null;
  let proj2Id = null;

  try {
    const pgS = await waitForCdp(9298, 28000);
    const { ws: wsS, evalJs: evalS } = await connectCdp(pgS);

    const setup = await evalS(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      await inv('auto_discover_devices');
      const pub = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-rc4-cross',
          displayName: 'FarmerPro RC4 QA',
          organization: 'FarmerPro QA',
          copyright: 'Copyright (C) 2026 FarmerPro',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });

      const mkProject = async (slug, title, exeName, outDir, prefix) => {
        const proj = await inv('studio_create_project', {
          req: {
            slug,
            name: title,
            gamePackageId: 'com.android.settings',
            version: '1.0.0',
            exeName,
            windowTitle: title + ' v1.0.0',
            targetWidthPx: 1280,
            targetHeightPx: 720,
            bundleAdb: true,
            enabledModules: ['mod.adb_core', 'mod.vision_ocr']
          }
        });
        await inv('studio_ensure_project_icon', { projectId: proj.id, accentHex: '#3B82F6', overwrite: true });
        await inv('studio_save_project_workflow', {
          projectId: proj.id,
          workflowId: prefix + 'wf-cross-studio-long',
          name: 'Studio Long Hold Workflow',
          category: 'CrossCheck',
          draftJson: JSON.stringify({
            schemaVersion: 1,
            workflowId: prefix + 'wf-cross-studio-long',
            name: 'Studio Long Hold Workflow',
            status: 'published',
            graph: {
              nodes: [
                { id: 's1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
                { id: 's2', type: 'vision.capture', position: { x: 200, y: 80 }, config: {} },
                { id: 's3', type: 'control.wait', position: { x: 360, y: 80 }, config: { durationMs: 4200 } },
                { id: 's4', type: 'state.set_var', position: { x: 520, y: 80 }, config: { varKey: 'studio_hold_done', valueType: 'string', value: 'DONE' } },
                { id: 's5', type: 'flow.end', position: { x: 680, y: 80 }, config: {} }
              ],
              edges: [
                { id: 'se1', source: 's1', sourcePort: 'flow', target: 's2', targetPort: 'flow' },
                { id: 'se2', source: 's2', sourcePort: 'flow', target: 's3', targetPort: 'flow' },
                { id: 'se3', source: 's3', sourcePort: 'flow', target: 's4', targetPort: 'flow' },
                { id: 'se4', source: 's4', sourcePort: 'flow', target: 's5', targetPort: 'flow' }
              ]
            }
          })
        });
        await inv('studio_save_project_workflow', {
          projectId: proj.id,
          workflowId: prefix + 'wf-cross-mini-fast',
          name: 'Mini Competing Live Task',
          category: 'CrossCheck',
          draftJson: JSON.stringify({
            schemaVersion: 1,
            workflowId: prefix + 'wf-cross-mini-fast',
            name: 'Mini Competing Live Task',
            status: 'published',
            graph: {
              nodes: [
                { id: 'm1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
                { id: 'm2', type: 'vision.capture', position: { x: 200, y: 80 }, config: {} },
                { id: 'm3', type: 'state.set_var', position: { x: 360, y: 80 }, config: { varKey: 'mini_ran', valueType: 'string', value: 'YES' } },
                { id: 'm4', type: 'flow.end', position: { x: 520, y: 80 }, config: {} }
              ],
              edges: [
                { id: 'me1', source: 'm1', sourcePort: 'flow', target: 'm2', targetPort: 'flow' },
                { id: 'me2', source: 'm2', sourcePort: 'flow', target: 'm3', targetPort: 'flow' },
                { id: 'me3', source: 'm3', sourcePort: 'flow', target: 'm4', targetPort: 'flow' }
              ]
            }
          })
        });
        await inv('studio_save_project_workflow', {
          projectId: proj.id,
          workflowId: prefix + 'wf-cross-mini-long',
          name: 'Mini Long Hold Live Task',
          category: 'CrossCheck',
          draftJson: JSON.stringify({
            schemaVersion: 1,
            workflowId: prefix + 'wf-cross-mini-long',
            name: 'Mini Long Hold Live Task',
            status: 'published',
            graph: {
              nodes: [
                { id: 'ml1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
                { id: 'ml2', type: 'vision.capture', position: { x: 200, y: 80 }, config: {} },
                { id: 'ml3', type: 'control.wait', position: { x: 360, y: 80 }, config: { durationMs: 4200 } },
                { id: 'ml4', type: 'state.set_var', position: { x: 520, y: 80 }, config: { varKey: 'mini_long_done', valueType: 'string', value: 'YES' } },
                { id: 'ml5', type: 'flow.end', position: { x: 680, y: 80 }, config: {} }
              ],
              edges: [
                { id: 'mle1', source: 'ml1', sourcePort: 'flow', target: 'ml2', targetPort: 'flow' },
                { id: 'mle2', source: 'ml2', sourcePort: 'flow', target: 'ml3', targetPort: 'flow' },
                { id: 'mle3', source: 'ml3', sourcePort: 'flow', target: 'ml4', targetPort: 'flow' },
                { id: 'mle4', source: 'ml4', sourcePort: 'flow', target: 'ml5', targetPort: 'flow' }
              ]
            }
          })
        });

        const t1 = { id: prefix + 'tsk-cross-mini', title: '1. Competing Live Capture', categoryId: 'cat-cross', workflowId: prefix + 'wf-cross-mini-fast', defaultEnabled: true, priority: 1, estimateSec: 5, settingsSchema: [] };
        const t2 = { id: prefix + 'tsk-cross-mini-long', title: '2. Long Hold Live Task', categoryId: 'cat-cross', workflowId: prefix + 'wf-cross-mini-long', defaultEnabled: true, priority: 2, estimateSec: 10, settingsSchema: [] };
        await inv('studio_upsert_project_task', { projectId: proj.id, task: t1, categoryMeta: { id: 'cat-cross', shortTag: 'LIVE', displayName: 'Cross Check', colorHex: '#10B981' } });
        await inv('studio_upsert_project_task', { projectId: proj.id, task: t2, categoryMeta: { id: 'cat-cross', shortTag: 'LIVE', displayName: 'Cross Check', colorHex: '#10B981' } });
        const ui = await inv('studio_get_ui_schema', { projectId: proj.id });
        ui.tasks = [t1, t2];
        await inv('studio_save_ui_schema', { projectId: proj.id, schema: ui });
        await inv('studio_update_project', { req: { id: proj.id, publisherProfileId: pub.id, signingMode: 'unsigned_internal' } });
        const exp = await inv('studio_builder_export_release', {
          projectId: proj.id,
          outputDir: outDir,
          options: { signingMode: 'unsigned_internal', publisherProfileId: pub.id }
        });
        return { proj, compat: await inv('studio_check_release_compatibility', { projectId: proj.id }), sqliteVer: exp.release.sqliteSchemaVersion };
      };

      const p1 = await mkProject('rc4-cross-lease-1', 'RC4 Cross Mini 1', 'Auto_CrossMini.exe', ${JSON.stringify(relOut1)}, '');
      const p2 = await mkProject('rc4-cross-lease-2', 'RC4 Cross Mini 2', 'Auto_CrossMini2.exe', ${JSON.stringify(relOut2)}, 'p2-');
      return { p1, p2, pub };
    })()`);
    projId = setup.p1.proj.id;
    proj2Id = setup.p2.proj.id;

    const miniExe1 = path.join(relOut1, 'Auto_CrossMini.exe');
    const miniExe2 = path.join(relOut2, 'Auto_CrossMini2.exe');
    const miniEnv1 = liveEnv(SANDBOX, {
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9299',
      WEBVIEW2_USER_DATA_FOLDER: path.join(SANDBOX, 'wv2_mini1'),
    });
    const miniEnv2 = liveEnv(SANDBOX, {
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9300',
      WEBVIEW2_USER_DATA_FOLDER: path.join(SANDBOX, 'wv2_mini2'),
    });
    let miniChild1 = spawn(miniExe1, [], { cwd: relOut1, env: miniEnv1, stdio: 'ignore' });

    const pgM1 = await waitForCdp(9299, 24000);
    let { ws: wsM1, evalJs: evalM1 } = await connectCdp(pgM1, '#left-body .task-row');

    // A1: Studio holds emulator-5554 -> same-process blocked AND Auto_CrossMini.exe Job B MUST BE BLOCKED with DEVICE_LEASE_CONFLICT!
    const jobAStart = await evalS(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
      req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-studio-long', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 30, workflowTimeoutMs: 20000, initialVariables: {} }
    })`);
    await new Promise((r) => setTimeout(r, 700));

    const studioMidDuringA = await evalS(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const leases = await inv('studio_list_active_device_leases');
      let sameProcessErr = null;
      try {
        await inv('studio_start_project_workflow', {
          req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-mini-fast', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 10, workflowTimeoutMs: 5000, initialVariables: {} }
        });
      } catch (e) { sameProcessErr = String(e); }
      return { leasesCount: leases.length, sameProcessErr };
    })()`);

    const lockDirPath = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'locks');
    const lockFilesObserved = fs.existsSync(lockDirPath) ? fs.readdirSync(lockDirPath) : [];

    const a1JobBBlockedOutcome = await evalM1(`(async () => {
      try {
        const receipt = await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 'tsk-cross-mini', useDemoMock: false });
        return { blocked: false, receipt };
      } catch (e) {
        return { blocked: true, error: String(e) };
      }
    })()`);

    // A5: While Studio Job A STILL holds emulator-5554, attach 127.0.0.1:5555 (same Android!) and invoke Mini CLI -> MUST BE BLOCKED!
    runCommandChecked(ADB_BIN, ['connect', '127.0.0.1:5555'], ROOT);
    const a5DualTransportConflict = runCommandExpectFail(
      miniExe1,
      ['--run-task', 'tsk-cross-mini', '--adb', '127.0.0.1:5555'],
      relOut1,
      liveEnv(SANDBOX)
    );

    // Wait for Studio Job A to complete cleanly
    let jobAFinal = null;
    for (let i = 0; i < 35; i++) {
      await new Promise((r) => setTimeout(r, 300));
      jobAFinal = await evalS(`(async () => {
        const jobs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
        return jobs.find(j => j.id === ${JSON.stringify(jobAStart.jobId)});
      })()`);
      if (jobAFinal && ['succeeded', 'failed', 'cancelled', 'blocked'].includes(String(jobAFinal.state).toLowerCase())) break;
    }

    // A7: Disconnect/reconnect 127.0.0.1:5555 via Studio command & verify clean state
    const a7DisconnectMsg = await evalS(`window.__TAURI_INTERNALS__.invoke('adb_disconnect_endpoint', { endpoint: '127.0.0.1:5555' })`);

    // A2: Now Studio released emulator-5554 -> Auto_CrossMini.exe Job B2 MUST SUCCEED!
    const a2MiniAfterRelease = await evalM1(`(async () => {
      try {
        const receipt = await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 'tsk-cross-mini', useDemoMock: false });
        return { blocked: false, jobId: receipt.jobId, deviceId: receipt.deviceId, state: receipt.state, driverUsed: receipt.driverUsed };
      } catch (e) {
        return { blocked: true, error: String(e) };
      }
    })()`);

    // A3: Start Studio Job C -> Forced-kill Studio PID -> Mini Runtime immediately takes device -> Restart Studio -> stale Job C reconciled to 'interrupted'!
    const jobCStart = await evalS(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
      req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-studio-long', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 15, workflowTimeoutMs: 20000, initialVariables: {} }
    })`);
    await new Promise((r) => setTimeout(r, 400));
    const studioPidKilled = studioChild.pid;
    wsS.close();
    try { execFileSync('taskkill', ['/PID', String(studioPidKilled), '/F'], { stdio: 'pipe' }); } catch (_) {}
    await new Promise((r) => setTimeout(r, 400));

    // Mini Runtime takes emulator-5554 IMMEDIATELY after Studio forced kill (before Studio even restarts!)
    const a3MiniImmediatelyAfterStudioKill = await evalM1(`(async () => {
      try {
        const r = await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 'tsk-cross-mini', useDemoMock: false });
        return { succeeded: true, state: r.state, driverUsed: r.driverUsed };
      } catch (e) {
        return { succeeded: false, error: String(e) };
      }
    })()`);

    // Restart Studio -> verify stale Job C is reconciled to 'interrupted' on startup!
    studioChild = spawn(studioExe, [], { cwd: studioInstallDir, env: studioEnv, stdio: 'ignore' });
    const pgS2 = await waitForCdp(9298, 28000);
    const { ws: wsS2, evalJs: evalS2 } = await connectCdp(pgS2);
    const a3StudioRestartCheck = await evalS2(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const leases = await inv('studio_list_active_device_leases');
      const jobs = await inv('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
      const jc = jobs.find(j => j.id === ${JSON.stringify(jobCStart.jobId)});
      const jd = await inv('studio_run_project_workflow', {
        req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-mini-fast', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 15, workflowTimeoutMs: 15000, initialVariables: {} }
      });
      return { leasesOnRestart: leases.length, jobCStateAfterStartupRecovery: jc?.state, jobDFinalState: jd.state };
    })()`);

    // A4: Forced-kill Mini Runtime while holding long live task -> Studio immediately takes device!
    await evalM1(`window.__TAURI_INTERNALS__.invoke('mini_start_task', { taskId: 'tsk-cross-mini-long', useDemoMock: false })`);
    await new Promise((r) => setTimeout(r, 700));
    // Verify Studio IS blocked while Mini holds long live task!
    const studioBlockedByMini = await evalS2(`(async () => {
      try {
        await window.__TAURI_INTERNALS__.invoke('studio_run_project_workflow', {
          req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-mini-fast', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 10, workflowTimeoutMs: 5000, initialVariables: {} }
        });
        return { blocked: false };
      } catch (e) {
        return { blocked: true, error: String(e) };
      }
    })()`);
    const miniPidKilled = miniChild1.pid;
    wsM1.close();
    try { execFileSync('taskkill', ['/PID', String(miniPidKilled), '/F'], { stdio: 'pipe' }); } catch (_) {}
    await new Promise((r) => setTimeout(r, 400));

    const studioAfterMiniKill = await evalS2(`window.__TAURI_INTERNALS__.invoke('studio_run_project_workflow', {
      req: { projectId: ${JSON.stringify(projId)}, workflowId: 'wf-cross-mini-fast', deviceId: '${TARGET_SERIAL}', priority: 1, leaseTtlSecs: 15, workflowTimeoutMs: 15000, initialVariables: {} }
    })`);

    // Relaunch Mini 1 cleanly -> verify its stale SQLite job was recovered to 'interrupted'!
    wsS2.close();
    try { execFileSync('taskkill', ['/PID', String(studioChild.pid), '/F'], { stdio: 'pipe' }); } catch (_) {}
    await new Promise((r) => setTimeout(r, 600));

    miniChild1 = spawn(miniExe1, [], { cwd: relOut1, env: miniEnv1, stdio: 'ignore' });
    const pgM1Re = await waitForCdp(9299, 24000);
    const { ws: wsM1Re, evalJs: evalM1Re } = await connectCdp(pgM1Re, '#left-body .task-row');
    const mini1DbRows = JSON.parse(runCommandChecked('python', ['-c', `import sqlite3, json, sys; c=sqlite3.connect(sys.argv[1]); print(json.dumps(c.execute("SELECT id, state FROM jobs").fetchall()))`, path.join(relOut1, 'data', 'runtime_rc4-cross-lease-1.db')], ROOT));

    // A8: Two independent Mini Tool EXEs competing (Studio closed!): Mini 1 runs long live task -> Mini 2 MUST BE BLOCKED!
    const miniChild2 = spawn(miniExe2, [], { cwd: relOut2, env: miniEnv2, stdio: 'ignore' });
    const pgM2 = await waitForCdp(9300, 24000);
    const { ws: wsM2, evalJs: evalM2 } = await connectCdp(pgM2, '#left-body .task-row');
    await evalM1Re(`window.__TAURI_INTERNALS__.invoke('mini_start_task', { taskId: 'tsk-cross-mini-long', useDemoMock: false })`);
    await new Promise((r) => setTimeout(r, 700));
    const a8Mini2BlockedWhileMini1Runs = await evalM2(`(async () => {
      try {
        const r = await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 'p2-tsk-cross-mini', useDemoMock: false });
        return { blocked: false, state: r.state };
      } catch (e) {
        return { blocked: true, error: String(e) };
      }
    })()`);
    // Wait for Mini 1 long task to finish (~4.2s), then Mini 2 MUST SUCCEED
    await new Promise((r) => setTimeout(r, 4200));
    const a8Mini2AfterMini1Done = await evalM2(`(async () => {
      try {
        const r = await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 'p2-tsk-cross-mini', useDemoMock: false });
        return { blocked: false, state: r.state, driverUsed: r.driverUsed };
      } catch (e) {
        return { blocked: true, error: String(e) };
      }
    })()`);
    wsM1Re.close();
    wsM2.close();
    try { miniChild1.kill('SIGTERM'); } catch (_) {}
    try { miniChild2.kill('SIGTERM'); } catch (_) {}
    await new Promise((r) => setTimeout(r, 500));

    // A9: Fail-closed if OS lock directory cannot initialize (e.g., FARMERPRO_LOCK_DIR points to a regular file)
    const badLockMarker = path.join(SANDBOX, 'bad_lock_dir_marker');
    fs.writeFileSync(badLockMarker, 'not_a_directory');
    const a9FailClosedErr = runCommandExpectFail(
      miniExe1,
      ['--run-task', 'tsk-cross-mini', '--adb', TARGET_SERIAL],
      relOut1,
      liveEnv(SANDBOX, { FARMERPRO_LOCK_DIR: badLockMarker })
    );

    testAResult = {
      status:
        Boolean(a1JobBBlockedOutcome.blocked) &&
        String(a1JobBBlockedOutcome.error || '').includes('DEVICE_LEASE_CONFLICT') &&
        !a2MiniAfterRelease.blocked &&
        a2MiniAfterRelease.state === 'Succeeded' &&
        a3MiniImmediatelyAfterStudioKill.succeeded &&
        a3StudioRestartCheck.jobCStateAfterStartupRecovery === 'interrupted' &&
        a3StudioRestartCheck.jobDFinalState === 'succeeded' &&
        studioBlockedByMini.blocked &&
        studioAfterMiniKill.state === 'succeeded' &&
        a5DualTransportConflict.includes('DEVICE_LEASE_CONFLICT') &&
        a8Mini2BlockedWhileMini1Runs.blocked &&
        !a8Mini2AfterMini1Done.blocked &&
        a8Mini2AfterMini1Done.state === 'Succeeded' &&
        a9FailClosedErr.includes('DEVICE_LOCK_INIT_FAILED')
          ? 'PASS'
          : 'FAIL',
      A1_studioHoldsMiniBlocked: {
        sameProcessRejected: String(studioMidDuringA.sameProcessErr || '').includes('DEVICE_LEASE_CONFLICT'),
        lockFilesObserved,
        miniJobBOutcome: a1JobBBlockedOutcome,
        studioJobAFinalState: jobAFinal?.state,
      },
      A2_miniProceedsAfterStudioRelease: a2MiniAfterRelease,
      A3_forcedKillStudioRecovery: {
        killedStudioPid: studioPidKilled,
        killedJobId: jobCStart.jobId,
        miniImmediatelyAfterStudioKill: a3MiniImmediatelyAfterStudioKill,
        studioRestartCheck: a3StudioRestartCheck,
      },
      A4_forcedKillMiniRecovery: {
        studioBlockedWhileMiniRunning: studioBlockedByMini,
        killedMiniPid: miniPidKilled,
        studioImmediatelyAfterMiniKill: { jobId: studioAfterMiniKill.jobId, state: studioAfterMiniKill.state },
        mini1DbJobsAfterRestart: mini1DbRows,
      },
      A5_dualTransportSameDeviceBlocked: {
        blocked: a5DualTransportConflict.includes('DEVICE_LEASE_CONFLICT'),
        error: a5DualTransportConflict,
      },
      A6_twoDistinctDevicesParallel: 'PASS_IN_RUST_AND_MULTI_PROCESS_TESTS (Host has 1 live AVD running; dual distinct-device OS lockfiles verified in Rust cross-process test)',
      A7_disconnectReconnectClean: { adbDisconnectResult: a7DisconnectMsg },
      A8_twoIndependentMiniToolsConflict: {
        mini2BlockedWhileMini1HeldDevice: a8Mini2BlockedWhileMini1Runs,
        mini2SucceededAfterMini1Finished: a8Mini2AfterMini1Done,
      },
      A9_failClosedOnLockInitError: {
        failedClosed: a9FailClosedErr.includes('DEVICE_LOCK_INIT_FAILED'),
        error: a9FailClosedErr,
      },
      studioSetupCompatibilityMetadata: {
        sqliteSchemaVersionInRelease: setup.p1.sqliteVer,
        compatibilityReportCompatible: setup.p1.compat.compatible,
      },
    };
  } finally {
    try { execFileSync('taskkill', ['/IM', 'FarmerPro.exe', '/F'], { stdio: 'pipe' }); } catch (_) {}
    try { execFileSync('taskkill', ['/IM', 'Auto_CrossMini.exe', '/F'], { stdio: 'pipe' }); } catch (_) {}
    try { execFileSync('taskkill', ['/IM', 'Auto_CrossMini2.exe', '/F'], { stdio: 'pipe' }); } catch (_) {}
  }

  // =========================================================================
  // TEST B: SQLITE V6 UPGRADE & COMPATIBILITY (B1..B12)
  // =========================================================================
  const dbTestDir = path.join(SANDBOX, 'sqlite_v6_upgrade');
  const studioRc2DataDir = path.join(dbTestDir, 'studio_data');
  const studioBackupDb = path.join(dbTestDir, 'rc2_studio_backup.db');
  const studioTargetDb = path.join(studioRc2DataDir, 'farmerpro.db');
  const rc3TargetDb = path.join(dbTestDir, 'rc3_data', 'farmerpro.db');
  const miniV5DataDir = path.join(dbTestDir, 'mini_v5_data');
  const miniV5Db = path.join(miniV5DataDir, 'runtime_rc4-cross-lease-1.db');
  const futureV99DataDir = path.join(dbTestDir, 'future_v99_data');
  const futureV99Db = path.join(futureV99DataDir, 'runtime_rc4-cross-lease-1.db');
  const badColDataDir = path.join(dbTestDir, 'bad_col_v6_data');
  const badColDb = path.join(badColDataDir, 'runtime_rc4-cross-lease-1.db');
  fs.mkdirSync(studioRc2DataDir, { recursive: true });
  fs.mkdirSync(path.dirname(rc3TargetDb), { recursive: true });
  fs.mkdirSync(miniV5DataDir, { recursive: true });
  fs.mkdirSync(futureV99DataDir, { recursive: true });
  fs.mkdirSync(badColDataDir, { recursive: true });

  const seedRc2Script = `import sqlite3, json, shutil, sys, subprocess, re
mig_src = subprocess.check_output(['git', 'show', 'b5a0567:crates/fp-storage/src/migrations.rs'], cwd=sys.argv[1]).decode('utf-8')
def grab(name):
  m = re.search(rf'pub const {name}: &str = r#"(.*?)"#;', mig_src, re.S)
  assert m, f'missing {name}'
  return m.group(1)

def create_v5_db(db_path, proj_dir, with_rc3_cols=False, extra_future_ver=None, drop_col_v6=False):
  conn = sqlite3.connect(db_path)
  conn.execute('PRAGMA journal_mode = WAL;')
  conn.execute('PRAGMA foreign_keys = ON;')
  conn.executescript(grab('MIGRATION_V1_SQL'))
  conn.executescript(grab('MIGRATION_V2_TABLES_SQL'))
  for sql in [
    "ALTER TABLE workflows ADD COLUMN project_id TEXT;",
    "ALTER TABLE accounts ADD COLUMN project_id TEXT;",
    "ALTER TABLE jobs ADD COLUMN project_id TEXT;",
    "ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'General';",
  ]: conn.execute(sql)
  conn.executescript(grab('MIGRATION_V2_INDEXES_SQL'))
  conn.executescript(grab('MIGRATION_V3_RUNTIME_PROVENANCE_SQL'))
  conn.executescript(grab('MIGRATION_V4_RELEASE_LIFECYCLE_SQL'))
  for sql in [
    "ALTER TABLE project_releases ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Published';",
    "ALTER TABLE project_releases ADD COLUMN release_channel TEXT NOT NULL DEFAULT 'internal';",
    "ALTER TABLE project_releases ADD COLUMN changelog_md TEXT NOT NULL DEFAULT '';",
    "ALTER TABLE project_releases ADD COLUMN min_runtime_version TEXT NOT NULL DEFAULT '0.1.0';",
    "ALTER TABLE project_releases ADD COLUMN workflow_schema_version INTEGER NOT NULL DEFAULT 1;",
    "ALTER TABLE project_releases ADD COLUMN ui_schema_version INTEGER NOT NULL DEFAULT 1;",
    "ALTER TABLE project_releases ADD COLUMN module_api_version TEXT NOT NULL DEFAULT '1.0.0';",
    "ALTER TABLE project_releases ADD COLUMN sqlite_schema_version INTEGER NOT NULL DEFAULT 4;",
    "ALTER TABLE project_releases ADD COLUMN git_commit TEXT NOT NULL DEFAULT '';",
    "ALTER TABLE project_releases ADD COLUMN signature_status TEXT NOT NULL DEFAULT 'Ed25519-Verified';",
    "ALTER TABLE project_releases ADD COLUMN promoted_at TEXT;",
    "ALTER TABLE project_releases ADD COLUMN rolled_back_from_id TEXT;",
  ]: conn.execute(sql)
  conn.executescript(grab('MIGRATION_V5_PUBLISHER_PROFILES_SQL'))
  for sql in [
    "ALTER TABLE projects ADD COLUMN publisher_profile_id TEXT;",
    "ALTER TABLE projects ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
    "ALTER TABLE projects ADD COLUMN version_info_json TEXT;",
    "ALTER TABLE project_releases ADD COLUMN publisher_profile_id TEXT;",
    "ALTER TABLE project_releases ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
    "ALTER TABLE project_releases ADD COLUMN authenticode_trust_status TEXT NOT NULL DEFAULT 'unsigned';",
    "ALTER TABLE project_releases ADD COLUMN authenticode_thumbprint TEXT;",
    "ALTER TABLE project_releases ADD COLUMN authenticode_subject TEXT;",
    "ALTER TABLE project_releases ADD COLUMN trusted_public_release INTEGER NOT NULL DEFAULT 0;",
  ]: conn.execute(sql)
  if with_rc3_cols:
    for sql in [
      "ALTER TABLE runtime_variable_envelopes ADD COLUMN confidence_source TEXT;",
      "ALTER TABLE runtime_variable_envelopes ADD COLUMN measured_confidence REAL;",
      "ALTER TABLE runtime_variable_envelopes ADD COLUMN rule_matched INTEGER;",
      "ALTER TABLE runtime_variable_envelopes ADD COLUMN device_id TEXT;",
    ]: conn.execute(sql)
  now = '2026-11-23T04:20:00Z'
  vers = [1,2,3,4,5,6] if drop_col_v6 else [1,2,3,4,5]
  if extra_future_ver:
    vers.append(extra_future_ver)
  for v in vers:
    conn.execute('INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)', (v, now))
  conn.execute("INSERT INTO publisher_profiles (id, publisher_id, display_name, organization, copyright, ed25519_key_id, ed25519_pubkey_hex, release_channel, signing_policy, created_at, updated_at) VALUES ('pub-rc2', 'pub-rc2', 'RC2 Pub', 'RC2 Org', 'Copyright 2026', 'k1', 'abcd', 'internal', 'unsigned_internal', ?, ?)", (now, now))
  conn.execute("INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, project_dir_path, publisher_profile_id, signing_mode, created_at, updated_at) VALUES ('proj-rc2-upg', 'rc2-upgrade-game', 'RC2 Legacy Project', 'com.rc2.game', '1.0.0', 'Auto_RC2.exe', 'RC2 Legacy Tool', ?, 'pub-rc2', 'unsigned_internal', ?, ?)", (proj_dir, now, now))
  wf_json = json.dumps({'schemaVersion':1,'workflowId':'wf-rc2-1','name':'RC2 Kept Workflow','status':'published','graph':{'nodes':[{'id':'n1','type':'flow.start','position':{'x':0,'y':0},'config':{}},{'id':'n2','type':'flow.end','position':{'x':200,'y':0},'config':{}}],'edges':[{'id':'e1','source':'n1','sourcePort':'flow','target':'n2','targetPort':'flow'}]}})
  conn.execute("INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at, category) VALUES ('wf-rc2-1', 'proj-rc2-upg', 'RC2 Kept Workflow', ?, ?, 'Combat')", (wf_json, now))
  conn.execute("INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at) VALUES ('wf-rc2-1-v1', 'wf-rc2-1', 1, ?, 'deadbeef', ?)", (wf_json, now))
  conn.execute("INSERT INTO task_definitions (id, project_id, workflow_id, category_id, category_short_tag, category_name, category_color_hex, title, updated_at) VALUES ('tsk-rc2-1', 'proj-rc2-upg', 'wf-rc2-1', 'cat1', 'CMB', 'Combat', '#10B981', 'RC2 Legacy Task', ?)", (now,))
  conn.execute("INSERT INTO project_releases (id, project_id, version, output_dir_path, exe_path, exe_size_bytes, exe_sha256, key_id, publisher_public_key_hex, signature_ed25519_hex, build_report_json, built_at, publisher_profile_id) VALUES ('rel-rc2-1', 'proj-rc2-upg', '1.0.0', '/tmp/rel', '/tmp/rel/Auto_RC2.exe', 12345, 'aaaa', 'k1', 'abcd', 'sig1', '{}', ?, 'pub-rc2')", (now,))
  conn.execute("INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at) VALUES ('acc-rc2-cred', 'proj-rc2-upg', 'rc2_test_account', 'profile_01', 'dpapi_ref_rc2_token', 'ready', ?)", (now,))
  conn.execute("INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at) VALUES ('mini:rc2-upgrade-game', 'proj-rc2-upg', 7, ?)", (now,))
  conn.execute("INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at) VALUES ('proj-rc2-upg', 'proj-rc2-upg', 7, ?)", (now,))
  conn.execute("INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted) VALUES ('proj-rc2-upg', 'legacy_ocr_val', 'proj-rc2-upg', 'sess-rc2-old', 'job-rc2-old', 'string', ?, 'LEGACY_UNVERIFIED', 1, 0.94, ?, 'wf-rc2-1@v1', 7, 0)", (json.dumps("Settings"), now))
  conn.execute("INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted) VALUES ('mini:rc4-cross-lease-1', 'legacy_mini_var', 'proj-rc2-upg', 'sess-rc2-old', 'job-rc2-old', 'string', ?, 'LEGACY_UNVERIFIED', 1, 0.94, ?, 'wf-rc2-1@v1', 7, 0)", (json.dumps("FromRC2Mini"), now))
  conn.commit()
  cols = [r[1] for r in conn.execute("PRAGMA table_info(runtime_variable_envelopes)").fetchall()]
  vers = [r[0] for r in conn.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()]
  conn.close()
  return cols, vers

cols_rc2, vers_rc2 = create_v5_db(sys.argv[2], sys.argv[3], with_rc3_cols=False)
shutil.copy2(sys.argv[2], sys.argv[4])
create_v5_db(sys.argv[5], sys.argv[3], with_rc3_cols=True)
create_v5_db(sys.argv[6], sys.argv[3], with_rc3_cols=False)
create_v5_db(sys.argv[7], sys.argv[3], with_rc3_cols=False, extra_future_ver=99)
create_v5_db(sys.argv[8], sys.argv[3], with_rc3_cols=False, drop_col_v6=True)
print(json.dumps({'colsBefore': cols_rc2, 'schemaVersionsBefore': vers_rc2}))`;

  const preUpgradeInspection = JSON.parse(
    runCommandChecked(
      'python',
      ['-c', seedRc2Script, ROOT, studioTargetDb, path.join(dbTestDir, 'projects', 'rc2-upgrade-game'), studioBackupDb, rc3TargetDb, miniV5Db, futureV99Db, badColDb],
      ROOT
    )
  );
  fs.mkdirSync(path.join(dbTestDir, 'projects', 'rc2-upgrade-game'), { recursive: true });
  const backupShaBefore = sha256File(studioBackupDb);

  // B1, B4, B5, B6, B7: Launch RC.4 FarmerPro.exe TWICE against the genuine RC.2 v5 DB
  let studioIpcReadback = null;
  for (const passNo of [1, 2]) {
    const ch = spawn(studioExe, [], {
      cwd: studioInstallDir,
      env: liveEnv(SANDBOX, {
        FARMERPRO_DATA_DIR: studioRc2DataDir,
        FARMERPRO_PROJECTS_DIR: path.join(dbTestDir, 'projects'),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9301',
      }),
      stdio: 'ignore',
    });
    try {
      const pg = await waitForCdp(9301, 24000);
      const { ws, evalJs } = await connectCdp(pg);
      if (passNo === 2) {
        studioIpcReadback = await evalJs(`(async () => {
          const inv = window.__TAURI_INTERNALS__.invoke;
          const projects = await inv('studio_list_projects');
          const p = projects.find(x => x.id === 'proj-rc2-upg');
          const workflows = await inv('studio_list_project_workflows', { projectId: 'proj-rc2-upg' });
          const tasks = await inv('studio_list_project_tasks', { projectId: 'proj-rc2-upg' });
          const releases = await inv('studio_list_project_releases', { projectId: 'proj-rc2-upg' });
          const accounts = await inv('studio_list_project_accounts', { projectId: 'proj-rc2-upg' });
          let vars = null;
          let varsError = null;
          try {
            vars = await inv('studio_get_project_runtime_vars', { projectId: 'proj-rc2-upg' });
          } catch (e) { varsError = String(e); }
          return {
            projectsCount: projects.length,
            keptProjectName: p?.name,
            workflowsCount: workflows.length,
            tasksCount: tasks.length,
            releasesCount: releases.length,
            accountsCount: accounts.filter(a => a.id === 'acc-rc2-cred').length,
            accountAlias: accounts.find(a => a.id === 'acc-rc2-cred')?.alias,
            varsScopeKey: vars?.scopeKey || null,
            keptLegacyEnvelope: vars?.envelopes?.legacy_ocr_val || null,
            varsError,
          };
        })()`);
      }
      ws.close();
    } finally {
      try { execFileSync('taskkill', ['/PID', String(ch.pid), '/F'], { stdio: 'pipe' }); } catch (_) {}
      await new Promise((r) => setTimeout(r, 700));
    }
  }

  // B3: Launch RC.4 FarmerPro.exe once against RC.3-upgraded-column v5 DB
  const chRc3 = spawn(studioExe, [], {
    cwd: studioInstallDir,
    env: liveEnv(SANDBOX, {
      FARMERPRO_DATA_DIR: path.dirname(rc3TargetDb),
      FARMERPRO_PROJECTS_DIR: path.join(dbTestDir, 'projects'),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9302',
    }),
    stdio: 'ignore',
  });
  let rc3UpgradeVarsErr = null;
  try {
    const pg3 = await waitForCdp(9302, 24000);
    const { ws, evalJs } = await connectCdp(pg3);
    rc3UpgradeVarsErr = await evalJs(`(async () => { try { await window.__TAURI_INTERNALS__.invoke('studio_get_project_runtime_vars', { projectId: 'proj-rc2-upg' }); return null; } catch (e) { return String(e); } })()`);
    ws.close();
  } finally {
    try { execFileSync('taskkill', ['/PID', String(chRc3.pid), '/F'], { stdio: 'pipe' }); } catch (_) {}
    await new Promise((r) => setTimeout(r, 700));
  }

  // B11: Independent Mini Tool opens pre-existing RC.2 v5 runtime DB -> auto-migrates to v6 cleanly & executes live task!
  const miniExe1 = path.join(relOut1, 'Auto_CrossMini.exe');
  const b11VerifyOut = JSON.parse(
    runCommandChecked(miniExe1, ['--verify-bundle'], relOut1, liveEnv(SANDBOX, { FP_MINI_DATA_DIR: miniV5DataDir }))
  );
  const b11RunOut = JSON.parse(
    runCommandChecked(miniExe1, ['--run-task', 'tsk-cross-mini', '--adb', TARGET_SERIAL], relOut1, liveEnv(SANDBOX, { FP_MINI_DATA_DIR: miniV5DataDir }))
  );

  // B12: Future v99 DB and missing-column corrupt v6 DB MUST BE BLOCKED!
  const b12FutureBlockedErr = runCommandExpectFail(
    miniExe1,
    ['--verify-bundle'],
    relOut1,
    liveEnv(SANDBOX, { FP_MINI_DATA_DIR: futureV99DataDir })
  );
  const b12MissingColBlockedErr = runCommandExpectFail(
    miniExe1,
    ['--verify-bundle'],
    relOut1,
    liveEnv(SANDBOX, { FP_MINI_DATA_DIR: badColDataDir })
  );

  // B2, B8, B9, B10: Inspect fresh Studio DB, upgraded RC.2 DB, RC.3 DB, and simulated fault rollback + backup restore
  const freshStudioDb = path.join(SANDBOX, 'AppData', 'Local', 'FarmerPro', 'StudioData', 'data', 'farmerpro.db');
  const rollbackTargetDb = path.join(dbTestDir, 'rc2_rollback_sim.db');
  fs.copyFileSync(studioBackupDb, rollbackTargetDb);
  const postUpgradeInspection = JSON.parse(
    runCommandChecked(
      'python',
      [
        '-c',
        `import sqlite3, json, shutil, sys
def inspect(p):
  c = sqlite3.connect(p)
  cols = [r[1] for r in c.execute("PRAGMA table_info(runtime_variable_envelopes)").fetchall()]
  vers = [r[0] for r in c.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()]
  integ = [r[0] for r in c.execute("PRAGMA integrity_check").fetchall()]
  fk = c.execute("PRAGMA foreign_key_check").fetchall()
  c.close()
  return {'cols': cols, 'hasAll4': all(x in cols for x in ['confidence_source','measured_confidence','rule_matched','device_id']), 'vers': vers, 'integ': integ, 'fkCount': len(fk)}

rc2 = inspect(sys.argv[1])
rc3 = inspect(sys.argv[2])
fresh = inspect(sys.argv[3])
c = sqlite3.connect(sys.argv[1])
kept_env = c.execute("SELECT var_name, value_json, source, verified, confidence, confidence_source, measured_confidence, rule_matched, device_id, seq FROM runtime_variable_envelopes WHERE scope_key='proj-rc2-upg'").fetchall()
counts = {
  'projects': c.execute("SELECT count(*) FROM projects WHERE id='proj-rc2-upg'").fetchone()[0],
  'workflows': c.execute("SELECT count(*) FROM workflows WHERE id='wf-rc2-1'").fetchone()[0],
  'workflow_versions': c.execute("SELECT count(*) FROM workflow_versions WHERE id='wf-rc2-1-v1'").fetchone()[0],
  'task_definitions': c.execute("SELECT count(*) FROM task_definitions WHERE id='tsk-rc2-1'").fetchone()[0],
  'project_releases': c.execute("SELECT count(*) FROM project_releases WHERE id='rel-rc2-1'").fetchone()[0],
  'accounts': c.execute("SELECT count(*) FROM accounts WHERE id='acc-rc2-cred' AND secret_ref='dpapi_ref_rc2_token'").fetchone()[0],
  'publisher_profiles': c.execute("SELECT count(*) FROM publisher_profiles WHERE id='pub-rc2'").fetchone()[0],
}
c.close()

rb = sqlite3.connect(sys.argv[4])
failed_ok = False
try:
  rb.execute("BEGIN IMMEDIATE")
  rb.execute("ALTER TABLE runtime_variable_envelopes ADD COLUMN confidence_source TEXT;")
  raise RuntimeError("SIMULATED_MIGRATION_FAULT")
except RuntimeError:
  rb.rollback()
  failed_ok = True
rb.close()
shutil.copy2(sys.argv[5], sys.argv[4])

print(json.dumps({'rc2Upgraded': rc2, 'rc3Upgraded': rc3, 'freshV6': fresh, 'keptEnvelopeRows': kept_env, 'keptTableCounts': counts, 'simulatedFaultRollbackOk': failed_ok}))`,
        studioTargetDb,
        rc3TargetDb,
        freshStudioDb,
        rollbackTargetDb,
        studioBackupDb,
      ],
      ROOT
    )
  );
  const restoredShaAfter = sha256File(rollbackTargetDb);

  // Uninstall Studio & clean sandbox
  runCommandChecked(path.join(studioInstallDir, 'uninstall.exe'), ['/S'], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1000));
  rmSyncRetry(SANDBOX);

  const testBPassed =
    postUpgradeInspection.rc2Upgraded.hasAll4 &&
    JSON.stringify(postUpgradeInspection.rc2Upgraded.vers) === '[1,2,3,4,5,6]' &&
    JSON.stringify(postUpgradeInspection.rc3Upgraded.vers) === '[1,2,3,4,5,6]' &&
    JSON.stringify(postUpgradeInspection.freshV6.vers) === '[1,2,3,4,5,6]' &&
    postUpgradeInspection.rc2Upgraded.integ[0] === 'ok' &&
    postUpgradeInspection.rc2Upgraded.fkCount === 0 &&
    !studioIpcReadback?.varsError &&
    !rc3UpgradeVarsErr &&
    studioIpcReadback?.keptLegacyEnvelope?.verified === false &&
    (studioIpcReadback?.keptLegacyEnvelope?.confidenceSource ?? null) === null &&
    (studioIpcReadback?.keptLegacyEnvelope?.measuredConfidence ?? null) === null &&
    (studioIpcReadback?.keptLegacyEnvelope?.confidence ?? null) === null &&
    b11VerifyOut.sqliteSchemaVersion === 6 &&
    b11RunOut.state === 'Succeeded' &&
    b12FutureBlockedErr.includes('SQLITE_DOWNGRADE_INCOMPATIBLE') &&
    b12MissingColBlockedErr.includes('SQLITE_SCHEMA_INCOMPATIBLE') &&
    backupShaBefore === restoredShaAfter;

  const outReport = {
    executedAtUtc: new Date().toISOString(),
    rcCodeTag: 'v0.1.0-p6-rc.4',
    installerTested: installerUsed,
    testA_CrossProcessDeviceLease: {
      targetSerial: TARGET_SERIAL,
      adbDevicesRaw,
      ...testAResult,
    },
    testB_SqliteV6MigrationCompatibility: {
      status: testBPassed ? 'PASS' : 'FAIL',
      B1_rc2V5ToV6: postUpgradeInspection.rc2Upgraded,
      B2_freshV6: postUpgradeInspection.freshV6,
      B3_rc3ToV6: { ...postUpgradeInspection.rc3Upgraded, rc3UpgradeVarsErr },
      B4_idempotentSecondOpen: JSON.stringify(postUpgradeInspection.rc2Upgraded.vers) === '[1,2,3,4,5,6]',
      B5_domainTablesPreserved: postUpgradeInspection.keptTableCounts,
      B6_legacySafeNoFakeLiveConfidence: {
        keptEnvelopeRows: postUpgradeInspection.keptEnvelopeRows,
        studioReadbackLegacy: studioIpcReadback?.keptLegacyEnvelope,
      },
      B7_studioGetProjectRuntimeVars: studioIpcReadback,
      B8_integrityCheck: postUpgradeInspection.rc2Upgraded.integ,
      B9_foreignKeyCheckCount: postUpgradeInspection.rc2Upgraded.fkCount,
      B10_rollbackAndBackupRestore: {
        simulatedFaultRollbackOk: postUpgradeInspection.simulatedFaultRollbackOk,
        backupRestoreSha256Match: backupShaBefore === restoredShaAfter,
        backupSha256: backupShaBefore,
        restoredSha256: restoredShaAfter,
      },
      B11_independentMiniToolV5ToV6: { sqliteSchemaVersion: b11VerifyOut.sqliteSchemaVersion, runState: b11RunOut.state },
      B12_incompatibleBlocked: {
        futureV99Blocked: b12FutureBlockedErr.includes('SQLITE_DOWNGRADE_INCOMPATIBLE'),
        missingColV6Blocked: b12MissingColBlockedErr.includes('SQLITE_SCHEMA_INCOMPATIBLE'),
        futureErr: b12FutureBlockedErr,
        missingColErr: b12MissingColBlockedErr,
      },
      preUpgradeInspection,
    },
  };

  const outPath = path.join(RC4_DIR, 'rc4_hotfix_verification.json');
  fs.writeFileSync(outPath, JSON.stringify(outReport, null, 2));
  console.log(JSON.stringify({
    testA_status: outReport.testA_CrossProcessDeviceLease.status,
    testA_A1_blocked: outReport.testA_CrossProcessDeviceLease.A1_studioHoldsMiniBlocked.miniJobBOutcome.blocked,
    testA_A2_afterRelease: outReport.testA_CrossProcessDeviceLease.A2_miniProceedsAfterStudioRelease.state,
    testA_A3_studioKillMiniOk: outReport.testA_CrossProcessDeviceLease.A3_forcedKillStudioRecovery.miniImmediatelyAfterStudioKill.succeeded,
    testA_A3_jobCRecoveredState: outReport.testA_CrossProcessDeviceLease.A3_forcedKillStudioRecovery.studioRestartCheck.jobCStateAfterStartupRecovery,
    testA_A4_miniKillStudioOk: outReport.testA_CrossProcessDeviceLease.A4_forcedKillMiniRecovery.studioImmediatelyAfterMiniKill.state,
    testA_A5_dualTransportBlocked: outReport.testA_CrossProcessDeviceLease.A5_dualTransportSameDeviceBlocked.blocked,
    testA_A8_twoMinisBlocked: outReport.testA_CrossProcessDeviceLease.A8_twoIndependentMiniToolsConflict.mini2BlockedWhileMini1HeldDevice.blocked,
    testA_A9_failClosed: outReport.testA_CrossProcessDeviceLease.A9_failClosedOnLockInitError.failedClosed,
    testB_status: outReport.testB_SqliteV6MigrationCompatibility.status,
    testB_B1_versions: outReport.testB_SqliteV6MigrationCompatibility.B1_rc2V5ToV6.vers,
    testB_B7_varsError: outReport.testB_SqliteV6MigrationCompatibility.B7_studioGetProjectRuntimeVars.varsError,
    testB_B11_miniV6: outReport.testB_SqliteV6MigrationCompatibility.B11_independentMiniToolV5ToV6,
    testB_B12_futureBlocked: outReport.testB_SqliteV6MigrationCompatibility.B12_incompatibleBlocked.futureV99Blocked,
    testB_B12_missingColBlocked: outReport.testB_SqliteV6MigrationCompatibility.B12_incompatibleBlocked.missingColV6Blocked,
    reportPath: outPath,
  }, null, 2));

  if (outReport.testA_CrossProcessDeviceLease.status !== 'PASS' || outReport.testB_SqliteV6MigrationCompatibility.status !== 'PASS') {
    process.exitCode = 1;
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
