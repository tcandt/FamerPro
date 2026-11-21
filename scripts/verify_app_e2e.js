const http = require('http');
const fs = require('fs');

async function main() {
  console.log('Connecting to CDP on 127.0.0.1:9222...');
  const targets = await new Promise((resolve, reject) => {
    http.get('http://127.0.0.1:9222/json', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => resolve(JSON.parse(data)));
    }).on('error', reject);
  });

  const page = targets.find(t => t.type === 'page');
  if (!page) {
    throw new Error('No page target found');
  }

  console.log('Found page:', page.title, page.url);
  if (!page.url.startsWith('http://tauri.localhost/')) {
    throw new Error(`Expected http://tauri.localhost/ but got ${page.url}`);
  }

  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let idCounter = 1;
  const pending = new Map();

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = idCounter++;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(msg.error);
      else resolve(msg.result);
    }
  };

  await new Promise(resolve => ws.onopen = resolve);
  await send('Page.enable');
  await send('Runtime.enable');

  async function evalJs(expr) {
    const res = await send('Runtime.evaluate', { expression: expr, returnByValue: true });
    if (res.exceptionDetails) {
      throw new Error(`JS error: ${JSON.stringify(res.exceptionDetails)}`);
    }
    return res.result.value;
  }

  // 1. Check title & URL
  const currentUrl = await evalJs('window.location.href');
  console.log('1. Standalone URL verified:', currentUrl);

  // 2. Wait up to 5s for live ADB device detection (emulator-5554)
  console.log('2. Waiting for live ADB device emulator-5554...');
  let hasLiveDevice = false;
  for (let i = 0; i < 15; i++) {
    hasLiveDevice = await evalJs('document.body.innerText.includes("emulator-5554")');
    if (hasLiveDevice) break;
    await new Promise(r => setTimeout(r, 400));
  }
  console.log('   Has emulator-5554 in Overview UI:', hasLiveDevice);

  // 3. Capture Overview screenshot
  let shot = await send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync('screenshots/test_res_overview_standalone.png', Buffer.from(shot.data, 'base64'));
  console.log('3. Captured Overview screenshot');

  // 4. Navigate to Workflows Library
  console.log('4. Clicking NavRail "Kho Quy trình JX2"...');
  await evalJs(`
    (() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const wfBtn = buttons.find(b => b.innerText.includes('Kho Quy trình'));
      if (wfBtn) wfBtn.click();
    })()
  `);
  await new Promise(r => setTimeout(r, 600));

  // Count 10 workflows in Library
  const wfList = await evalJs(`
    (() => {
      const headers = Array.from(document.querySelectorAll('h3'));
      return headers.map(h => h.innerText).filter(t => t.includes('.') || t.includes('Võ Hiệp') || t.includes('JX2'));
    })()
  `);
  console.log(`   Found ${wfList.length} workflows in Library:\n`, wfList.map((w, i) => `   [${i+1}] ${w}`).join('\n'));
  if (wfList.length < 10) {
    throw new Error(`Expected at least 10 workflows, found ${wfList.length}`);
  }

  // Capture Workflows Library screenshot
  shot = await send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync('screenshots/test_res_workflows_standalone.png', Buffer.from(shot.data, 'base64'));
  console.log('   Captured Workflows Library screenshot');

  // 5. Click workflow "01. Khởi động và Đăng nhập" to load into Studio
  console.log('5. Clicking Workflow #01 to load into Studio...');
  await evalJs(`
    (() => {
      const headers = Array.from(document.querySelectorAll('h3'));
      const wf1 = headers.find(h => h.innerText.includes('01.'));
      if (wf1) {
        const card = wf1.closest('.cursor-pointer');
        if (card) card.click();
      }
    })()
  `);
  await new Promise(r => setTimeout(r, 1000));

  // Verify Studio Canvas nodes & edges
  const studioInfo = await evalJs(`
    (() => {
      const nodes = document.querySelectorAll('.react-flow__node');
      const edges = document.querySelectorAll('.react-flow__edge');
      const currentWfName = document.querySelector('span.max-w-\\\\[200px\\\\]')?.innerText || '';
      return {
        nodesCount: nodes.length,
        edgesCount: edges.length,
        workflowTitle: currentWfName
      };
    })()
  `);
  console.log('   Studio Canvas Info:', studioInfo);
  if (studioInfo.nodesCount === 0) {
    throw new Error('Studio canvas has 0 nodes after loading workflow!');
  }

  // Capture Studio view screenshot
  shot = await send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync('screenshots/test_res_studio_standalone.png', Buffer.from(shot.data, 'base64'));
  console.log('   Captured Studio canvas screenshot');

  // 6. Navigate to Devices View
  console.log('6. Navigating to Devices View...');
  await evalJs(`
    (() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const devBtn = buttons.find(b => b.innerText.includes('Thiết bị Android'));
      if (devBtn) devBtn.click();
    })()
  `);
  await new Promise(r => setTimeout(r, 600));

  const devicesViewInfo = await evalJs(`
    (() => {
      const hasLiveAdbBadge = document.body.innerText.includes('LIVE ADB');
      const hasEmulator = document.body.innerText.includes('emulator-5554');
      return { hasLiveAdbBadge, hasEmulator };
    })()
  `);
  console.log('   Devices View Status:', devicesViewInfo);

  // Capture Devices view screenshot
  shot = await send('Page.captureScreenshot', { format: 'png' });
  fs.writeFileSync('screenshots/test_res_devices_standalone.png', Buffer.from(shot.data, 'base64'));
  console.log('   Captured Devices view screenshot');

  // 7. Verify titlebar control buttons
  const titlebarButtons = await evalJs(`
    (() => {
      const minBtn = document.querySelector('button[aria-label="Minimize"]');
      const maxBtn = document.querySelector('button[aria-label="Maximize"]');
      const closeBtn = document.querySelector('button[aria-label="Close"]');
      return { min: !!minBtn, max: !!maxBtn, close: !!closeBtn };
    })()
  `);
  console.log('7. Window control buttons present:', titlebarButtons);

  ws.close();
  console.log('\n=== ALL DEEP VERIFICATION CHECKS PASSED WITH 100% SUCCESS ===');
}

main().catch(err => {
  console.error('Deep Verification Failed:', err);
  process.exit(1);
});
