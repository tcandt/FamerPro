const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p5_2_evidence');
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

function runCommandChecked(exe, args, cwd) {
  return execFileSync(exe, args, { cwd, encoding: 'utf8' }).trim();
}

function runCommandExpectFail(exe, args, cwd) {
  let out = '';
  try {
    out = execFileSync(exe, args, { cwd, encoding: 'utf8', stdio: 'pipe' });
  } catch (e) {
    return `${e.stdout || ''} ${e.stderr || ''} ${e.message || ''}`.trim();
  }
  throw new Error(`Expected command to fail, but succeeded: ${out}`);
}

async function spawnPausedAndKill(updaterExe, args, expectedPauseMarker, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(updaterExe, args, { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
    let outBuf = '';
    let killed = false;
    const timer = setTimeout(() => {
      if (!killed) {
        try { child.kill('SIGKILL'); } catch (_) {}
        reject(new Error(`Timed out waiting for ${expectedPauseMarker}. stdout=${outBuf}`));
      }
    }, 15000);

    child.stdout.on('data', (chunk) => {
      outBuf += chunk.toString('utf8');
      if (!killed && outBuf.includes(expectedPauseMarker)) {
        killed = true;
        clearTimeout(timer);
        const ok = child.kill('SIGKILL');
        child.on('exit', (code, signal) => {
          resolve({ killed: ok, code, signal: String(signal || 'SIGKILL'), marker: expectedPauseMarker });
        });
      }
    });
    child.stderr.on('data', (chunk) => {
      outBuf += chunk.toString('utf8');
    });
  });
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
    fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(res.data, 'base64'));
    return { filename, width, height };
  };

  for (let i = 0; i < 60; i++) {
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

async function main() {
  fs.mkdirSync(EV_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) fs.rmSync(SANDBOX, { recursive: true, force: true });
  fs.mkdirSync(path.join(SANDBOX, 'data'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'projects'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'installed'), { recursive: true });

  const sandboxStudioExe = path.join(SANDBOX, 'FarmerPro.exe');
  const updaterExe = path.join(SANDBOX, 'fp-updater.exe');
  fs.copyFileSync(path.join(ROOT, 'target', 'debug', 'farmerpro.exe'), sandboxStudioExe);
  fs.copyFileSync(path.join(ROOT, 'target', 'debug', 'fp-updater.exe'), updaterExe);
  copyDirSync(path.join(ROOT, 'bin'), path.join(SANDBOX, 'bin'));
  fs.mkdirSync(path.join(SANDBOX, 'target', 'debug'), { recursive: true });
  fs.copyFileSync(
    path.join(ROOT, 'target', 'debug', 'fp-mini-runtime.exe'),
    path.join(SANDBOX, 'target', 'debug', 'fp-mini-runtime.exe')
  );

  const installAlpha = path.join(SANDBOX, 'installed', 'Auto_AlphaHarvester');
  const installBeta = path.join(SANDBOX, 'installed', 'Auto_BetaArena');
  const { adbOut, ldRunning } = getAdbStatus();

  let studioConsoleErrors = [];
  let seeded = null;

  // STEP 1: Studio creates 2 projects, exports v1.0.0 for both, verifies 5-Dim Compatibility & Draft->RC->Published,
  // installs v1.0.0 into managed versioned install roots, then builds v1.1.0 for Alpha and promotes Draft->RC->Published!
  const studioChild1 = spawn(sandboxStudioExe, [], {
    cwd: SANDBOX,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9264',
    },
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9264, 22000);
    const { ws, evalJs, consoleErrors } = await connectCdp(page);
    studioConsoleErrors.push(...consoleErrors);

    seeded = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const p1 = await inv('studio_create_project', {
        req: {
          slug: 'alpha-harvester',
          name: 'Alpha Harvester Tool',
          gamePackageId: 'com.alpha.harvester',
          version: '1.0.0',
          exeName: 'Auto_AlphaHarvester.exe',
          windowTitle: 'Alpha Harvester v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });

      const wfAlpha1 = {
        schemaVersion: 1,
        workflowId: 'wf-alpha-v1',
        name: 'Alpha Harvest Flow',
        status: 'published',
        graph: {
          nodes: [
            { id: 'n1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'n2', type: 'state.set_var', position: { x: 200, y: 80 }, config: { varKey: 'account_pin', valueType: 'number', value: 482910 } },
            { id: 'n3', type: 'state.set_var', position: { x: 360, y: 80 }, config: { varKey: 'farm_zone', valueType: 'string', value: 'Sector-B7' } },
            { id: 'n4', type: 'state.increment_var', position: { x: 520, y: 80 }, config: { varKey: 'resource_count', delta: 250 } },
            { id: 'n5', type: 'flow.end', position: { x: 680, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'e1', source: 'n1', sourcePort: 'flow', target: 'n2', targetPort: 'flow' },
            { id: 'e2', source: 'n2', sourcePort: 'flow', target: 'n3', targetPort: 'flow' },
            { id: 'e3', source: 'n3', sourcePort: 'flow', target: 'n4', targetPort: 'flow' },
            { id: 'e4', source: 'n4', sourcePort: 'flow', target: 'n5', targetPort: 'flow' }
          ]
        }
      };
      await inv('studio_save_project_workflow', {
        projectId: p1.id, workflowId: 'wf-alpha-v1', name: 'Alpha Harvest Flow',
        category: 'Resource', draftJson: JSON.stringify(wfAlpha1)
      });
      await inv('studio_upsert_project_task', {
        projectId: p1.id,
        task: { id: 't-alpha', title: 'Thu hoạch Sector-B7 (v1.0)', categoryId: 'cat.farm', workflowId: 'wf-alpha-v1', defaultEnabled: true, priority: 1, estimateSec: 12, settingsSchema: [] },
        categoryMeta: { id: 'cat.farm', shortTag: 'FARM', displayName: 'Farm', colorHex: '#1878F5' }
      });

      const schema1 = {
        schemaVersion: 1,
        projectSlug: 'alpha-harvester',
        windowTitle: 'Alpha Harvester v1.0.0',
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
          { id: 'col-pin', label: 'PIN Guard', runtimeVarKey: 'account_pin', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 520, badgeColor: '#F59E0B' }
        ],
        taskCategories: [{ id: 'cat.farm', shortTag: 'FARM', displayName: 'Farm', colorHex: '#1878F5' }],
        tasks: [{ id: 't-alpha', title: 'Thu hoạch Sector-B7 (v1.0)', categoryId: 'cat.farm', workflowId: 'wf-alpha-v1', defaultEnabled: true, priority: 1, estimateSec: 12, settingsSchema: [] }],
        utilities: [],
        initialRuntimeVars: { __uiCompactDensity: 'compact', __uiCollapsedSections: [], __uiLogMaxHeightPx: 145 }
      };
      await inv('studio_save_ui_schema', { projectId: p1.id, schema: schema1 });
      await inv('studio_ensure_project_icon', { projectId: p1.id, accentHex: '#1878F5', overwrite: true });
      const compatA = await inv('studio_check_release_compatibility', { projectId: p1.id });
      const expA1 = await inv('studio_builder_export_release', { projectId: p1.id, outputDir: 'projects/alpha-harvester/releases/v1.0.0' });
      await inv('studio_transition_release_lifecycle', { projectId: p1.id, releaseId: expA1.release.id, targetState: 'RC', note: 'QA Candidate' });
      const pubA1 = await inv('studio_transition_release_lifecycle', { projectId: p1.id, releaseId: expA1.release.id, targetState: 'Published', note: 'Release GA v1.0.0' });

      // Deploy initial v1.0.0 into managed root via Studio IPC
      const depA1 = await inv('studio_deploy_minitool_release', {
        installRoot: ${JSON.stringify(installAlpha)},
        releaseDir: expA1.release.outputDirPath
      });

      // Upgrade project metadata & build v1.1.0 release artifact (does not auto-deploy yet!)
      await inv('studio_update_project', {
        req: { id: p1.id, version: '1.1.0', windowTitle: 'Alpha Harvester v1.1.0' }
      });
      await inv('studio_upsert_project_task', {
        projectId: p1.id,
        task: { id: 't-alpha-bonus', title: 'Auto Event Boss Bonus (v1.1.0 New)', categoryId: 'cat.farm', workflowId: 'wf-alpha-v1', defaultEnabled: true, priority: 2, estimateSec: 18, settingsSchema: [] },
        categoryMeta: { id: 'cat.farm', shortTag: 'FARM', displayName: 'Farm', colorHex: '#1878F5' }
      });
      const schema11 = {
        ...schema1,
        windowTitle: 'Alpha Harvester v1.1.0',
        versionBadge: 'v1.1.0',
        themeAccentHex: '#10B981',
        tasks: [
          schema1.tasks[0],
          { id: 't-alpha-bonus', title: 'Auto Event Boss Bonus (v1.1.0 New)', categoryId: 'cat.farm', workflowId: 'wf-alpha-v1', defaultEnabled: true, priority: 2, estimateSec: 18, settingsSchema: [] }
        ]
      };
      await inv('studio_save_ui_schema', { projectId: p1.id, schema: schema11 });
      const expA11 = await inv('studio_builder_export_release', { projectId: p1.id, outputDir: 'projects/alpha-harvester/releases/v1.1.0' });
      await inv('studio_transition_release_lifecycle', { projectId: p1.id, releaseId: expA11.release.id, targetState: 'RC', note: 'P5.2 Upgrade RC' });
      const pubA11 = await inv('studio_transition_release_lifecycle', { projectId: p1.id, releaseId: expA11.release.id, targetState: 'Published', note: 'Promote v1.1.0 GA' });

      // Independent Project B: Beta Arena Tool
      const p2 = await inv('studio_create_project', {
        req: {
          slug: 'beta-arena',
          name: 'Beta Arena Tool',
          gamePackageId: 'com.beta.arena',
          version: '1.0.0',
          exeName: 'Auto_BetaArena.exe',
          windowTitle: 'Beta Arena v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core']
        }
      });
      const wfBeta = {
        schemaVersion: 1,
        workflowId: 'wf-beta-v1',
        name: 'Beta Arena Flow',
        status: 'published',
        graph: {
          nodes: [
            { id: 'b1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'b2', type: 'state.set_var', position: { x: 240, y: 80 }, config: { varKey: 'farm_zone', valueType: 'string', value: 'Beta-Citadel' } },
            { id: 'b3', type: 'flow.end', position: { x: 440, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'be1', source: 'b1', sourcePort: 'flow', target: 'b2', targetPort: 'flow' },
            { id: 'be2', source: 'b2', sourcePort: 'flow', target: 'b3', targetPort: 'flow' }
          ]
        }
      };
      await inv('studio_save_project_workflow', {
        projectId: p2.id, workflowId: 'wf-beta-v1', name: 'Beta Arena Flow',
        category: 'Arena', draftJson: JSON.stringify(wfBeta)
      });
      await inv('studio_upsert_project_task', {
        projectId: p2.id,
        task: { id: 't-beta', title: 'Đấu trường Beta Độc lập', categoryId: 'cat.pvp', workflowId: 'wf-beta-v1', defaultEnabled: true, priority: 1, estimateSec: 10, settingsSchema: [] },
        categoryMeta: { id: 'cat.pvp', shortTag: 'PVP', displayName: 'PvP', colorHex: '#8B5CF6' }
      });
      const schemaB = {
        schemaVersion: 1,
        projectSlug: 'beta-arena',
        windowTitle: 'Beta Arena v1.0.0',
        versionBadge: 'v1.0.0',
        themeAccentHex: '#8B5CF6',
        headerBgHex: '#18142C',
        layoutPreset: 'compact1_col',
        defaultWindowSize: [740, 500],
        sectionsOrder: ['header_bar', 'device_table', 'task_checklist', 'quick_log', 'footer_controls'],
        hiddenComponents: ['utilities_panel'],
        deviceColumns: [
          { id: 'bcol-zone', label: 'Khu vực', runtimeVarKey: 'farm_zone', fallbackDisplay: 'Chưa xác minh', minWindowWidth: 420, badgeColor: '#A78BFA' }
        ],
        taskCategories: [{ id: 'cat.pvp', shortTag: 'PVP', displayName: 'PvP', colorHex: '#8B5CF6' }],
        tasks: [{ id: 't-beta', title: 'Đấu trường Beta Độc lập', categoryId: 'cat.pvp', workflowId: 'wf-beta-v1', defaultEnabled: true, priority: 1, estimateSec: 10, settingsSchema: [] }],
        utilities: [],
        initialRuntimeVars: { __uiCompactDensity: 'compact', __uiCollapsedSections: [], __uiLogMaxHeightPx: 145 }
      };
      await inv('studio_save_ui_schema', { projectId: p2.id, schema: schemaB });
      await inv('studio_ensure_project_icon', { projectId: p2.id, accentHex: '#8B5CF6', overwrite: true });
      const expB1 = await inv('studio_builder_export_release', { projectId: p2.id, outputDir: 'projects/beta-arena/releases/v1.0.0' });
      const depB1 = await inv('studio_deploy_minitool_release', {
        installRoot: ${JSON.stringify(installBeta)},
        releaseDir: expB1.release.outputDirPath
      });

      const relHistoryA = await inv('studio_list_project_releases', { projectId: p1.id });
      const transA = await inv('studio_list_project_release_transitions', { projectId: p1.id });

      return {
        p1Id: p1.id, p2Id: p2.id,
        compatA,
        pubA1State: pubA1.lifecycleState,
        pubA11State: pubA11.lifecycleState,
        depA1, depB1,
        relDirA1: expA1.release.outputDirPath,
        relDirA11: expA11.release.outputDirPath,
        relHistoryA,
        transA
      };
    })()`);

    ws.close();
  } finally {
    try { studioChild1.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 800));

  // STEP 2: Launch installed Alpha v1.0.0 from managed root -> execute TestKit workflow to create persistent SQLite data + redacted secret
  const alphaV1Exe = path.join(installAlpha, 'versions', 'v1.0.0', 'Auto_AlphaHarvester.exe');
  const betaV1Exe = path.join(installBeta, 'versions', 'v1.0.0', 'Auto_BetaArena.exe');
  let v1BeforeSnap = null;
  let alphaV1Errors = [];

  const alphaChild1 = spawn(alphaV1Exe, [], {
    cwd: path.dirname(alphaV1Exe),
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9265',
    },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9265, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page, '#device-tbody td');
    alphaV1Errors = consoleErrors;

    await evalJs(`document.getElementById('btn-run-testkit').click()`);
    const deadline = Date.now() + 8000;
    while (Date.now() < deadline) {
      const cells = await evalJs(`Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent)`);
      if (cells.some((c) => c.includes('Sector-B7')) && cells.some((c) => c.includes('250'))) {
        v1BeforeSnap = {
          cells,
          title: await evalJs(`document.title`),
          badge: await evalJs(`document.querySelector('.badge-ver')?.textContent || ''`),
          taskCount: await evalJs(`document.querySelectorAll('.task-row').length`),
        };
        await snap('p5_2_minitool_v1_before.png', 740, 500);
        break;
      }
      await new Promise((r) => setTimeout(r, 150));
    }
    ws.close();
  } finally {
    try { alphaChild1.kill('SIGTERM'); } catch (_) {}
  }

  // Run Beta v1.0.0 via CLI to seed Beta's own isolated DB cleanly
  runCommandChecked(betaV1Exe, ['--run-task', 't-beta', '--mock'], path.dirname(betaV1Exe));
  await new Promise((r) => setTimeout(r, 700));

  // STEP 3: Updater Safety & Fault Tests against real EXEs!
  // 3A. Running EXE Process Lock Guard (spawn alphaV1Exe headless/GUI & attempt upgrade -> MUST FAIL!)
  const runningLockChild = spawn(alphaV1Exe, [], {
    cwd: path.dirname(alphaV1Exe),
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9268' },
    stdio: 'ignore',
  });
  await new Promise((r) => setTimeout(r, 1200));
  const runningBlockErr = runCommandExpectFail(
    updaterExe,
    ['upgrade', '--install-root', installAlpha, '--release-dir', path.join(SANDBOX, seeded.relDirA11)],
    SANDBOX
  );
  try { runningLockChild.kill('SIGTERM'); } catch (_) {}
  await new Promise((r) => setTimeout(r, 900));

  // 3B. Tampered bundle rejection
  const tamperedDir = path.join(SANDBOX, 'tampered_v110');
  copyDirSync(path.join(SANDBOX, seeded.relDirA11), tamperedDir);
  fs.writeFileSync(path.join(tamperedDir, 'bundle', 'ui-schema.json'), '{"tampered":true}');
  const tamperBlockErr = runCommandExpectFail(
    updaterExe,
    ['upgrade', '--install-root', installAlpha, '--release-dir', tamperedDir],
    SANDBOX
  );

  // 3C. Read-only / non-writable destination rejection
  const blockedRoot = path.join(SANDBOX, 'blocked_readonly_root');
  fs.mkdirSync(blockedRoot, { recursive: true });
  fs.writeFileSync(path.join(blockedRoot, '.updater'), 'file_blocking_updater_dir');
  const permBlockErr = runCommandExpectFail(
    updaterExe,
    ['install-initial', '--install-root', blockedRoot, '--release-dir', path.join(SANDBOX, seeded.relDirA11)],
    SANDBOX
  );

  // 3D. Real OS Forced-Kill (SIGKILL / TerminateProcess) at AfterBackup & AfterStage -> Reboot Reconcile!
  const killAfterBackup = await spawnPausedAndKill(
    updaterExe,
    ['upgrade', '--install-root', installAlpha, '--release-dir', path.join(SANDBOX, seeded.relDirA11), '--pause-at', 'AfterBackup'],
    'PAUSED_AT:AfterBackup',
    SANDBOX
  );
  const recAfterBackup = JSON.parse(
    runCommandChecked(updaterExe, ['reconcile', '--install-root', installAlpha], SANDBOX)
  );

  const killAfterStage = await spawnPausedAndKill(
    updaterExe,
    ['upgrade', '--install-root', installAlpha, '--release-dir', path.join(SANDBOX, seeded.relDirA11), '--pause-at', 'AfterStage'],
    'PAUSED_AT:AfterStage',
    SANDBOX
  );
  const recAfterStage = JSON.parse(
    runCommandChecked(updaterExe, ['reconcile', '--install-root', installAlpha], SANDBOX)
  );

  // 3E. SQLite migration failure rollback
  const migFailErr = runCommandExpectFail(
    updaterExe,
    ['upgrade', '--install-root', installAlpha, '--release-dir', path.join(SANDBOX, seeded.relDirA11), '--simulate-migration-failure'],
    SANDBOX
  );

  // STEP 4: Clean Upgrade v1.0.0 -> v1.1.0 via Studio GUI (capture Studio Release Lifecycle UI!)
  let deployUpgradeResult = null;
  const studioChild2 = spawn(sandboxStudioExe, [], {
    cwd: SANDBOX,
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9264' },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9264, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors.push(...consoleErrors);

    deployUpgradeResult = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      return await inv('studio_deploy_minitool_release', {
        installRoot: ${JSON.stringify(installAlpha)},
        releaseDir: ${JSON.stringify(path.join(SANDBOX, seeded.relDirA11))}
      });
    })()`);

    // Select Alpha project, navigate to Mini Tool Builder, click Check Compatibility & Inspect Install, then capture Studio screenshot!
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
    await new Promise((r) => setTimeout(r, 600));
    await evalJs(`(() => {
      const inp = Array.from(document.querySelectorAll('input')).find(i => i.placeholder && i.placeholder.includes('installed/'));
      if (inp) {
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
        setter.call(inp, ${JSON.stringify(installAlpha)});
        inp.dispatchEvent(new Event('input', { bubbles: true }));
      }
      const compatBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Check SemVer'));
      if (compatBtn) compatBtn.click();
      const inspBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.trim() === 'Inspect');
      if (inspBtn) inspBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 800));
    // Scroll right panel so Compatibility Gate, Rollback & Release Lifecycle are prominently visible
    await evalJs(`(() => {
      const cols = Array.from(document.querySelectorAll('.fp-panel'));
      if (cols[1]) cols[1].scrollTop = 260;
    })()`);
    await snap('p5_2_studio_release_lifecycle.png', 1600, 900);
    ws.close();
  } finally {
    try { studioChild2.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 700));

  // STEP 5: Launch Upgraded Alpha v1.1.0 -> verify v1.1.0 UI + NEW task + ALL v1.0.0 data/secrets preserved!
  const alphaV11Exe = path.join(installAlpha, 'versions', 'v1.1.0', 'Auto_AlphaHarvester.exe');
  let v11UpgradedSnap = null;
  let alphaV11Errors = [];
  const alphaChild2 = spawn(alphaV11Exe, [], {
    cwd: path.dirname(alphaV11Exe),
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9266' },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9266, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page, '#device-tbody td');
    alphaV11Errors = consoleErrors;
    await new Promise((r) => setTimeout(r, 500));
    v11UpgradedSnap = await evalJs(`(() => {
      const cells = Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent);
      const tasks = Array.from(document.querySelectorAll('.task-row')).map(el => el.textContent);
      return {
        title: document.title,
        badge: document.querySelector('.badge-ver')?.textContent || '',
        cells,
        tasks,
        preservedSectorB7: cells.some(c => c.includes('Sector-B7') && c.includes('TESTKIT')),
        preservedResource250: cells.some(c => c.includes('250') && c.includes('TESTKIT')),
        preservedRedactedPin: cells.some(c => c.includes('***REDACTED***') && !c.includes('482910')),
        hasNewV11Task: tasks.some(t => t.includes('Auto Event Boss Bonus'))
      };
    })()`);
    await snap('p5_2_minitool_v1_1_upgraded.png', 740, 500);
    ws.close();
  } finally {
    try { alphaChild2.kill('SIGTERM'); } catch (_) {}
  }

  // Verify independent Beta installation remained untouched at v1.0.0!
  const betaPtr = JSON.parse(fs.readFileSync(path.join(installBeta, 'active_version.json'), 'utf8'));
  const alphaPtrAfterUpgrade = JSON.parse(fs.readFileSync(path.join(installAlpha, 'active_version.json'), 'utf8'));

  await new Promise((r) => setTimeout(r, 700));

  // STEP 6: Binary-only Rollback Alpha from v1.1.0 -> v1.0.0 cleanly (preserving data) and launch v1.0.0 EXE again!
  const rollbackOut = JSON.parse(
    runCommandChecked(updaterExe, ['rollback', '--install-root', installAlpha], SANDBOX)
  );
  let rolledBackSnap = null;
  const alphaChild3 = spawn(alphaV1Exe, [], {
    cwd: path.dirname(alphaV1Exe),
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9267' },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9267, 22000);
    const { ws, evalJs, snap } = await connectCdp(page, '#device-tbody td');
    await new Promise((r) => setTimeout(r, 400));
    rolledBackSnap = await evalJs(`(() => {
      const cells = Array.from(document.querySelectorAll('#device-tbody td')).map(t => t.textContent);
      const tasks = Array.from(document.querySelectorAll('.task-row')).map(el => el.textContent);
      return {
        title: document.title,
        badge: document.querySelector('.badge-ver')?.textContent || '',
        cells,
        taskCount: tasks.length,
        preservedSectorB7: cells.some(c => c.includes('Sector-B7')),
        preservedRedactedPin: cells.some(c => c.includes('***REDACTED***') && !c.includes('482910'))
      };
    })()`);
    await snap('p5_2_minitool_rolled_back.png', 740, 500);
    ws.close();
  } finally {
    try { alphaChild3.kill('SIGTERM'); } catch (_) {}
  }

  const finalAlphaPtr = JSON.parse(fs.readFileSync(path.join(installAlpha, 'active_version.json'), 'utf8'));
  const finalAlphaJournal = JSON.parse(fs.readFileSync(path.join(installAlpha, '.updater', 'update-journal.json'), 'utf8'));

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
      alphaV1: alphaV1Errors,
      alphaV11: alphaV11Errors,
    },
    studioLifecycle: seeded,
    v1BeforeSnap,
    guardsAndCrashRecovery: {
      runningExeLockBlocked: runningBlockErr.includes('EXE_FILE_LOCKED') || runningBlockErr.includes('MINI_TOOL_PROCESS_RUNNING'),
      runningBlockErr,
      tamperBlocked: tamperBlockErr.includes('INTEGRITY_HASH_MISMATCH') || tamperBlockErr.includes('SHA-256'),
      tamperBlockErr,
      readOnlyPermissionBlocked: permBlockErr.includes('INSTALL_DIR_NOT_WRITABLE'),
      permBlockErr,
      killAfterBackup,
      recAfterBackup,
      killAfterStage,
      recAfterStage,
      migrationFailureRolledBack: migFailErr.includes('SIMULATED_MIGRATION_FAILURE'),
      migFailErr,
    },
    upgradeAndPreserve: {
      deployUpgradeResult,
      alphaPtrAfterUpgrade,
      v11UpgradedSnap,
      betaPtrUntouched: betaPtr,
    },
    rollbackVerification: {
      rollbackOut,
      rolledBackSnap,
      finalAlphaPtr,
      finalAlphaJournal,
    },
  };

  fs.writeFileSync(path.join(EV_DIR, 'p5_2_e2e_report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(JSON.stringify(report, null, 2));

  if (
    !report.guardsAndCrashRecovery.runningExeLockBlocked ||
    !report.guardsAndCrashRecovery.tamperBlocked ||
    !report.guardsAndCrashRecovery.readOnlyPermissionBlocked ||
    recAfterBackup.status !== 'RECOVERED_TO_PREVIOUS' ||
    recAfterStage.status !== 'RECOVERED_TO_PREVIOUS' ||
    !report.guardsAndCrashRecovery.migrationFailureRolledBack ||
    alphaPtrAfterUpgrade.activeVersion !== '1.1.0' ||
    !v11UpgradedSnap?.preservedSectorB7 ||
    !v11UpgradedSnap?.preservedResource250 ||
    !v11UpgradedSnap?.preservedRedactedPin ||
    !v11UpgradedSnap?.hasNewV11Task ||
    betaPtr.activeVersion !== '1.0.0' ||
    finalAlphaPtr.activeVersion !== '1.0.0' ||
    !rolledBackSnap?.preservedSectorB7
  ) {
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
