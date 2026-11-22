const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_3_sandbox');
const PORT = 9236;

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
      if (await evalJs('document.body.innerText.includes("TAURI LIVE IPC")')) break;
      await new Promise((r) => setTimeout(r, 300));
    }

    // Create a clean project with a saved workflow + template so Workflow Studio loads real project data
    const projId = await evalJs(`
      (async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const p = await inv('studio_create_project', {
          req: { slug: 'studio-e2e', name: 'Studio E2E Game', gamePackageId: 'com.studio.e2e' }
        });
        const sync = Array.from(document.querySelectorAll('button')).find(b => b.innerText.includes('Đồng bộ Projects'));
        if (sync) sync.click();
        return p.id;
      })()
    `);
    await new Promise((r) => setTimeout(r, 600));

    // Open Workflow Studio
    await evalJs(`
      (() => {
        const b = Array.from(document.querySelectorAll('aside button')).find(x => x.innerText.includes('Workflow Studio'));
        if (b) b.click();
      })()
    `);
    await new Promise((r) => setTimeout(r, 500));

    // Save current workflow into Active Project via UI button "Lưu Project"
    await evalJs(`
      (() => {
        const b = Array.from(document.querySelectorAll('button')).find(x => x.innerText.includes('Lưu Project'));
        if (b) b.click();
      })()
    `);
    await new Promise((r) => setTimeout(r, 800));

    // Click Breakpoint toggle on selected node & click Validate
    await evalJs(`
      (() => {
        const bp = Array.from(document.querySelectorAll('button')).find(x => x.innerText.includes('Breakpoint'));
        if (bp) bp.click();
        const val = Array.from(document.querySelectorAll('button')).find(x => x.innerText.includes('Kiểm tra'));
        if (val) val.click();
      })()
    `);
    await new Promise((r) => setTimeout(r, 500));

    // Run workflow via legacy TestKit mock driver in UI ("Chạy luồng") to observe real StepEvents in ExecutionDock!
    await evalJs(`
      (() => {
        const runBtn = Array.from(document.querySelectorAll('button')).find(x => x.innerText.trim() === 'Chạy luồng');
        if (runBtn) runBtn.click();
      })()
    `);
    await new Promise((r) => setTimeout(r, 1200));

    const studioCheck = await evalJs(`
      (async () => {
        const wfs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_workflows', { projectId: '${projId}' });
        return {
          savedWorkflowsCount: wfs.length,
          hasValidBadge: document.body.innerText.includes('Hợp lệ'),
          hasBreakpointBadge: document.body.innerText.includes('BP: 1'),
          hasCommandSentEvents: document.body.innerText.includes('command_sent'),
          noFakeStatic965: !document.body.innerText.includes('96.5%')
        };
      })()
    `);

    const captureAt = async (w, h, filename) => {
      await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 1, mobile: false });
      await new Promise((r) => setTimeout(r, 350));
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(shot.data, 'base64'));
      return { filename, width: w, height: h };
    };

    const shots = [];
    shots.push(await captureAt(1600, 900, 'p4_3_studio_1600x900.png'));
    shots.push(await captureAt(1280, 720, 'p4_3_studio_1280x720.png'));

    const report = {
      milestone: 'P4.3',
      executedAtUtc: new Date().toISOString(),
      studioCheck,
      consoleErrors,
      screenshots: shots,
    };
    fs.writeFileSync(path.join(EV_DIR, 'p4_3_report.json'), JSON.stringify(report, null, 2));
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
