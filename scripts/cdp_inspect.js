const http = require('http');
const fs = require('fs');

async function main() {
  const targets = await new Promise((resolve, reject) => {
    http.get('http://127.0.0.1:9222/json', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => resolve(JSON.parse(data)));
    }).on('error', reject);
  });

  const page = targets.find(t => t.type === 'page');
  if (!page) {
    console.error('No page target found in CDP:', targets);
    process.exit(1);
  }

  console.log('Found page target:', page.title, page.url);
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

  await new Promise((resolve) => ws.onopen = resolve);
  console.log('Connected to CDP WebSocket');

  // Enable Page & Runtime
  await send('Page.enable');
  await send('Runtime.enable');

  // Evaluate URL and basic DOM info
  const evalUrl = await send('Runtime.evaluate', { expression: 'window.location.href' });
  console.log('Location href:', evalUrl.result.value);

  const evalTitle = await send('Runtime.evaluate', { expression: 'document.title' });
  console.log('Document title:', evalTitle.result.value);

  const evalBodyText = await send('Runtime.evaluate', {
    expression: 'document.body.innerText.slice(0, 500)'
  });
  console.log('Body text sample:\n', evalBodyText.result.value);

  // Check if emulator-5554 is in DOM
  const checkEmulator = await send('Runtime.evaluate', {
    expression: 'document.body.innerText.includes("emulator-5554")'
  });
  console.log('Contains emulator-5554:', checkEmulator.result.value);

  // Check if JX2 Mobile is in DOM
  const checkJx2 = await send('Runtime.evaluate', {
    expression: 'document.body.innerText.includes("Võ Hiệp") || document.body.innerText.includes("JX2")'
  });
  console.log('Contains JX2/Võ Hiệp:', checkJx2.result.value);

  // Capture Screenshot
  const screenshot = await send('Page.captureScreenshot', { format: 'png' });
  const buffer = Buffer.from(screenshot.data, 'base64');
  fs.writeFileSync('screenshots/farmerpro_running_standalone.png', buffer);
  console.log(`Saved screenshot to screenshots/farmerpro_running_standalone.png (${buffer.length} bytes)`);

  ws.close();
  process.exit(0);
}

main().catch(err => {
  console.error('Error in CDP inspect:', err);
  process.exit(1);
});
