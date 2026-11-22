const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p4_evidence');
const SANDBOX = path.join(EV_DIR, 'p4_4_sandbox');
const PORT = 9237;

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

    // Wait for Tauri internals & seed a project with a valid workflow
    for (let i = 0; i < 30; i++) {
      const ok = await evalJs('Boolean(window.__TAURI_INTERNALS__)');
      if (ok) break;
      await new Promise((r) => setTimeout(r, 250));
    }

    const seeded = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const p = await inv('studio_create_project', {
        req: {
          slug: 'p44-tasks-game',
          name: 'P4.4 Generic Game',
          gamePackageId: 'com.generic.game',
          version: '1.1.0',
          exeName: 'Auto_GenericGame.exe',
          windowTitle: 'Auto Generic Game',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      const graph = {
        schemaVersion: 1,
        workflowId: 'wf-p44-daily',
        name: 'Daily Generic Loop',
        status: 'published',
        graph: {
          nodes: [
            { id: 'n-start', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
            { id: 'n-cp', type: 'state.checkpoint', position: { x: 240, y: 80 }, config: { name: 'p44_cp_ok' } },
            { id: 'n-end', type: 'flow.end', position: { x: 440, y: 80 }, config: {} }
          ],
          edges: [
            { id: 'e1', source: 'n-start', sourcePort: 'flow', target: 'n-cp', targetPort: 'flow' },
            { id: 'e2', source: 'n-cp', sourcePort: 'flow', target: 'n-end', targetPort: 'flow' }
          ]
        }
      };
      const wf = await inv('studio_save_project_workflow', {
        projectId: p.id,
        workflowId: 'wf-p44-daily',
        name: 'Daily Generic Loop',
        category: 'Daily',
        draftJson: JSON.stringify(graph)
      });
      return { projectId: p.id, workflowId: wf.id };
    })()`);

    await evalJs('location.reload()');
    await new Promise((r) => setTimeout(r, 1500));

    // Navigate to Tasks & Modules
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Tasks & Modules'));
      if (!btn) throw new Error('Tasks & Modules nav not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 500));

    // Open "+ Thêm Task mới", fill form, save
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Thêm Task mới'));
      if (!btn) throw new Error('Thêm Task mới button not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 250));

    await evalJs(`(() => {
      const form = document.querySelector('form');
      if (!form) throw new Error('Task modal form not found');
      const titleInput = form.querySelector('input[placeholder*="Vận tiêu"]');
      if (!titleInput) throw new Error('Title input not found');
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setter.call(titleInput, 'Nhiệm vụ Vượt Ải Tự Động');
      titleInput.dispatchEvent(new Event('input', { bubbles: true }));
    })()`);
    await new Promise((r) => setTimeout(r, 150));

    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Lưu Task vào SQLite'));
      if (!btn) throw new Error('Save Task button not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 800));

    // Toggle mod.proxy_network checkbox
    await evalJs(`(() => {
      const cards = Array.from(document.querySelectorAll('div'));
      const target = cards.find(d => d.textContent && d.textContent.includes('mod.proxy_network') && d.querySelectorAll('input[type="checkbox"]').length === 2);
      if (!target) throw new Error('mod.proxy_network card not found');
      const cb = target.querySelectorAll('input[type="checkbox"]')[0];
      cb.click();
    })()`);
    await new Promise((r) => setTimeout(r, 700));

    // Click Chạy thử on the newly created task
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Chạy thử'));
      if (!btn) throw new Error('Chạy thử button not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 1200));

    const tasksCheck = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      const tasks = await inv('studio_list_project_tasks', { projectId: '${seeded.projectId}' });
      const mods = await inv('studio_list_project_modules', { projectId: '${seeded.projectId}' });
      const text = document.body.innerText;
      return {
        tasksCount: tasks.length,
        taskTitle: tasks[0]?.title || null,
        proxyModulePackaged: Boolean(mods.find(m => m.moduleId === 'mod.proxy_network')?.packagedInRelease),
        noFakeJx2: !text.includes('Nhiệm vụ tân thủ chính tuyến'),
        hasTestkitBadge: text.includes('TESTKIT')
      };
    })()`);

    const screenshots = [];
    screenshots.push(await snap('p4_4_tasks_1600x900.png', 1600, 900));

    // Navigate to Test & Debug
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Test & Debug'));
      if (!btn) throw new Error('Test & Debug nav not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 1000));

    const runsCheck = await evalJs(`(() => {
      const text = document.body.innerText;
      return {
        noFakeDemoJob8492: !text.includes('job-demo-8492'),
        noFakeCoords242: !text.includes('242.0, 428.0'),
        hasStepEvents: text.includes('events') && !text.includes('0 events'),
        hasCheckpointBox: text.includes('n-cp') || text.includes('Checkpoint')
      };
    })()`);

    screenshots.push(await snap('p4_4_runs_1600x900.png', 1600, 900));
    screenshots.push(await snap('p4_4_runs_1280x720.png', 1280, 720));

    // Navigate to Proxy & Port Map and run real Rust Proxy Probe on closed port 65432
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Proxy & Port Map'));
      if (!btn) throw new Error('Proxy & Port Map nav not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 400));

    await evalJs(`(() => {
      const inputs = Array.from(document.querySelectorAll('input'));
      const portInput = inputs.find(i => i.value === '10808');
      if (portInput) {
        const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
        setter.call(portInput, '65432');
        portInput.dispatchEvent(new Event('input', { bubbles: true }));
      }
    })()`);
    await new Promise((r) => setTimeout(r, 150));

    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Chạy Proxy Probe Thật'));
      if (!btn) throw new Error('Proxy probe button not found');
      btn.click();
    })()`);
    let networkCheck = null;
    for (let i = 0; i < 20; i++) {
      networkCheck = await evalJs(`(() => {
        const text = document.body.innerText;
        return {
          noFakeViettelIp: !text.includes('103.142.26.88'),
          hasHonestUnreachable: text.includes('UNREACHABLE') || text.includes('Error:')
        };
      })()`);
      if (networkCheck.hasHonestUnreachable) break;
      await new Promise((r) => setTimeout(r, 250));
    }

    // Navigate to Account Vault and verify empty/real state + create account
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Account Vault'));
      if (!btn) throw new Error('Account Vault nav not found');
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 500));

    const accountsBefore = await evalJs(`!document.body.innerText.includes('Farmer_Main_01')`);

    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent && b.textContent.includes('Thêm tài khoản'));
      btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 250));

    await evalJs(`(() => {
      const form = document.querySelector('form');
      const aliasInput = form.querySelector('input[placeholder="acc_worker_01"]');
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setter.call(aliasInput, 'vault_worker_01');
      aliasInput.dispatchEvent(new Event('input', { bubbles: true }));
      const submitBtn = Array.from(form.querySelectorAll('button')).find(b => b.type === 'submit');
      submitBtn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 800));

    const accountsAfter = await evalJs(`document.body.innerText.includes('vault_worker_01')`);

    ws.close();
    console.log(
      JSON.stringify(
        {
          milestone: 'P4.4',
          executedAtUtc: new Date().toISOString(),
          tasksCheck,
          runsCheck,
          networkCheck,
          accountsCheck: { noFakeFarmerMain01: accountsBefore, createdRealAccount: accountsAfter },
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
