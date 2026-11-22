const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_5_sandbox');
const PORT = 9238;

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

async function waitForCdp(timeoutMs = 20000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const targets = await new Promise((resolve, reject) => {
        http
          .get(`http://127.0.0.1:${PORT}/json`, (res) => {
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
  throw new Error(`Timed out waiting for WebView2 CDP on port ${PORT}`);
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

  const env = {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
    FARMERPRO_PROJECTS_DIR: path.join(SANDBOX, 'projects'),
  };

  const child = spawn(path.join(SANDBOX, 'farmerpro.exe'), [], { cwd: SANDBOX, env, stdio: 'ignore' });

  try {
    const page = await waitForCdp(20000);
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

    const setViewport = async (width, height) => {
      await send('Emulation.setDeviceMetricsOverride', {
        width,
        height,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await new Promise((r) => setTimeout(r, 350));
    };

    const snap = async (filename, width, height) => {
      await setViewport(width, height);
      const res = await send('Page.captureScreenshot', { format: 'png' });
      const outPath = path.join(EV_DIR, filename);
      fs.writeFileSync(outPath, Buffer.from(res.data, 'base64'));
      return { filename, width, height };
    };

    for (let i = 0; i < 30; i++) {
      const ok = await evalJs('Boolean(window.__TAURI_INTERNALS__)');
      if (ok) break;
      await new Promise((r) => setTimeout(r, 250));
    }

    const seeded = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const p = await inv('studio_create_project', {
        req: {
          slug: 'p45-builder-game',
          name: 'Realm Conquest Builder',
          gamePackageId: 'com.realm.conquest',
          version: '2.0.1',
          exeName: 'Auto_RealmConquest.exe',
          windowTitle: 'Auto Realm Conquest v2.0.1',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      const graph = {
        schemaVersion: 1,
        workflowId: 'wf-p45-main',
        name: 'Realm Daily Sweep',
        status: 'published',
        graph: {
          nodes: [
            { id: 'n-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'n-cp', type: 'state.checkpoint', position: { x: 240, y: 80 }, config: { name: 'realm_cp' } },
            { id: 'n-end', type: 'flow.end', position: { x: 440, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'e1', source: 'n-start', sourcePort: 'flow', target: 'n-cp', targetPort: 'flow' },
            { id: 'e2', source: 'n-cp', sourcePort: 'flow', target: 'n-end', targetPort: 'flow' }
          ]
        }
      };
      await inv('studio_save_project_workflow', {
        projectId: p.id,
        workflowId: 'wf-p45-main',
        name: 'Realm Daily Sweep',
        category: 'Daily',
        draftJson: JSON.stringify(graph)
      });
      await inv('studio_upsert_project_task', {
        projectId: p.id,
        task: {
          id: 'task-realm-daily',
          title: 'Quét Phó bản Realm',
          categoryId: 'cat.pve',
          workflowId: 'wf-p45-main',
          defaultEnabled: true,
          priority: 1,
          estimateSec: 40,
          settingsSchema: []
        },
        categoryMeta: {
          id: 'cat.pve',
          shortTag: 'PVE',
          displayName: 'Phó bản',
          colorHex: '#22C55E'
        }
      });
      return { projectId: p.id };
    })()`);

    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));

    // Navigate to Mini Tool Builder
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Mini Tool Builder'));
      if (!btn) throw new Error('Mini Tool Builder nav not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 600));

    // Switch layout to expanded & preview state to RUNNING
    await evalJs(`(() => {
      const expandedBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.trim() === 'expanded');
      if (expandedBtn) expandedBtn.click();
      const runningBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.trim() === 'RUNNING');
      if (runningBtn) runningBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 250));

    // Click Build & Xuất EXE (which saves schema, creates valid ICO if missing, runs preflight, exports signed EXE)
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Build & Xuất EXE'));
      if (!btn) throw new Error('Build & Xuất EXE button not found');
      btn.click();
    })()`);

    let buildCheck = null;
    for (let i = 0; i < 40; i++) {
      buildCheck = await evalJs(`(async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const releases = await inv('studio_list_project_releases', { projectId: '${seeded.projectId}' });
        const text = document.body.innerText;
        return {
          releasesCount: releases.length,
          exePath: releases[0]?.exePath || null,
          keyId: releases[0]?.keyId || null,
          hasReadyBadge: text.includes('READY'),
          hasBuildReport: text.includes('Build Report Mới nhất') || text.includes('PE Verified: true'),
          hasTaskInPreview: text.includes('Quét Phó bản Realm')
        };
      })()`);
      if (buildCheck.releasesCount > 0 && buildCheck.hasReadyBadge) break;
      await new Promise((r) => setTimeout(r, 350));
    }

    const screenshots = [];
    screenshots.push(await snap('p4_5_builder_1600x900.png', 1600, 900));
    screenshots.push(await snap('p4_5_builder_1280x720.png', 1280, 720));

    const exportedExeExists = Boolean(buildCheck?.exePath && fs.existsSync(buildCheck.exePath));

    ws.close();
    console.log(
      JSON.stringify(
        {
          milestone: 'P4.5',
          executedAtUtc: new Date().toISOString(),
          buildCheck,
          exportedExeExists,
          consoleErrors,
          screenshots,
        },
        null,
        2
      )
    );
  } finally {
    try {
      child.kill('SIGTERM');
    } catch (_) {}
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
