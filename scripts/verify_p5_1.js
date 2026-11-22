const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p5_1_evidence');
const SANDBOX = path.join(EV_DIR, 'sandbox');

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

async function waitForCdp(port, timeoutMs = 22000) {
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

async function connectCdp(page) {
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
    await new Promise((r) => setTimeout(r, 300));
    const res = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(res.data, 'base64'));
    return { filename, width, height };
  };

  for (let i = 0; i < 40; i++) {
    const ready = await evalJs('Boolean(window.__TAURI_INTERNALS__)');
    if (ready) break;
    await new Promise((r) => setTimeout(r, 250));
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

async function main() {
  fs.mkdirSync(EV_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) fs.rmSync(SANDBOX, { recursive: true, force: true });
  fs.mkdirSync(path.join(SANDBOX, 'data'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'projects'), { recursive: true });

  const sandboxExe = path.join(SANDBOX, 'FarmerPro.exe');
  fs.copyFileSync(path.join(ROOT, 'target', 'debug', 'farmerpro.exe'), sandboxExe);
  copyDirSync(path.join(ROOT, 'bin'), path.join(SANDBOX, 'bin'));
  fs.mkdirSync(path.join(SANDBOX, 'target', 'debug'), { recursive: true });
  fs.copyFileSync(
    path.join(ROOT, 'target', 'debug', 'fp-mini-runtime.exe'),
    path.join(SANDBOX, 'target', 'debug', 'fp-mini-runtime.exe')
  );

  const { adbOut, ldRunning } = getAdbStatus();

  // TEST G Step 1-5: Launch FarmerPro Studio, create 2 projects with multi-step timed var workflow & secrets, export BOTH EXEs, then close Studio!
  let alphaExePath = '';
  let betaExePath = '';
  let studioConsoleErrors = [];

  const studioChild = spawn(sandboxExe, [], {
    cwd: SANDBOX,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9260',
    },
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9260, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors = consoleErrors;

    const seeded = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const p1 = await inv('studio_create_project', {
        req: {
          slug: 'realtime-alpha',
          name: 'Realtime Alpha Harvester',
          gamePackageId: 'com.alpha.realtime',
          version: '1.0.0',
          exeName: 'Auto_RealtimeAlpha.exe',
          windowTitle: 'Realtime Alpha v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });

      const wfAlpha = {
        schemaVersion: 1,
        workflowId: 'wf-alpha-rt',
        name: 'Realtime Timed Vars Flow',
        status: 'published',
        graph: {
          nodes: [
            { id: 'n-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'n-pin', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'account_pin', valueType: 'number', value: 482910 } },
            { id: 'n-auth', type: 'state.set_var', position: { x: 360, y: 80 }, config: { varKey: 'auth_bundle', valueType: 'json', value: { token: 'tok-secret-999', tier: 2 } } },
            { id: 'n-zone', type: 'state.set_var', position: { x: 520, y: 80 }, config: { varKey: 'farm_zone', valueType: 'string', value: 'Sector-B7' } },
            { id: 'n-base', type: 'state.set_var', position: { x: 680, y: 80 }, config: { varKey: 'resource_count', valueType: 'number', value: 100 } },
            { id: 'n-wait-1', type: 'control.wait', position: { x: 840, y: 80 }, config: { durationMs: 2600 } },
            { id: 'n-inc', type: 'state.increment_var', position: { x: 1000, y: 80 }, config: { varKey: 'resource_count', delta: 150 } },
            { id: 'n-wait-2', type: 'control.wait', position: { x: 1160, y: 80 }, config: { durationMs: 1800 } },
            { id: 'n-end', type: 'flow.end', position: { x: 1320, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'e1', source: 'n-start', sourcePort: 'flow', target: 'n-pin', targetPort: 'flow' },
            { id: 'e2', source: 'n-pin', sourcePort: 'flow', target: 'n-auth', targetPort: 'flow' },
            { id: 'e3', source: 'n-auth', sourcePort: 'flow', target: 'n-zone', targetPort: 'flow' },
            { id: 'e4', source: 'n-zone', sourcePort: 'flow', target: 'n-base', targetPort: 'flow' },
            { id: 'e5', source: 'n-base', sourcePort: 'flow', target: 'n-wait-1', targetPort: 'flow' },
            { id: 'e6', source: 'n-wait-1', sourcePort: 'flow', target: 'n-inc', targetPort: 'flow' },
            { id: 'e7', source: 'n-inc', sourcePort: 'flow', target: 'n-wait-2', targetPort: 'flow' },
            { id: 'e8', source: 'n-wait-2', sourcePort: 'flow', target: 'n-end', targetPort: 'flow' }
          ]
        }
      };
      await inv('studio_save_project_workflow', {
        projectId: p1.id,
        workflowId: 'wf-alpha-rt',
        name: 'Realtime Timed Vars Flow',
        category: 'Resource',
        draftJson: JSON.stringify(wfAlpha)
      });
      await inv('studio_upsert_project_task', {
        projectId: p1.id,
        task: { id: 't-alpha', title: 'Thu hoạch & Đồng bộ Realtime', categoryId: 'cat.rt', workflowId: 'wf-alpha-rt', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [] },
        categoryMeta: { id: 'cat.rt', shortTag: 'RT', displayName: 'Realtime', colorHex: '#1878F5' }
      });

      const schema1 = {
        schemaVersion: 1,
        projectSlug: 'realtime-alpha',
        windowTitle: 'Realtime Alpha v1.0.0',
        versionBadge: 'v1.0.0',
        themeAccentHex: '#1878F5',
        headerBgHex: '#121E2A',
        layoutPreset: 'compact1_col',
        defaultWindowSize: [740, 500],
        sectionsOrder: ['header_bar', 'device_table', 'task_checklist', 'quick_log', 'footer_controls'],
        hiddenComponents: ['utilities_panel'],
        deviceColumns: [
          { id: 'col-zone', label: 'Khu vực', runtimeVarKey: 'farm_zone', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 420, badgeColor: '#38BDF8' },
          { id: 'col-res', label: 'Tài nguyên', runtimeVarKey: 'resource_count', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 480, badgeColor: '#3BC38D' },
          { id: 'col-done', label: 'Hoàn thành', runtimeVarKey: 'task_completed_count', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 540, badgeColor: '#F59E0B' }
        ],
        taskCategories: [{ id: 'cat.rt', shortTag: 'RT', displayName: 'Realtime', colorHex: '#1878F5' }],
        tasks: [{ id: 't-alpha', title: 'Thu hoạch & Đồng bộ Realtime', categoryId: 'cat.rt', workflowId: 'wf-alpha-rt', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [] }],
        utilities: [],
        initialRuntimeVars: { __uiCompactDensity: 'compact', __uiCollapsedSections: [], __uiLogMaxHeightPx: 145 }
      };
      await inv('studio_save_ui_schema', { projectId: p1.id, schema: schema1 });
      await inv('studio_ensure_project_icon', { projectId: p1.id, accentHex: '#1878F5', overwrite: true });
      const exp1 = await inv('studio_builder_export_release', { projectId: p1.id });

      // Project 2: Titan Beta (identical variable keys, distinct isolated project)
      const p2 = await inv('studio_create_project', {
        req: {
          slug: 'titan-beta',
          name: 'Titan Beta Arena',
          gamePackageId: 'com.titan.beta',
          version: '2.0.0',
          exeName: 'Auto_TitanBeta.exe',
          windowTitle: 'Titan Beta v2.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      const wfBeta = {
        schemaVersion: 1,
        workflowId: 'wf-beta-rt',
        name: 'Titan Beta Flow',
        status: 'published',
        graph: {
          nodes: [
            { id: 'b-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'b-zone', type: 'state.set_var', position: { x: 240, y: 80 }, config: { varKey: 'farm_zone', valueType: 'string', value: 'Titan-Ridge' } },
            { id: 'b-inc', type: 'state.increment_var', position: { x: 440, y: 80 }, config: { varKey: 'resource_count', delta: 999 } },
            { id: 'b-end', type: 'flow.end', position: { x: 640, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'be1', source: 'b-start', sourcePort: 'flow', target: 'b-zone', targetPort: 'flow' },
            { id: 'be2', source: 'b-zone', sourcePort: 'flow', target: 'b-inc', targetPort: 'flow' },
            { id: 'be3', source: 'b-inc', sourcePort: 'flow', target: 'b-end', targetPort: 'flow' }
          ]
        }
      };
      await inv('studio_save_project_workflow', {
        projectId: p2.id,
        workflowId: 'wf-beta-rt',
        name: 'Titan Beta Flow',
        category: 'Arena',
        draftJson: JSON.stringify(wfBeta)
      });
      await inv('studio_upsert_project_task', {
        projectId: p2.id,
        task: { id: 't-beta', title: 'Đấu trường Titan Độc lập', categoryId: 'cat.pvp', workflowId: 'wf-beta-rt', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [] },
        categoryMeta: { id: 'cat.pvp', shortTag: 'PVP', displayName: 'PvP', colorHex: '#8B5CF6' }
      });
      const schema2 = {
        schemaVersion: 1,
        projectSlug: 'titan-beta',
        windowTitle: 'Titan Beta v2.0.0',
        versionBadge: 'v2.0.0',
        themeAccentHex: '#8B5CF6',
        headerBgHex: '#18142C',
        layoutPreset: 'expanded',
        defaultWindowSize: [900, 540],
        sectionsOrder: ['header_bar', 'device_table', 'task_checklist', 'quick_log', 'footer_controls'],
        hiddenComponents: ['utilities_panel'],
        deviceColumns: [
          { id: 'bcol-zone', label: 'Khu vực', runtimeVarKey: 'farm_zone', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 420, badgeColor: '#A78BFA' },
          { id: 'bcol-res', label: 'Tài nguyên', runtimeVarKey: 'resource_count', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 480, badgeColor: '#3BC38D' }
        ],
        taskCategories: [{ id: 'cat.pvp', shortTag: 'PVP', displayName: 'PvP', colorHex: '#8B5CF6' }],
        tasks: [{ id: 't-beta', title: 'Đấu trường Titan Độc lập', categoryId: 'cat.pvp', workflowId: 'wf-beta-rt', defaultEnabled: true, priority: 1, estimateSec: 15, settingsSchema: [] }],
        utilities: [],
        initialRuntimeVars: { __uiCompactDensity: 'comfortable', __uiCollapsedSections: [], __uiLogMaxHeightPx: 180 }
      };
      await inv('studio_save_ui_schema', { projectId: p2.id, schema: schema2 });
      await inv('studio_ensure_project_icon', { projectId: p2.id, accentHex: '#8B5CF6', overwrite: true });
      const exp2 = await inv('studio_builder_export_release', { projectId: p2.id });

      return { p1Id: p1.id, p2Id: p2.id, alphaExe: exp1.release.exePath, betaExe: exp2.release.exePath };
    })()`);

    alphaExePath = seeded.alphaExe;
    betaExePath = seeded.betaExe;

    // Reload Studio to select p1 in Mini Tool Builder and capture screenshot for Test G
    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1200));
    await evalJs(`(() => {
      const sel = Array.from(document.querySelectorAll('select')).find(s => Array.from(s.options).some(o => o.value === '${seeded.p1Id}'));
      if (sel) {
        const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set;
        setter.call(sel, '${seeded.p1Id}');
        sel.dispatchEvent(new Event('change', { bubbles: true }));
      }
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Mini Tool Builder'));
      if (btn) btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 700));
    await snap('p5_1_studio_builder.png', 1600, 900);
    ws.close();
  } finally {
    try {
      studioChild.kill('SIGTERM');
    } catch (_) {}
  }

  // Explicitly ensure FarmerPro Studio is terminated before running standalone Mini Tool!
  await new Promise((r) => setTimeout(r, 1000));

  // TEST A + B + F: Open Auto_RealtimeAlpha.exe (Case 1 GUI Realtime Stream + Case 2 Cross-Process WAL + Recovery + Secret Scan)
  let testAReport = {};
  let testBReport = {};
  let testCReport = {};
  let testFReport = {};
  let alphaConsoleErrors = [];

  const alphaDir = path.dirname(alphaExePath);
  const alphaChild = spawn(alphaExePath, [], {
    cwd: alphaDir,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9261',
    },
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9261, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    alphaConsoleErrors = consoleErrors;

    // Step 1: Verify boot state shows 'Chưa xác minh' & fail-closed on Live ADB click
    const beforeBoot = await evalJs(`(async () => {
      const tds = Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent);
      let failClosedErr = '';
      try {
        await window.__TAURI_INTERNALS__.invoke('mini_run_task', { taskId: 't-alpha', useDemoMock: false });
      } catch (e) {
        failClosedErr = String(e);
      }
      return { cells: tds, failClosedErr };
    })()`);
    await snap('p5_1_minitool_before.png', 740, 500);

    // Step 2 (TEST A - Case 1 In-process Realtime): Click TestKit start button inside open EXE!
    await evalJs(`document.getElementById('btn-run-testkit').click()`);

    // Poll mid-flight for Step 1 (`Sector-B7` & `100` while task is STILL `Đang làm` BEFORE wait #1 expires!)
    let step1Snap = null;
    const t0 = Date.now();
    while (Date.now() - t0 < 8000) {
      const st = await evalJs(`(() => {
        const zone = document.querySelector('#device-tbody td[data-var-key="farm_zone"]')?.textContent || '';
        const res = document.querySelector('#device-tbody td[data-var-key="resource_count"]')?.textContent || '';
        const pill = document.querySelector('.status-pill')?.textContent || '';
        return { zone, res, pill, seqs: window.__fpRuntimeTestApi.getStateSummary().streamSeqs };
      })()`);
      if (st.zone.includes('Sector-B7') && st.res.includes('100')) {
        step1Snap = st;
        await snap('p5_1_minitool_midflight_step1.png', 740, 500);
        break;
      }
      await new Promise((r) => setTimeout(r, 150));
    }

    // Poll mid-flight for Step 2 (`250` while wait #2 or right before/at completion — WITHOUT restarting EXE!)
    let step2Snap = null;
    const t1 = Date.now();
    while (Date.now() - t1 < 9000) {
      const st = await evalJs(`(() => {
        const res = document.querySelector('#device-tbody td[data-var-key="resource_count"]')?.textContent || '';
        const zone = document.querySelector('#device-tbody td[data-var-key="farm_zone"]')?.textContent || '';
        const pill = document.querySelector('.status-pill')?.textContent || '';
        return { zone, res, pill, seqs: window.__fpRuntimeTestApi.getStateSummary().streamSeqs };
      })()`);
      if (st.res.includes('250')) {
        step2Snap = st;
        await snap('p5_1_minitool_midflight_step2.png', 740, 500);
        break;
      }
      await new Promise((r) => setTimeout(r, 180));
    }

    // Wait for workflow completion (`task_completed_count = 1`)
    let postTask1 = null;
    const t2 = Date.now();
    while (Date.now() - t2 < 8000) {
      const st = await evalJs(`(() => {
        const done = document.querySelector('#device-tbody td[data-var-key="task_completed_count"]')?.textContent || '';
        const logs = document.getElementById('right-log')?.innerText || '';
        return { done, logs, summary: window.__fpRuntimeTestApi.getStateSummary() };
      })()`);
      if (st.done.includes('1')) {
        postTask1 = st;
        break;
      }
      await new Promise((r) => setTimeout(r, 200));
    }

    // Step 3 (TEST A - Case 2 Cross-Process SQLite WAL Watch): Keep GUI open, run EXTERNAL CLI `--run-task t-alpha --mock`!
    execFileSync(alphaExePath, ['--run-task', 't-alpha', '--mock'], {
      cwd: alphaDir,
      encoding: 'utf8',
    });

    // Wait for already-open GUI WAL watcher to auto-emit & show `500` (100+150 -> wait -> 250 -> wait -> CLI incremented from 250+150 or set 100+150 = 250, completed=2!)
    // Note: workflow starts with `state.set_var resource_count=100` then `+150` = `250`, and `task_completed_count` becomes `2`!
    let crossProcWalSeen = null;
    const t3 = Date.now();
    while (Date.now() - t3 < 6000) {
      const st = await evalJs(`(() => {
        const done = document.querySelector('#device-tbody td[data-var-key="task_completed_count"]')?.textContent || '';
        const res = document.querySelector('#device-tbody td[data-var-key="resource_count"]')?.textContent || '';
        return { done, res, summary: window.__fpRuntimeTestApi.getStateSummary() };
      })()`);
      if (st.done.includes('2')) {
        crossProcWalSeen = st;
        await snap('p5_1_minitool_crossproc_wal.png', 740, 500);
        break;
      }
      await new Promise((r) => setTimeout(r, 200));
    }

    // Step 4 (TEST B - Duplicate, Out-of-Order, Missing Seq Gap & Focus Reconnect Recovery):
    testBReport = await evalJs(`(async () => {
      const sBefore = window.__fpRuntimeTestApi.getStateSummary();
      const sid = Object.keys(sBefore.streamSeqs).find(k => k.endsWith('::testkit')) || Object.keys(sBefore.streamSeqs)[0];
      const latestSeq = sBefore.streamSeqs[sid] || 1;

      // 1) Duplicate seq injection (<= curSeq) -> must be ignored!
      await window.__fpRuntimeTestApi.handleIncomingDelta({
        streamId: sid,
        scopeKey: sid,
        projectId: 'p1',
        sessionId: 'dup-sess',
        jobId: 'dup-job',
        seq: latestSeq,
        workflowVersion: 'wf@v1',
        emittedAtUtc: new Date().toISOString(),
        deltas: [{
          projectId: 'p1', sessionId: 'dup', jobId: 'dup', name: 'farm_zone',
          value: 'DUP-POISON', valueType: 'string', source: 'TESTKIT_MOCK',
          verified: false, updatedAtUtc: new Date().toISOString(), workflowVersion: 'wf@v1',
          seq: latestSeq, redacted: false
        }]
      });

      // 2) Dropped event / skipped sequence jump (latestSeq + 5) -> must trigger gap recovery!
      await window.__fpRuntimeTestApi.handleIncomingDelta({
        streamId: sid,
        scopeKey: sid,
        projectId: 'p1',
        sessionId: 'gap-sess',
        jobId: 'gap-job',
        seq: latestSeq + 5,
        workflowVersion: 'wf@v1',
        emittedAtUtc: new Date().toISOString(),
        deltas: []
      });

      // 3) Simulate WebView focus reconnect sync
      window.dispatchEvent(new Event('focus'));
      await new Promise(r => setTimeout(r, 350));

      const sAfter = window.__fpRuntimeTestApi.getStateSummary();
      const zoneAfter = document.querySelector('#device-tbody td[data-var-key="farm_zone"]')?.textContent || '';
      return {
        streamId: sid,
        latestSeq,
        duplicatesIgnored: sAfter.seqStats.duplicatesIgnored,
        gapsRecovered: sAfter.seqStats.gapsRecovered,
        reconnectSyncs: sAfter.seqStats.reconnectSyncs,
        zoneStillClean: zoneAfter.includes('Sector-B7') && !zoneAfter.includes('DUP-POISON'),
        badgeShowsTestkit: zoneAfter.includes('TESTKIT')
      };
    })()`);

    // Step 5 (TEST F - Runtime Logs & Tauri State Secret Check):
    const uiLogsAndState = await evalJs(`(() => {
      const logTxt = document.getElementById('right-log')?.innerText || '';
      const bodyTxt = document.body?.innerText || '';
      const sumTxt = JSON.stringify(window.__fpRuntimeTestApi.getStateSummary());
      return { logTxt, bodyTxt, sumTxt };
    })()`);

    const dbFiles = fs
      .readdirSync(path.join(alphaDir, 'data'))
      .map((f) => path.join(alphaDir, 'data', f));
    const dbRawJoined = dbFiles.map((fp) => fs.readFileSync(fp).toString('latin1')).join('\n');

    testFReport = {
      pinLeakedInUiLogs: uiLogsAndState.logTxt.includes('482910'),
      tokenLeakedInUiLogs: uiLogsAndState.logTxt.includes('tok-secret-999'),
      pinLeakedInTauriState: uiLogsAndState.sumTxt.includes('482910'),
      tokenLeakedInTauriState: uiLogsAndState.sumTxt.includes('tok-secret-999'),
      pinLeakedInSqliteRaw: dbRawJoined.includes('482910'),
      tokenLeakedInSqliteRaw: dbRawJoined.includes('tok-secret-999'),
      hasRedactedMarkers: uiLogsAndState.sumTxt.includes('***REDACTED***') && dbRawJoined.includes('***REDACTED***'),
    };

    testAReport = {
      failClosedBeforeRun: beforeBoot.failClosedErr.includes('NO_LIVE_ADB_DEVICE_ONLINE'),
      step1MidflightSeen: Boolean(step1Snap && step1Snap.zone.includes('Sector-B7') && step1Snap.res.includes('100')),
      step1Detail: step1Snap,
      step2MidflightSeen: Boolean(step2Snap && step2Snap.res.includes('250')),
      step2Detail: step2Snap,
      postTask1Completed: Boolean(postTask1),
      crossProcessWalSeen: Boolean(crossProcWalSeen && crossProcWalSeen.done.includes('2')),
      crossProcessDetail: crossProcWalSeen ? { done: crossProcWalSeen.done, res: crossProcWalSeen.res } : null,
    };

    testCReport = {
      testkitBadgeRenderedInUi: testBReport.badgeShowsTestkit,
      liveVerifiedNotClaimedOnMock: !uiLogsAndState.bodyTxt.includes('LIVE ✓'),
    };

    ws.close();
  } finally {
    try {
      alphaChild.kill('SIGTERM');
    } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 800));

  // TEST G Step 9: Reopen Auto_RealtimeAlpha.exe to prove crash/restart SQLite recovery
  let restartRecovery = {};
  const alphaChild2 = spawn(alphaExePath, [], {
    cwd: alphaDir,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9262',
    },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9262, 20000);
    const { ws, evalJs, snap } = await connectCdp(page);
    await new Promise((r) => setTimeout(r, 600));
    restartRecovery = await evalJs(`(() => {
      const cells = Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent);
      return {
        cells,
        hasSectorB7Testkit: cells.some(c => c.includes('Sector-B7') && c.includes('TESTKIT')),
        hasResource250Testkit: cells.some(c => c.includes('250') && c.includes('TESTKIT')),
        hasCompleted2Testkit: cells.some(c => c.includes('2') && c.includes('TESTKIT'))
      };
    })()`);
    await snap('p5_1_minitool_restarted_recovery.png', 740, 500);
    ws.close();
  } finally {
    try {
      alphaChild2.kill('SIGTERM');
    } catch (_) {}
  }

  // TEST D (Cross-Project Isolation): Open Auto_TitanBeta.exe -> verify zero Alpha data -> run Beta -> check Alpha unaffected!
  let testDReport = {};
  let betaConsoleErrors = [];
  const betaDir = path.dirname(betaExePath);
  const betaChild = spawn(betaExePath, [], {
    cwd: betaDir,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9263',
    },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9263, 20000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    betaConsoleErrors = consoleErrors;
    await new Promise((r) => setTimeout(r, 500));
    const initialBeta = await evalJs(`Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent)`);

    // TEST C Showcase on clean Beta before running TestKit: LIVE_ADB_OCR (conf 0.94, unverified) + LEGACY_UNVERIFIED
    const provenanceLabelsProbe = await evalJs(`(async () => {
      await window.__fpRuntimeTestApi.handleIncomingDelta({
        streamId: 'mini:titan-beta',
        scopeKey: 'mini:titan-beta',
        projectId: 'p2',
        sessionId: 'sess-ocr-94',
        jobId: 'job-ocr-94',
        seq: 1,
        workflowVersion: 'wf@v1',
        emittedAtUtc: new Date().toISOString(),
        deltas: [
          {
            projectId: 'p2', sessionId: 'sess-ocr-94', jobId: 'job-ocr-94',
            name: 'farm_zone', value: 'OCR-Raw-94', valueType: 'string',
            source: 'LIVE_ADB_OCR', verified: false, confidence: 0.94,
            updatedAtUtc: new Date().toISOString(), workflowVersion: 'wf@v1',
            seq: 1, redacted: false
          },
          {
            projectId: 'p2', sessionId: 'legacy-p4', jobId: 'legacy-p4',
            name: 'resource_count', value: 777, valueType: 'number',
            source: 'LEGACY_UNVERIFIED', verified: false, confidence: null,
            updatedAtUtc: new Date().toISOString(), workflowVersion: 'legacy',
            seq: 1, redacted: false
          }
        ]
      });
      const zoneCell = document.querySelector('#device-tbody td[data-var-key="farm_zone"]')?.textContent || '';
      const resCell = document.querySelector('#device-tbody td[data-var-key="resource_count"]')?.textContent || '';
      return {
        zoneCell,
        resCell,
        liveUnverifiedLabelCorrect: zoneCell.includes('LIVE · Chưa xác minh') && zoneCell.includes('94%') && !zoneCell.includes('LIVE ✓'),
        legacyLabelCorrect: resCell.includes('LEGACY · Chưa xác minh')
      };
    })()`);
    testCReport.liveUnverifiedAndLegacyLabels = provenanceLabelsProbe;
    await snap('p5_1_minitool_provenance_variants.png', 900, 540);

    await evalJs(`document.getElementById('btn-run-testkit').click()`);
    await new Promise((r) => setTimeout(r, 1500));
    const afterBeta = await evalJs(`Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent)`);
    await snap('p5_1_minitool_titan_isolated.png', 900, 540);
    ws.close();

    testDReport = {
      initialZeroLeakFromAlpha: initialBeta.filter(c => c === 'Chưa xác minh').length === 2,
      initialCells: initialBeta,
      afterBetaOwnVars: afterBeta,
      hasTitanRidge: afterBeta.some(c => c.includes('Titan-Ridge') && c.includes('TESTKIT')),
      has999: afterBeta.some(c => c.includes('999') && c.includes('TESTKIT')),
    };
  } finally {
    try {
      betaChild.kill('SIGTERM');
    } catch (_) {}
  }

  const report = {
    executedAtUtc: new Date().toISOString(),
    gitHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
    adbLiveHardware: {
      adbDevicesOutput: adbOut || '(0 devices)',
      ldplayer14Vm0IsRunning: ldRunning,
      status: 'BLOCKED (0 ADB online)',
    },
    consoleErrors: {
      studio: studioConsoleErrors,
      alphaMiniTool: alphaConsoleErrors,
      betaMiniTool: betaConsoleErrors,
    },
    acceptanceTests: {
      testA_liveUiUpdateTestkit: testAReport,
      testB_eventLossDuplicateReconnectRecovery: testBReport,
      testC_dataProvenanceDistinction: testCReport,
      testD_crossProjectIsolation: testDReport,
      testF_secretProtectionRuntime: testFReport,
      testG_studioEndToEndAndRestartRecovery: restartRecovery,
    },
  };

  fs.writeFileSync(path.join(EV_DIR, 'p5_1_e2e_report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(JSON.stringify(report, null, 2));

  // Hard assertion checks
  if (
    !testAReport.step1MidflightSeen ||
    !testAReport.step2MidflightSeen ||
    !testAReport.crossProcessWalSeen ||
    !testBReport.zoneStillClean ||
    testBReport.duplicatesIgnored < 1 ||
    testBReport.gapsRecovered < 1 ||
    !testDReport.initialZeroLeakFromAlpha ||
    !testDReport.hasTitanRidge ||
    !testCReport.liveUnverifiedAndLegacyLabels?.liveUnverifiedLabelCorrect ||
    !testCReport.liveUnverifiedAndLegacyLabels?.legacyLabelCorrect ||
    testFReport.pinLeakedInUiLogs ||
    testFReport.tokenLeakedInUiLogs ||
    testFReport.pinLeakedInSqliteRaw ||
    testFReport.tokenLeakedInSqliteRaw ||
    !testFReport.hasRedactedMarkers ||
    !restartRecovery.hasSectorB7Testkit
  ) {
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
