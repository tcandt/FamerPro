const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_0_sandbox');
const PORT = 9233;

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

async function waitForCdp(timeoutMs = 15000) {
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
  throw new Error(`Timed out waiting for farmerpro.exe WebView2 CDP on port ${PORT}`);
}

async function main() {
  fs.mkdirSync(EV_DIR, { recursive: true });
  if (fs.existsSync(SANDBOX)) {
    fs.rmSync(SANDBOX, { recursive: true, force: true });
  }
  fs.mkdirSync(path.join(SANDBOX, 'data'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'projects'), { recursive: true });
  copyDirSync(path.join(ROOT, 'bin'), path.join(SANDBOX, 'bin'));

  const exeSrc = path.join(ROOT, 'target', 'debug', 'farmerpro.exe');
  const exeDst = path.join(SANDBOX, 'farmerpro.exe');
  fs.copyFileSync(exeSrc, exeDst);

  const env = {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
    FARMERPRO_PROJECTS_DIR: path.join(SANDBOX, 'projects'),
  };

  console.log('Launching sandboxed farmerpro.exe:', exeDst);
  const child = spawn(exeDst, [], { cwd: SANDBOX, env, stdio: 'ignore' });

  try {
    const page = await waitForCdp(20000);
    console.log('Connected to live Tauri page:', page.url);

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
      if (r.exceptionDetails) {
        throw new Error(`Eval error: ${JSON.stringify(r.exceptionDetails)}`);
      }
      return r.result.value;
    };

    // Wait for React render + Tauri LIVE IPC StatusBar badge
    let hasTauriLive = false;
    for (let i = 0; i < 20; i++) {
      hasTauriLive = await evalJs('document.body.innerText.includes("TAURI LIVE IPC")');
      if (hasTauriLive) break;
      await new Promise((r) => setTimeout(r, 300));
    }

    // Verify real Rust IPC calls directly through Tauri internals
    const ipcProbe = await evalJs(`
      (async () => {
        const inv = window.__TAURI_INTERNALS__?.invoke;
        if (!inv) return { ok: false, reason: 'missing __TAURI_INTERNALS__.invoke' };
        const status = await inv('app_get_status');
        const projects = await inv('studio_list_projects');
        const devices = await inv('list_devices');
        return { ok: true, status, projectsCount: projects.length, devicesCount: devices.length };
      })()
    `);

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
        clientWidth: document.documentElement.clientWidth,
        scrollHeight: document.documentElement.scrollHeight,
        clientHeight: document.documentElement.clientHeight,
        headerHeight: document.querySelector('header')?.getBoundingClientRect().height || 0
      })`);
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      const outPath = path.join(EV_DIR, filename);
      fs.writeFileSync(outPath, Buffer.from(shot.data, 'base64'));
      return { filename, width: w, height: h, ...metrics };
    };

    const shots = [];
    shots.push(await captureAt(1280, 720, 'p4_0_projects_1280x720.png'));
    shots.push(await captureAt(1600, 900, 'p4_0_projects_1600x900.png'));
    shots.push(await captureAt(1920, 1080, 'p4_0_projects_1920x1080.png'));

    // Test sidebar collapse on 1280x720
    await evalJs(`document.querySelector('button[aria-label="Toggle Sidebar"]')?.click()`);
    shots.push(await captureAt(1280, 720, 'p4_0_collapsed_1280x720.png'));
    // Re-expand sidebar and switch to Workflow Studio
    await evalJs(`document.querySelector('button[aria-label="Toggle Sidebar"]')?.click()`);
    await evalJs(`
      (() => {
        const btns = Array.from(document.querySelectorAll('aside button'));
        const b = btns.find(x => x.innerText.includes('Workflow Studio'));
        if (b) b.click();
      })()
    `);
    shots.push(await captureAt(1600, 900, 'p4_0_studio_1600x900.png'));

    const report = {
      milestone: 'P4.0',
      executedAtUtc: new Date().toISOString(),
      tauriUrl: page.url,
      hasTauriLiveBadge: hasTauriLive,
      ipcProbe,
      consoleErrors,
      screenshots: shots,
    };
    fs.writeFileSync(path.join(EV_DIR, 'p4_0_report.json'), JSON.stringify(report, null, 2));
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
