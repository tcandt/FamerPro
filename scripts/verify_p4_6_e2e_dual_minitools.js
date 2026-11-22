const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_6_sandbox');

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

async function waitForCdp(port, timeoutMs = 20000) {
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

async function seedAndBuildProject(evalJs, cfg) {
  return await evalJs(`(async () => {
    const inv = window.__TAURI_INTERNALS__.invoke;
    const proj = await inv('studio_create_project', {
      req: {
        slug: ${JSON.stringify(cfg.slug)},
        name: ${JSON.stringify(cfg.name)},
        gamePackageId: ${JSON.stringify(cfg.packageId)},
        version: ${JSON.stringify(cfg.version)},
        exeName: ${JSON.stringify(cfg.exeName)},
        windowTitle: ${JSON.stringify(cfg.windowTitle)},
        targetWidthPx: 1280,
        targetHeightPx: 720,
        bundleAdb: false,
        enabledModules: ${JSON.stringify(cfg.modules)}
      }
    });

    const wfGraph = {
      schemaVersion: 1,
      workflowId: ${JSON.stringify(cfg.workflowId)},
      name: ${JSON.stringify(cfg.workflowName)},
      status: 'published',
      graph: {
        nodes: [
          { id: 'n-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
          { id: 'n-cp', type: 'state.checkpoint', position: { x: 240, y: 80 }, config: { name: ${JSON.stringify(cfg.checkpointName)} } },
          { id: 'n-end', type: 'flow.end', position: { x: 440, y: 80 }, config: {} }
        ],
        edges: [
          { id: 'e1', source: 'n-start', sourcePort: 'flow', target: 'n-cp', targetPort: 'flow' },
          { id: 'e2', source: 'n-cp', sourcePort: 'flow', target: 'n-end', targetPort: 'flow' }
        ]
      }
    };
    await inv('studio_save_project_workflow', {
      projectId: proj.id,
      workflowId: ${JSON.stringify(cfg.workflowId)},
      name: ${JSON.stringify(cfg.workflowName)},
      category: ${JSON.stringify(cfg.categoryName)},
      draftJson: JSON.stringify(wfGraph)
    });

    const taskDef = {
      id: ${JSON.stringify(cfg.taskId)},
      title: ${JSON.stringify(cfg.taskTitle)},
      categoryId: ${JSON.stringify(cfg.categoryId)},
      workflowId: ${JSON.stringify(cfg.workflowId)},
      defaultEnabled: true,
      priority: 1,
      estimateSec: 45,
      settingsSchema: []
    };
    const catMeta = {
      id: ${JSON.stringify(cfg.categoryId)},
      shortTag: ${JSON.stringify(cfg.categoryTag)},
      displayName: ${JSON.stringify(cfg.categoryName)},
      colorHex: ${JSON.stringify(cfg.accentHex)}
    };
    await inv('studio_upsert_project_task', {
      projectId: proj.id,
      task: taskDef,
      categoryMeta: catMeta
    });

    const uiSchema = {
      schemaVersion: 1,
      projectSlug: proj.slug,
      windowTitle: ${JSON.stringify(cfg.windowTitle)},
      versionBadge: 'v' + proj.version,
      themeAccentHex: ${JSON.stringify(cfg.accentHex)},
      headerBgHex: ${JSON.stringify(cfg.headerBgHex)},
      layoutPreset: ${JSON.stringify(cfg.layoutPreset)},
      defaultWindowSize: [860, 620],
      sectionsOrder: ['device_table', 'task_and_log_split', 'footer_controls'],
      hiddenComponents: [],
      deviceColumns: ${JSON.stringify(cfg.columns)},
      taskCategories: [catMeta],
      tasks: [taskDef],
      utilities: ${JSON.stringify(cfg.utilities)},
      initialRuntimeVars: {}
    };
    await inv('studio_save_ui_schema', { projectId: proj.id, schema: uiSchema });
    await inv('studio_ensure_project_icon', {
      projectId: proj.id,
      accentHex: ${JSON.stringify(cfg.accentHex)},
      overwrite: true
    });

    const preflight = await inv('studio_builder_preflight', { projectId: proj.id });
    if (!preflight.ready) throw new Error('Preflight failed: ' + JSON.stringify(preflight.issues));
    const exported = await inv('studio_builder_export_release', { projectId: proj.id });
    return {
      projectId: proj.id,
      slug: proj.slug,
      exePath: exported.release.exePath,
      exeSha256: exported.release.exeSha256,
      keyId: exported.release.keyId,
      signatureVerifiedOnExport: exported.buildReport.peEmbeddedPubkeyVerified,
      totalDurationMs: exported.buildReport.totalDurationMs
    };
  })()`);
}

async function launchStandaloneMiniTool(exePath, port, screenshotName) {
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
    await new Promise((r) => setTimeout(r, 1200));

    const runtimeInfo = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const boot = await inv('mini_get_bootstrap');
      const text = document.body.innerText;
      return {
        windowTitle: boot.uiSchema?.windowTitle,
        projectSlug: boot.manifest?.project?.slug,
        layoutPreset: boot.uiSchema?.layoutPreset,
        accentHex: boot.uiSchema?.themeAccentHex,
        signatureVerified: boot.signatureVerified,
        keyId: boot.keyId,
        isDemoMode: boot.isDemoMode,
        devicesCount: (boot.devices || []).length,
        tasks: (boot.uiSchema?.tasks || []).map(t => t.title),
        columns: (boot.uiSchema?.deviceColumns || []).map(c => c.label),
        renderedTextSnippet: text.slice(0, 400)
      };
    })()`);

    const shot = await snap(screenshotName, 920, 660);
    ws.close();
    return { runtimeInfo, consoleErrors, shot };
  } finally {
    try {
      child.kill('SIGTERM');
    } catch (_) {}
  }
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

  // Phase 1: Start Studio, create 2 completely distinct Game Projects, export 2 standalone Mini Tools
  let studioChild = spawn(path.join(SANDBOX, 'farmerpro.exe'), [], {
    cwd: SANDBOX,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9240',
      FARMERPRO_PROJECTS_DIR: path.join(SANDBOX, 'projects'),
    },
    stdio: 'ignore',
  });

  let alphaExport, betaExport, studioShot1920, studioConsoleErrors;
  try {
    const page = await waitForCdp(9240, 20000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors = consoleErrors;

    alphaExport = await seedAndBuildProject(evalJs, {
      slug: 'alpha-strategy',
      name: 'Alpha Strategy Auto',
      packageId: 'com.alpha.strategy',
      version: '1.0.0',
      exeName: 'Auto_AlphaStrategy.exe',
      windowTitle: 'Auto Alpha Strategy v1.0.0',
      accentHex: '#1878F5',
      headerBgHex: '#121E2A',
      layoutPreset: 'compact2_col',
      modules: ['mod.adb_core', 'mod.vision_ocr'],
      workflowId: 'wf-alpha-harvest',
      workflowName: 'Harvest Mine Loop',
      checkpointName: 'alpha_cp_done',
      taskId: 'task-alpha-mine',
      taskTitle: 'Thu hoạch Mỏ Alpha',
      categoryId: 'cat.res',
      categoryTag: 'RES',
      categoryName: 'Tài nguyên',
      columns: [
        { id: 'c1', label: 'Mỏ Khoáng', runtimeVarKey: 'ore_count', fallbackDisplay: '0/4', minWindowWidth: 500 },
        { id: 'c2', label: 'Năng lượng', runtimeVarKey: 'stamina', fallbackDisplay: '100%', minWindowWidth: 640 },
      ],
      utilities: [
        { id: 'util.alpha_shield', label: 'Bật khiên bảo vệ Alpha', defaultEnabled: true, requiredModule: null, settingsSchema: [] },
      ],
    });

    betaExport = await seedAndBuildProject(evalJs, {
      slug: 'beta-arena',
      name: 'Beta Arena Bot',
      packageId: 'com.beta.arena',
      version: '2.4.0',
      exeName: 'Auto_BetaArena.exe',
      windowTitle: 'Auto Beta Arena v2.4.0',
      accentHex: '#22C55E',
      headerBgHex: '#0F2419',
      layoutPreset: 'expanded',
      modules: ['mod.adb_core', 'mod.account_vault', 'mod.proxy_network'],
      workflowId: 'wf-beta-pvp',
      workflowName: 'Arena Match Loop',
      checkpointName: 'beta_cp_done',
      taskId: 'task-beta-pvp',
      taskTitle: 'Đấu trường PvP Beta',
      categoryId: 'cat.pvp',
      categoryTag: 'PVP',
      categoryName: 'Đấu trường',
      columns: [
        { id: 'b1', label: 'Hạng Arena', runtimeVarKey: 'arena_rank', fallbackDisplay: 'Tier-A', minWindowWidth: 500 },
      ],
      utilities: [
        { id: 'util.beta_retry', label: 'Tự động ghép trận lại khi thua', defaultEnabled: true, requiredModule: null, settingsSchema: [] },
      ],
    });

    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));
    studioShot1920 = await snap('p4_6_studio_dashboard_1920x1080.png', 1920, 1080);

    ws.close();
  } finally {
    try {
      studioChild.kill('SIGTERM');
    } catch (_) {}
  }

  // Wait briefly to guarantee FarmerPro Studio process is completely closed
  await new Promise((r) => setTimeout(r, 1000));

  // Phase 2: Launch BOTH standalone Mini Tools while FarmerPro Studio is closed!
  const alphaStandalone = await launchStandaloneMiniTool(
    alphaExport.exePath,
    9241,
    'p4_6_minitool_alpha.png'
  );
  const betaStandalone = await launchStandaloneMiniTool(
    betaExport.exePath,
    9242,
    'p4_6_minitool_beta.png'
  );

  const report = {
    milestone: 'P4.6',
    executedAtUtc: new Date().toISOString(),
    studioClosedBeforeLaunch: true,
    studioShot1920,
    studioConsoleErrors,
    alphaExport,
    betaExport,
    alphaStandalone,
    betaStandalone,
  };
  fs.writeFileSync(path.join(EV_DIR, 'p4_6_e2e_report.json'), JSON.stringify(report, null, 2), 'utf8');
  console.log(JSON.stringify(report, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
