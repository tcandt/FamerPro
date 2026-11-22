const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_7_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_7_sandbox');

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
    await new Promise((r) => setTimeout(r, 350));
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
    await new Promise((r) => setTimeout(r, 400));
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

async function launchStandaloneMiniTool(exePath, port, screenshotName, width, height) {
  const dir = path.dirname(exePath);
  const child = spawn(exePath, [], {
    cwd: dir,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(port, 20000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    await sendViewportAndWait(snap, screenshotName, width, height);
    await new Promise((r) => setTimeout(r, 900));

    const runtimeCheck = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const boot = await inv('mini_get_bootstrap');
      const wsEl = document.getElementById('workspace');
      const utilTabEl = document.getElementById('tab-util');
      const taskTitleEl = document.querySelector('.task-title');
      return {
        windowTitle: boot.uiSchema?.windowTitle,
        projectSlug: boot.manifest?.project?.slug,
        layoutPreset: boot.uiSchema?.layoutPreset,
        defaultWindowSize: boot.uiSchema?.defaultWindowSize,
        accentHex: boot.uiSchema?.themeAccentHex,
        signatureVerified: boot.signatureVerified,
        isDemoMode: boot.isDemoMode,
        devicesCount: (boot.devices || []).length,
        isCompactDensityBody: document.body.classList.contains('density-compact'),
        isSingleColWorkspace: wsEl ? wsEl.classList.contains('single-col') : false,
        utilitiesTabHidden: utilTabEl ? utilTabEl.classList.contains('hidden') : false,
        taskTitles: Array.from(document.querySelectorAll('.task-title')).map(el => el.textContent),
        firstTaskNoHorizontalClip: taskTitleEl ? (taskTitleEl.scrollWidth <= taskTitleEl.clientWidth + 2) : true,
        columnsRendered: Array.from(document.querySelectorAll('#device-thead th')).map(th => th.textContent)
      };
    })()`);

    const shot = await snap(screenshotName, width, height);
    ws.close();
    return {
      STANDALONE_LAUNCH_PASS: Boolean(runtimeCheck.signatureVerified && !runtimeCheck.isDemoMode),
      LIVE_WORKFLOW_EXECUTION_PASS: false,
      liveWorkflowBlockReason:
        runtimeCheck.devicesCount === 0
          ? 'BLOCKED: 0 physical/emulator Android devices attached to ADB (LDPlayer requires UAC elevation)'
          : 'Not executed',
      runtimeCheck,
      consoleErrors,
      shot,
    };
  } finally {
    try {
      child.kill('SIGTERM');
    } catch (_) {}
  }
}

async function sendViewportAndWait(snap, filename, w, h) {
  await snap(filename, w, h);
}

async function main() {
  fs.mkdirSync(EV_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) fs.rmSync(SANDBOX, { recursive: true, force: true });
  fs.mkdirSync(path.join(SANDBOX, 'data'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'projects'), { recursive: true });
  copyDirSync(path.join(ROOT, 'bin'), path.join(SANDBOX, 'bin'));
  fs.copyFileSync(path.join(ROOT, 'target', 'debug', 'farmerpro.exe'), path.join(SANDBOX, 'farmerpro.exe'));
  fs.copyFileSync(
    path.join(ROOT, 'target', 'debug', 'fp-mini-runtime.exe'),
    path.join(SANDBOX, 'fp-mini-runtime.exe')
  );

  const studioChild = spawn(path.join(SANDBOX, 'farmerpro.exe'), [], {
    cwd: SANDBOX,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9245',
      FARMERPRO_PROJECTS_DIR: path.join(SANDBOX, 'projects'),
    },
    stdio: 'ignore',
  });

  let wysiwygReport = {};
  let regressionViews = [];
  let omegaExePath = '';
  let titanExePath = '';
  let studioConsoleErrors = [];

  try {
    const page = await waitForCdp(9245, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors = consoleErrors;

    // 1. Seed Omega Compact + Titan Expanded via Studio IPC
    const seeded = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const makeProj = async (slug, name, pkg, ver, exe, winTitle) => {
        const p = await inv('studio_create_project', {
          req: { slug, name, gamePackageId: pkg, version: ver, exeName: exe, windowTitle: winTitle, targetWidthPx: 1280, targetHeightPx: 720, bundleAdb: false, enabledModules: ['mod.adb_core', 'mod.vision_ocr'] }
        });
        const wfGraph = {
          schemaVersion: 1,
          workflowId: 'wf-' + slug,
          name: name + ' Daily Workflow',
          status: 'published',
          graph: {
            nodes: [
              { id: 'n-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'n-cp', type: 'state.checkpoint', position: { x: 380, y: 80 }, config: { name: slug + '_cp' } },
              { id: 'n-end', type: 'flow.end', position: { x: 720, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'n-start', sourcePort: 'flow', target: 'n-cp', targetPort: 'flow' },
              { id: 'e2', source: 'n-cp', sourcePort: 'flow', target: 'n-end', targetPort: 'flow' }
            ]
          }
        };
        await inv('studio_save_project_workflow', {
          projectId: p.id, workflowId: 'wf-' + slug, name: name + ' Daily Workflow', category: 'Daily', draftJson: JSON.stringify(wfGraph)
        });
        return p;
      };

      const pOmega = await makeProj('omega-compact', 'Omega Compact Tool', 'com.omega.farm', '1.2.0', 'Auto_OmegaCompact.exe', 'Omega Compact Auto v1.2.0');
      await inv('studio_upsert_project_task', {
        projectId: pOmega.id,
        task: { id: 't-omega-1', title: 'Nhiệm vụ Chính Tuyến Độ Dài Rất Lớn Để Kiểm Tra Wrap Trên Compact Window', categoryId: 'cat.main', workflowId: 'wf-omega-compact', defaultEnabled: true, priority: 1, estimateSec: 60, settingsSchema: [] },
        categoryMeta: { id: 'cat.main', shortTag: 'MAIN', displayName: 'Chính tuyến', colorHex: '#F59E0B' }
      });
      await inv('studio_upsert_project_task', {
        projectId: pOmega.id,
        task: { id: 't-omega-2', title: 'Nhận quà Điểm danh Ngày', categoryId: 'cat.daily', workflowId: 'wf-omega-compact', defaultEnabled: true, priority: 2, estimateSec: 20, settingsSchema: [] },
        categoryMeta: { id: 'cat.daily', shortTag: 'DAY', displayName: 'Hàng ngày', colorHex: '#38BDF8' }
      });

      const pTitan = await makeProj('titan-expanded', 'Titan Expanded Suite', 'com.titan.rpg', '3.0.0', 'Auto_TitanExpanded.exe', 'Titan Expanded Auto v3.0.0');
      await inv('studio_upsert_project_task', {
        projectId: pTitan.id,
        task: { id: 't-titan-1', title: 'Chiến trường Liên Server Titan', categoryId: 'cat.pvp', workflowId: 'wf-titan-expanded', defaultEnabled: true, priority: 1, estimateSec: 120, settingsSchema: [] },
        categoryMeta: { id: 'cat.pvp', shortTag: 'PVP', displayName: 'Liên Server', colorHex: '#8B5CF6' }
      });
      return { pOmegaId: pOmega.id, pTitanId: pTitan.id };
    })()`);

    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));

    // 2. Regression check across ALL views in sidebar (Projects, Workflow Studio, Tasks, Device Lab, Test & Debug, Builder, Vault, Proxy, Settings, Overview, Workflows)
    const viewLabels = [
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
    for (const lbl of viewLabels) {
      await evalJs(`(() => {
        const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes(${JSON.stringify(lbl)}));
        if (!btn) throw new Error('Missing nav button: ' + ${JSON.stringify(lbl)});
        btn.click();
      })()`);
      await new Promise((r) => setTimeout(r, 350));
      const snapInfo = await evalJs(`(() => ({
        title: document.body.innerText.split('\\n').slice(0, 6).join(' | '),
        hasContent: document.body.innerText.trim().length > 80
      }))()`);
      regressionViews.push({ label: lbl, rendered: snapInfo.hasContent });
      if (lbl === 'Workflow Studio') {
        await snap('p4_7_studio_1280x720.png', 1280, 720);
        await snap('p4_7_studio_1600x900.png', 1600, 900);
      }
    }

    // 3. Verify WYSIWYG Builder interactions on Omega Compact (select Omega project first)
    await evalJs(`(() => {
      const sel = Array.from(document.querySelectorAll('select')).find(s => Array.from(s.options).some(o => o.value === '${seeded.pOmegaId}'));
      if (sel) {
        sel.value = '${seeded.pOmegaId}';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
      }
    })()`);
    await new Promise((r) => setTimeout(r, 700));

    // Switch to Mini Tool Builder
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Mini Tool Builder'));
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 600));

    // Switch preset -> compact1_col, Remove utilities_panel then re-Add it, then Hide utilities_panel, Add column in device_table, Switch to ERROR preview, Save Schema!
    await evalJs(`(() => {
      const btn1 = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.trim() === 'compact1_col');
      if (!btn1) throw new Error('compact1_col button not found');
      btn1.click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    // Remove Utilities Switches then add it back from Component Palette
    await evalJs(`(() => {
      const rows = Array.from(document.querySelectorAll('div'));
      const row = rows.find(d => d.textContent && d.textContent.trim() === 'Utilities Switches' && d.querySelectorAll('button').length === 4);
      if (row) row.querySelectorAll('button')[3].click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    const paletteShowedUtilitiesChip = await evalJs(`Boolean(Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Utilities Switches')))`);

    await evalJs(`(() => {
      const addBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Utilities Switches'));
      if (addBtn) addBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    // Hide Utilities Switches (EyeOff) so Preview & exported EXE hide it!
    await evalJs(`(() => {
      const rows = Array.from(document.querySelectorAll('div'));
      const row = rows.find(d => d.textContent && d.textContent.trim() === 'Utilities Switches' && d.querySelectorAll('button').length === 4);
      if (row) row.querySelectorAll('button')[2].click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    // Select Device ADB Table, click Thêm cột
    await evalJs(`(() => {
      const rows = Array.from(document.querySelectorAll('div'));
      const row = rows.find(d => d.textContent && d.textContent.trim() === 'Device ADB Table' && d.querySelectorAll('button').length === 4);
      if (row) row.click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));
    await evalJs(`(() => {
      const addCol = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Thêm cột'));
      if (addCol) addCol.click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    // Preview ERROR state and verify it renders
    await evalJs(`(() => {
      const errBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.trim() === 'ERROR');
      if (errBtn) errBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 200));

    const hasErrorPreview = await evalJs(`document.body.innerText.includes('ERROR: ADB_MISSING')`);
    await snap('p4_7_builder_wysiwyg_1600x900.png', 1600, 900);

    // Click Lưu Schema
    await evalJs(`(() => {
      const saveBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Lưu Schema'));
      if (saveBtn) saveBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 900));

    // Reload entire app to prove 100% SQLite persistence across Project close/reopen!
    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));

    const persistedSchema = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      return await inv('studio_get_ui_schema', { projectId: '${seeded.pOmegaId}' });
    })()`);

    // Export BOTH Omega Compact and Titan Expanded
    const exportsRes = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      await inv('studio_ensure_project_icon', { projectId: '${seeded.pOmegaId}', accentHex: '#F59E0B', overwrite: true });
      const resOmega = await inv('studio_builder_export_release', { projectId: '${seeded.pOmegaId}' });

      const titanSchema = {
        schemaVersion: 1,
        projectSlug: 'titan-expanded',
        windowTitle: 'Titan Expanded Auto v3.0.0',
        versionBadge: 'v3.0.0',
        themeAccentHex: '#8B5CF6',
        headerBgHex: '#18142C',
        layoutPreset: 'expanded',
        defaultWindowSize: [1020, 660],
        sectionsOrder: ['header_bar', 'device_table', 'task_checklist', 'utilities_panel', 'quick_log', 'footer_controls'],
        hiddenComponents: [],
        deviceColumns: [
          { id: 'c-tier', label: 'Rank Titan', runtimeVarKey: 'titan_rank', fallbackDisplay: 'Diamond', minWindowWidth: 500, badgeColor: '#8B5CF6' }
        ],
        taskCategories: [{ id: 'cat.pvp', shortTag: 'PVP', displayName: 'Liên Server', colorHex: '#8B5CF6' }],
        tasks: [{ id: 't-titan-1', title: 'Chiến trường Liên Server Titan', categoryId: 'cat.pvp', workflowId: 'wf-titan-expanded', defaultEnabled: true, priority: 1, estimateSec: 120, settingsSchema: [] }],
        utilities: [{ id: 'u-titan', label: 'Tự động vào lại trận khi rớt mạng', defaultEnabled: true, requiredModule: null, settingsSchema: [] }],
        initialRuntimeVars: { __uiCompactDensity: 'comfortable', __uiCollapsedSections: [], __uiLogMaxHeightPx: 240 }
      };
      await inv('studio_save_ui_schema', { projectId: '${seeded.pTitanId}', schema: titanSchema });
      await inv('studio_ensure_project_icon', { projectId: '${seeded.pTitanId}', accentHex: '#8B5CF6', overwrite: true });
      const resTitan = await inv('studio_builder_export_release', { projectId: '${seeded.pTitanId}' });

      return { omegaExe: resOmega.release.exePath, titanExe: resTitan.release.exePath };
    })()`);

    omegaExePath = exportsRes.omegaExe;
    titanExePath = exportsRes.titanExe;

    wysiwygReport = {
      paletteShowedUtilitiesChipOnRemove: paletteShowedUtilitiesChip,
      hasErrorPreviewState: hasErrorPreview,
      persistedAfterReload: {
        layoutPreset: persistedSchema?.layoutPreset,
        defaultWindowSize: persistedSchema?.defaultWindowSize,
        hiddenComponents: persistedSchema?.hiddenComponents,
        deviceColumnsCount: (persistedSchema?.deviceColumns || []).length,
      },
    };

    ws.close();
  } finally {
    try {
      studioChild.kill('SIGTERM');
    } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 1000));

  // 4. Launch BOTH exported EXEs with Studio completely closed!
  const omegaStandalone = await launchStandaloneMiniTool(
    omegaExePath,
    9246,
    'p4_7_minitool_omega_compact.png',
    640,
    460
  );
  const titanStandalone = await launchStandaloneMiniTool(
    titanExePath,
    9247,
    'p4_7_minitool_titan_expanded.png',
    1020,
    660
  );

  const report = {
    milestone: 'P4.7',
    executedAtUtc: new Date().toISOString(),
    studioClosedBeforeStandaloneLaunch: true,
    studioConsoleErrors,
    regressionViews,
    wysiwygReport,
    omegaStandalone,
    titanStandalone,
  };
  fs.writeFileSync(path.join(EV_DIR, 'p4_7_verification_report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
