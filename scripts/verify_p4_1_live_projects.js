const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_1_sandbox');
const PORT = 9234;

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

    for (let i = 0; i < 20; i++) {
      if (await evalJs('document.body.innerText.includes("Game Projects & Release Workspace")')) break;
      await new Promise((r) => setTimeout(r, 300));
    }

    // 1. Test real IPC Create Project x2 + Clone + Update + Delete + Integrity inside live farmerpro.exe
    const crudResults = await evalJs(`
      (async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const p1 = await inv('studio_create_project', {
          req: {
            slug: 'dragon-arena',
            name: 'Dragon Arena Auto',
            gamePackageId: 'com.arena.dragon',
            version: '1.0.0',
            targetWidthPx: 1080,
            targetHeightPx: 1920,
            bundleAdb: true,
            enabledModules: ['mod.adb_core', 'mod.vision_ocr', 'mod.proxy_network']
          }
        });
        await inv('studio_save_project_workflow', {
          projectId: p1.id,
          workflowId: 'wf-arena-daily',
          name: 'Daily Arena Battle',
          category: 'PvP',
          draftJson: JSON.stringify({
            schemaVersion: 1,
            workflowId: 'wf-arena-daily',
            name: 'Daily Arena Battle',
            status: 'published',
            graph: {
              nodes: [
                { id: 'n1', type: 'flow.start', position: { x: 0, y: 0 } },
                { id: 'n2', type: 'flow.end', position: { x: 240, y: 0 } }
              ],
              edges: [{ id: 'e1', source: 'n1', sourcePort: 'flow', target: 'n2', targetPort: 'flow' }]
            }
          })
        });
        const p2 = await inv('studio_create_project', {
          req: {
            slug: 'farm-valley',
            name: 'Farm Valley Helper',
            gamePackageId: 'com.valley.farm',
            version: '2.1.0',
            targetWidthPx: 1280,
            targetHeightPx: 720,
            bundleAdb: true
          }
        });
        const p3Clone = await inv('studio_clone_project', {
          req: {
            sourceProjectId: p1.id,
            newSlug: 'dragon-arena-sea',
            newName: 'Dragon Arena SEA',
            newGamePackageId: 'com.arena.dragon.sea'
          }
        });
        const p3Upd = await inv('studio_update_project', {
          req: { id: p3Clone.id, version: '1.2.5', name: 'Dragon Arena SEA v1.2' }
        });
        const detailP1 = await inv('studio_get_project_detail', { projectId: p1.id });
        const detailP2 = await inv('studio_get_project_detail', { projectId: p2.id });
        const listedBeforeDelete = await inv('studio_list_projects');

        const pTemp = await inv('studio_create_project', {
          req: { slug: 'temp-cleanup', name: 'Temp Proj', gamePackageId: 'com.tmp' }
        });
        const deleted = await inv('studio_delete_project', { projectId: pTemp.id, removeDiskDir: true });
        const listedAfterDelete = await inv('studio_list_projects');
        const integrity = await inv('studio_check_db_integrity');

        return {
          p1Id: p1.id,
          p2Id: p2.id,
          p3Id: p3Upd.id,
          p3Version: p3Upd.version,
          p1Workflows: detailP1.workflows.length,
          p2Workflows: detailP2.workflows.length,
          countBeforeDelete: listedBeforeDelete.length,
          deletedTemp: deleted,
          countAfterDelete: listedAfterDelete.length,
          integrityOk: integrity.isHealthy,
          fkViolations: integrity.foreignKeyViolations
        };
      })()
    `);

    // 2. Click "Đồng bộ Projects" in real UI to load the 3 persisted SQLite projects into React UI!
    await evalJs(`
      (() => {
        const btns = Array.from(document.querySelectorAll('button'));
        const sync = btns.find(b => b.innerText.includes('Đồng bộ Projects'));
        if (sync) sync.click();
      })()
    `);
    await new Promise((r) => setTimeout(r, 800));

    // Verify UI shows "Dragon Arena Auto" and "Farm Valley Helper"
    const uiHasProjects = await evalJs(
      'document.body.innerText.includes("Dragon Arena Auto") && document.body.innerText.includes("Farm Valley Helper")'
    );

    const captureAt = async (w, h, filename) => {
      await send('Emulation.setDeviceMetricsOverride', {
        width: w,
        height: h,
        deviceScaleFactor: 1,
        mobile: false,
      });
      await new Promise((r) => setTimeout(r, 350));
      const metrics = await evalJs(`({
        scrollWidth: document.documentElement.scrollWidth,
        clientWidth: document.documentElement.clientWidth
      })`);
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(shot.data, 'base64'));
      return { filename, width: w, height: h, ...metrics };
    };

    const shots = [];
    shots.push(await captureAt(1280, 720, 'p4_1_projects_1280x720.png'));
    shots.push(await captureAt(1600, 900, 'p4_1_projects_1600x900.png'));
    shots.push(await captureAt(1920, 1080, 'p4_1_projects_1920x1080.png'));

    // Also open Create Project modal to capture UI form state at 1600x900
    await evalJs(`
      (() => {
        const btns = Array.from(document.querySelectorAll('button'));
        const c = btns.find(b => b.innerText.includes('Tạo Game Project'));
        if (c) c.click();
      })()
    `);
    shots.push(await captureAt(1600, 900, 'p4_1_create_modal_1600x900.png'));

    const report = {
      milestone: 'P4.1',
      executedAtUtc: new Date().toISOString(),
      uiRenderedRealSqliteProjects: uiHasProjects,
      crudAndIsolation: crudResults,
      consoleErrors,
      screenshots: shots,
    };
    fs.writeFileSync(path.join(EV_DIR, 'p4_1_report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
    ws.close();
  } finally {
    try {
      child.kill();
    } catch (_) {}
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
