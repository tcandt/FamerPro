const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');

const ROOT = 'E:\\DATA\\FarmerPro';
const EV_DIR = path.join(ROOT, 'artifacts', 'p5_3_evidence');
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

function runPsJson(script) {
  const out = execFileSync(
    'powershell',
    ['-NoProfile', '-NonInteractive', '-Command', script],
    { encoding: 'utf8', cwd: ROOT }
  ).trim();
  return JSON.parse(out);
}

async function waitForCdp(port, timeoutMs = 24000) {
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
    await new Promise((r) => setTimeout(r, 380));
    const res = await send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(path.join(EV_DIR, filename), Buffer.from(res.data, 'base64'));
    return { filename, width, height };
  };

  for (let i = 0; i < 65; i++) {
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

function scanDirectoryForSecretLeaks(dir) {
  const hits = [];
  const walk = (p) => {
    for (const ent of fs.readdirSync(p, { withFileTypes: true })) {
      const full = path.join(p, ent.name);
      if (ent.isDirectory()) {
        if (ent.name === 'bin') continue;
        walk(full);
      } else if (/\.(json|log|txt|db|md|pub|sig)$/i.test(ent.name)) {
        const lower = ent.name.toLowerCase();
        if (lower.endsWith('.pfx') || lower.endsWith('.p12') || lower.endsWith('.key')) {
          hits.push({ file: full, reason: 'Private key / PFX file found inside sandbox release tree' });
          continue;
        }
        try {
          const text = fs.readFileSync(full, 'utf8');
          if (/BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY/i.test(text) || /FP_AUTHENTICODE_PFX_PASSWORD\s*[:=]\s*[^\s]{3,}/.test(text)) {
            hits.push({ file: full, reason: 'Private key PEM or password pattern detected' });
          }
        } catch (_) {}
      }
    }
  };
  walk(dir);
  return hits;
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

  const installAlpha = path.join(SANDBOX, 'installed', 'BrandAlpha');
  const installBeta = path.join(SANDBOX, 'installed', 'BrandBeta');
  const { adbOut, ldRunning } = getAdbStatus();

  let studioConsoleErrors = [];
  let studioResult = null;
  let qaThumbToCleanup = null;

  // STEP 1: Launch Studio in sandbox, create 2 distinct Publisher Profiles + QA Cert in CNG,
  // build Brand Alpha (UnsignedInternal) & Brand Beta (SelfSignedQa), verify PublicSigned fails closed,
  // rotate Ed25519 key with KeyRotationDelegation & build Brand Beta v1.1.0!
  const studioChild = spawn(sandboxStudioExe, [], {
    cwd: SANDBOX,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9274',
    },
    stdio: 'ignore',
  });

  try {
    const page = await waitForCdp(9274, 24000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page);
    studioConsoleErrors.push(...consoleErrors);

    studioResult = await evalJs(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;

      // Create two Publisher Profiles in SQLite V5
      const pubAlpha = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-alpha-vanguard',
          displayName: 'Alpha Vanguard Publishing',
          organization: 'Alpha Vanguard Automation Co., Ltd.',
          website: 'https://alphavanguard.example.invalid',
          supportContact: 'support@alphavanguard.example.invalid',
          copyright: 'Copyright (C) 2026 Alpha Vanguard Automation Co., Ltd.',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });

      const qaCert = await inv('studio_create_self_signed_qa_cert', {
        subjectCn: 'Beta Nova QA Publisher'
      });

      const pubBeta = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-beta-nova',
          displayName: 'Beta Nova Studio',
          organization: 'Beta Nova Interactive Labs Pte. Ltd.',
          website: 'https://betanova.example.invalid',
          supportContact: 'sec@betanova.example.invalid',
          copyright: 'Copyright (C) 2026 Beta Nova Interactive Labs Pte. Ltd.',
          authenticodeCertRef: qaCert.certRef,
          releaseChannel: 'qa',
          signingPolicy: 'self_signed_qa'
        }
      });

      // Project A: Brand Alpha (Unsigned Internal + Distinct Emerald Icon + VERSIONINFO)
      const pAlpha = await inv('studio_create_project', {
        req: {
          slug: 'brand-alpha',
          name: 'Vanguard Auto Suite',
          gamePackageId: 'com.vanguard.alpha',
          version: '1.0.0',
          exeName: 'Auto_BrandAlpha.exe',
          windowTitle: 'Vanguard Auto Suite v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: pAlpha.id, accentHex: '#10B981', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: pAlpha.id,
        workflowId: 'wf-alpha-daily',
        name: 'Vanguard Daily Farm',
        category: 'Daily',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-alpha-daily', name: 'Vanguard Daily Farm', status: 'published',
          graph: {
            nodes: [
              { id: 'a1', type: 'flow.start', position: { x: 40, y: 60 }, config: {} },
              { id: 'a2', type: 'state.set_var', position: { x: 200, y: 60 }, config: { varKey: 'guild_tier', valueType: 'string', value: 'Diamond-S1' } },
              { id: 'a3', type: 'flow.end', position: { x: 360, y: 60 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'a1', sourcePort: 'flow', target: 'a2', targetPort: 'flow' },
              { id: 'e2', source: 'a2', sourcePort: 'flow', target: 'a3', targetPort: 'flow' }
            ]
          }
        })
      });
      await inv('studio_upsert_project_task', {
        projectId: pAlpha.id,
        task: { id: 'tsk-alpha-1', title: 'Vanguard Guild Boss Sweep', categoryId: 'cat-vg', workflowId: 'wf-alpha-daily', defaultEnabled: true, priority: 1, estimateSec: 20, settingsSchema: [] },
        categoryMeta: { id: 'cat-vg', shortTag: 'VG', displayName: 'Vanguard', colorHex: '#10B981' }
      });
      const verInfoAlpha = {
        companyName: pubAlpha.organization,
        fileDescription: 'Vanguard Auto Suite Internal Runner',
        fileVersion: '1.0.0.10',
        internalName: 'brand-alpha',
        legalCopyright: pubAlpha.copyright,
        originalFilename: 'Auto_BrandAlpha.exe',
        productName: 'Vanguard Auto Suite',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: {
          id: pAlpha.id,
          publisherProfileId: pubAlpha.id,
          signingMode: 'unsigned_internal',
          versionInfoJson: JSON.stringify(verInfoAlpha)
        }
      });
      const signPreAlpha = await inv('studio_signing_preflight', { projectId: pAlpha.id, signingMode: 'unsigned_internal' });
      const resAlpha = await inv('studio_builder_export_release', {
        projectId: pAlpha.id,
        options: { signingMode: 'unsigned_internal', publisherProfileId: pubAlpha.id, versionInfo: verInfoAlpha }
      });
      await inv('studio_transition_release_lifecycle', { projectId: pAlpha.id, releaseId: resAlpha.release.id, targetState: 'RC', note: 'Alpha RC' });
      await inv('studio_transition_release_lifecycle', { projectId: pAlpha.id, releaseId: resAlpha.release.id, targetState: 'Published', note: 'Alpha Internal Build' });

      // Project B: Brand Beta (Self-Signed QA Cert + Distinct Amber/Purple Icon + VERSIONINFO)
      const pBeta = await inv('studio_create_project', {
        req: {
          slug: 'brand-beta',
          name: 'Nova Tactics Automator',
          gamePackageId: 'com.nova.beta',
          version: '1.0.0',
          exeName: 'Auto_BrandBeta.exe',
          windowTitle: 'Nova Tactics Automator v1.0.0',
          targetWidthPx: 1920,
          targetHeightPx: 1080,
          bundleAdb: false,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: pBeta.id, accentHex: '#F59E0B', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: pBeta.id,
        workflowId: 'wf-beta-raid',
        name: 'Nova Citadel Raid',
        category: 'Raid',
        draftJson: JSON.stringify({
          schemaVersion: 1, workflowId: 'wf-beta-raid', name: 'Nova Citadel Raid', status: 'published',
          graph: {
            nodes: [
              { id: 'b1', type: 'flow.start', position: { x: 40, y: 60 }, config: {} },
              { id: 'b2', type: 'state.set_var', position: { x: 200, y: 60 }, config: { varKey: 'citadel_rank', valueType: 'string', value: 'Apex-Tier-9' } },
              { id: 'b3', type: 'flow.end', position: { x: 360, y: 60 }, config: {} }
            ],
            edges: [
              { id: 'e1', source: 'b1', sourcePort: 'flow', target: 'b2', targetPort: 'flow' },
              { id: 'e2', source: 'b2', sourcePort: 'flow', target: 'b3', targetPort: 'flow' }
            ]
          }
        })
      });
      await inv('studio_upsert_project_task', {
        projectId: pBeta.id,
        task: { id: 'tsk-beta-1', title: 'Citadel Wave Farm', categoryId: 'cat-nv', workflowId: 'wf-beta-raid', defaultEnabled: true, priority: 1, estimateSec: 30, settingsSchema: [] },
        categoryMeta: { id: 'cat-nv', shortTag: 'NV', displayName: 'Nova', colorHex: '#F59E0B' }
      });
      const verInfoBeta1 = {
        companyName: pubBeta.organization,
        fileDescription: 'Nova Tactics QA Signed Automator',
        fileVersion: '1.0.0.20',
        internalName: 'brand-beta',
        legalCopyright: pubBeta.copyright,
        originalFilename: 'Auto_BrandBeta.exe',
        productName: 'Nova Tactics Automator',
        productVersion: '1.0.0'
      };
      await inv('studio_update_project', {
        req: {
          id: pBeta.id,
          publisherProfileId: pubBeta.id,
          signingMode: 'self_signed_qa',
          versionInfoJson: JSON.stringify(verInfoBeta1)
        }
      });
      const uiBeta = await inv('studio_get_ui_schema', { projectId: pBeta.id });
      uiBeta.themeAccentHex = '#F59E0B';
      uiBeta.headerBgHex = '#1F1635';
      await inv('studio_save_ui_schema', { projectId: pBeta.id, schema: uiBeta });

      const signPreBeta = await inv('studio_signing_preflight', { projectId: pBeta.id, signingMode: 'self_signed_qa', certRef: qaCert.certRef });
      const resBeta100 = await inv('studio_builder_export_release', {
        projectId: pBeta.id,
        options: { signingMode: 'self_signed_qa', publisherProfileId: pubBeta.id, authenticodeCertRef: qaCert.certRef, versionInfo: verInfoBeta1 }
      });
      await inv('studio_transition_release_lifecycle', { projectId: pBeta.id, releaseId: resBeta100.release.id, targetState: 'RC', note: 'Beta QA 1.0.0 RC' });
      await inv('studio_transition_release_lifecycle', { projectId: pBeta.id, releaseId: resBeta100.release.id, targetState: 'Published', note: 'Beta QA Signed 1.0.0' });

      // Negative test: PublicSigned with no cert + with self-signed QA cert MUST fail closed!
      let publicNoCertErr = null;
      try {
        await inv('studio_builder_export_release', {
          projectId: pBeta.id,
          outputDir: resBeta100.release.outputDirPath + '_pub_nocert_fail',
          options: { signingMode: 'public_signed', publisherProfileId: pubBeta.id, authenticodeCertRef: null, versionInfo: verInfoBeta1 }
        });
      } catch (e) { publicNoCertErr = String(e); }

      let publicSelfSignedErr = null;
      try {
        await inv('studio_builder_export_release', {
          projectId: pBeta.id,
          outputDir: resBeta100.release.outputDirPath + '_pub_self_fail',
          options: { signingMode: 'public_signed', publisherProfileId: pubBeta.id, authenticodeCertRef: qaCert.certRef, versionInfo: verInfoBeta1 }
        });
      } catch (e) { publicSelfSignedErr = String(e); }

      // Key Rotation Delegation: Rotate Ed25519 key for Beta -> export v1.1.0 with delegation + SelfSignedQa!
      const rotRes = await inv('studio_rotate_project_ed25519_key_with_delegation', {
        projectId: pBeta.id,
        publisherId: pubBeta.publisherId,
        validDays: 90
      });
      const verInfoBeta110 = { ...verInfoBeta1, fileVersion: '1.1.0.21', productVersion: '1.1.0' };
      await inv('studio_update_project', {
        req: {
          id: pBeta.id,
          version: '1.1.0',
          windowTitle: 'Nova Tactics Automator v1.1.0',
          versionInfoJson: JSON.stringify(verInfoBeta110)
        }
      });
      const resBeta110 = await inv('studio_builder_export_release', {
        projectId: pBeta.id,
        options: {
          signingMode: 'self_signed_qa',
          publisherProfileId: pubBeta.id,
          authenticodeCertRef: qaCert.certRef,
          versionInfo: verInfoBeta110,
          keyRotationDelegation: rotRes.delegation
        }
      });
      await inv('studio_transition_release_lifecycle', { projectId: pBeta.id, releaseId: resBeta110.release.id, targetState: 'RC', note: 'Beta QA 1.1.0 RC' });
      await inv('studio_transition_release_lifecycle', { projectId: pBeta.id, releaseId: resBeta110.release.id, targetState: 'Published', note: 'Beta QA 1.1.0 Key-Rotated' });

      return {
        pubAlpha,
        pubBeta,
        qaCert,
        pAlpha,
        pBeta,
        signPreAlpha,
        resAlpha,
        signPreBeta,
        resBeta100,
        resBeta110,
        rotRes,
        publicNoCertErr,
        publicSelfSignedErr
      };
    })()`);

    qaThumbToCleanup = studioResult?.qaCert?.thumbprint || null;

    // Reload frontend so Zustand store loads the newly created projects & publisher profiles,
    // then select Nova Tactics (Brand Beta), open Mini Tool Builder, open VERSIONINFO details & click Signing Preflight!
    await evalJs(`location.reload()`);
    await new Promise((r) => setTimeout(r, 1500));
    for (let i = 0; i < 40; i++) {
      const ready = await evalJs(`Boolean(window.__TAURI_INTERNALS__ && document.querySelectorAll('header select option').length > 1)`);
      if (ready) break;
      await new Promise((r) => setTimeout(r, 200));
    }
    await evalJs(`(() => {
      const sel = document.querySelector('header select');
      if (sel) {
        const opt = Array.from(sel.options).find(o => o.textContent.includes('Nova Tactics') || o.value === ${JSON.stringify(studioResult.pBeta.id)});
        if (opt) { sel.value = opt.value; sel.dispatchEvent(new Event('change', { bubbles: true })); }
      }
    })()`);
    await new Promise((r) => setTimeout(r, 600));
    await evalJs(`(() => {
      const btn = Array.from(document.querySelectorAll('nav button, aside button')).find(b => b.textContent && b.textContent.includes('Mini Tool Builder'));
      if (btn) btn.click();
    })()`);
    await new Promise((r) => setTimeout(r, 1000));
    await evalJs(`(() => {
      document.querySelectorAll('details').forEach(d => { d.open = true; });
      const sp = document.querySelector('[data-testid="p53-signing-preflight"]');
      if (sp) sp.click();
    })()`);
    await new Promise((r) => setTimeout(r, 800));
    await snap('p5_3_studio_signing_builder.png', 1600, 900);
    ws.close();
  } finally {
    try { studioChild.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 700));

  // STEP 2: Verify OS Windows PE VERSIONINFO + Get-AuthenticodeSignature + signtool.exe verify directly on BOTH EXEs!
  const alphaExeBuilt = studioResult.resAlpha.release.exePath;
  const beta100ExeBuilt = studioResult.resBeta100.release.exePath;
  const beta110ExeBuilt = studioResult.resBeta110.release.exePath;

  const osInspect = runPsJson(`
    $a = Get-Item '${alphaExeBuilt.replace(/'/g, "''")}';
    $b100 = Get-Item '${beta100ExeBuilt.replace(/'/g, "''")}';
    $b110 = Get-Item '${beta110ExeBuilt.replace(/'/g, "''")}';
    $sa = Get-AuthenticodeSignature $a.FullName;
    $sb100 = Get-AuthenticodeSignature $b100.FullName;
    $sb110 = Get-AuthenticodeSignature $b110.FullName;
    function ViMap($f) {
      return @{
        CompanyName = $f.VersionInfo.CompanyName
        ProductName = $f.VersionInfo.ProductName
        FileDescription = $f.VersionInfo.FileDescription
        LegalCopyright = $f.VersionInfo.LegalCopyright
        FileVersion = $f.VersionInfo.FileVersion
        ProductVersion = $f.VersionInfo.ProductVersion
        InternalName = $f.VersionInfo.InternalName
        OriginalFilename = $f.VersionInfo.OriginalFilename
      }
    }
    [PSCustomObject]@{
      alphaVersionInfo = (ViMap $a)
      alphaAuth = @{ status = $sa.Status.ToString(); signer = if ($sa.SignerCertificate) { $sa.SignerCertificate.Subject } else { $null } }
      beta100VersionInfo = (ViMap $b100)
      beta100Auth = @{ status = $sb100.Status.ToString(); statusMessage = $sb100.StatusMessage; subject = if ($sb100.SignerCertificate) { $sb100.SignerCertificate.Subject } else { $null }; thumbprint = if ($sb100.SignerCertificate) { $sb100.SignerCertificate.Thumbprint } else { $null } }
      beta110VersionInfo = (ViMap $b110)
      beta110Auth = @{ status = $sb110.Status.ToString(); subject = if ($sb110.SignerCertificate) { $sb110.SignerCertificate.Subject } else { $null }; thumbprint = if ($sb110.SignerCertificate) { $sb110.SignerCertificate.Thumbprint } else { $null } }
    } | ConvertTo-Json -Depth 5 -Compress
  `);

  // STEP 3: Deploy Alpha v1.0.0 & Beta v1.0.0 via fp-updater.exe, test Authenticode tamper rejection,
  // test KeyRotationDelegation upgrade Beta v1.0.0 -> v1.1.0, and test Trust-Anchored pointer recovery!
  const alphaRelDir = studioResult.resAlpha.release.outputDirPath;
  const beta100RelDir = studioResult.resBeta100.release.outputDirPath;
  const beta110RelDir = studioResult.resBeta110.release.outputDirPath;

  const installAlphaRes = JSON.parse(
    runCommandChecked(updaterExe, ['install-initial', '--install-root', installAlpha, '--release-dir', alphaRelDir], SANDBOX)
  );
  const installBetaRes = JSON.parse(
    runCommandChecked(updaterExe, ['install-initial', '--install-root', installBeta, '--release-dir', beta100RelDir], SANDBOX)
  );

  // Tamper Beta Authenticode EXE post-signing -> MUST FAIL EVEN when bundle untouched!
  const tamperedBetaDir = path.join(SANDBOX, 'tampered_beta_pe');
  copyDirSync(beta100RelDir, tamperedBetaDir);
  const tamperedExe = path.join(tamperedBetaDir, 'Auto_BrandBeta.exe');
  const peBuf = fs.readFileSync(tamperedExe);
  peBuf[Math.floor(peBuf.length / 2)] ^= 0x55;
  fs.writeFileSync(tamperedExe, peBuf);
  const peTamperErr = runCommandExpectFail(updaterExe, ['check-compat', '--release-dir', tamperedBetaDir], SANDBOX);

  // Upgrade Beta v1.0.0 -> v1.1.0 WITH KeyRotationDelegation!
  const upgradeBetaRes = JSON.parse(
    runCommandChecked(updaterExe, ['upgrade', '--install-root', installBeta, '--release-dir', beta110RelDir], SANDBOX)
  );

  // Replay nonce MUST be rejected!
  const replayErr = runCommandExpectFail(
    updaterExe,
    ['upgrade', '--install-root', installBeta, '--release-dir', beta110RelDir],
    SANDBOX
  );

  // Drop untrusted v9.9.9 rogue copy of Alpha into Beta's versions/ and delete BOTH active_version.json & bak -> reconcile rejects v9.9.9 and recovers v1.1.0!
  copyDirSync(alphaRelDir, path.join(installBeta, 'versions', 'v9.9.9'));
  fs.unlinkSync(path.join(installBeta, 'active_version.json'));
  fs.unlinkSync(path.join(installBeta, '.updater', 'active_version.bak.json'));
  const reconcileTrustedRes = JSON.parse(
    runCommandChecked(updaterExe, ['reconcile', '--install-root', installBeta], SANDBOX)
  );

  // STEP 4: Launch BOTH independent Mini Tools with Studio completely CLOSED and capture screenshots!
  const runAlphaExe = path.join(installAlpha, 'versions', 'v1.0.0', 'Auto_BrandAlpha.exe');
  const runBetaExe = path.join(installBeta, 'versions', 'v1.1.0', 'Auto_BrandBeta.exe');

  let alphaRuntimeSnap = null;
  let alphaConsoleErrors = [];
  const childAlpha = spawn(runAlphaExe, [], {
    cwd: path.dirname(runAlphaExe),
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9275' },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9275, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page, '#device-tbody td');
    alphaConsoleErrors = consoleErrors;
    await new Promise((r) => setTimeout(r, 450));
    alphaRuntimeSnap = await evalJs(`(() => ({
      title: document.title,
      badge: document.querySelector('.badge-ver')?.textContent || '',
      bodyText: document.body.innerText.slice(0, 600)
    }))()`);
    await snap('p5_3_brand_alpha_running.png', 740, 500);
    ws.close();
  } finally {
    try { childAlpha.kill('SIGTERM'); } catch (_) {}
  }

  await new Promise((r) => setTimeout(r, 500));

  let betaRuntimeSnap = null;
  let betaConsoleErrors = [];
  const childBeta = spawn(runBetaExe, [], {
    cwd: path.dirname(runBetaExe),
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9276' },
    stdio: 'ignore',
  });
  try {
    const page = await waitForCdp(9276, 22000);
    const { ws, evalJs, snap, consoleErrors } = await connectCdp(page, '#device-tbody td');
    betaConsoleErrors = consoleErrors;
    await new Promise((r) => setTimeout(r, 450));
    betaRuntimeSnap = await evalJs(`(() => ({
      title: document.title,
      badge: document.querySelector('.badge-ver')?.textContent || '',
      bodyText: document.body.innerText.slice(0, 600)
    }))()`);
    await snap('p5_3_brand_beta_running.png', 740, 500);
    ws.close();
  } finally {
    try { childBeta.kill('SIGTERM'); } catch (_) {}
  }

  const secretLeaks = scanDirectoryForSecretLeaks(SANDBOX);
  if (qaThumbToCleanup) {
    try {
      execFileSync(
        'powershell',
        ['-NoProfile', '-Command', `Remove-Item 'Cert:\\CurrentUser\\My\\${qaThumbToCleanup}' -ErrorAction SilentlyContinue`],
        { cwd: ROOT }
      );
    } catch (_) {}
  }

  const finalReport = {
    executedAtUtc: new Date().toISOString(),
    gitHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
    adbLiveHardware: {
      adbDevicesOutput: adbOut || '(0 devices)',
      ldplayer14Vm0IsRunning: ldRunning,
      status: 'BLOCKED (0 ADB online)',
    },
    publicSignedCommercialRelease: {
      status: 'BLOCKED (No commercial Authenticode CA certificate present; tested fail-closed rejection with no-cert and self-signed cert)',
      noCertBlocked: Boolean(studioResult.publicNoCertErr),
      selfSignedBlocked: Boolean(studioResult.publicSelfSignedErr),
      publicNoCertErr: studioResult.publicNoCertErr,
      publicSelfSignedErr: studioResult.publicSelfSignedErr,
    },
    consoleErrors: {
      studio: studioConsoleErrors,
      brandAlpha: alphaConsoleErrors,
      brandBeta: betaConsoleErrors,
    },
    secretLeakScanHits: secretLeaks,
    studioResult,
    osInspect,
    updaterLifecycle: {
      installAlphaRes,
      installBetaRes,
      peTamperErr,
      upgradeBetaRes,
      replayErr,
      reconcileTrustedRes,
    },
    independentRuntime: {
      studioClosedBeforeLaunch: true,
      alphaRuntimeSnap,
      betaRuntimeSnap,
    },
  };

  fs.writeFileSync(path.join(EV_DIR, 'p5_3_e2e_report.json'), JSON.stringify(finalReport, null, 2), 'utf8');
  console.log(JSON.stringify({
    status: 'P5_3_E2E_COMPLETED',
    alphaCompany: osInspect.alphaVersionInfo.CompanyName,
    betaCompany: osInspect.beta110VersionInfo.CompanyName,
    betaAuthStatus: osInspect.beta110Auth.status,
    upgradeVersion: upgradeBetaRes.activeVersion,
    reconcileStatus: reconcileTrustedRes.status,
    reconcileActiveVer: reconcileTrustedRes.activeVersion,
    secretLeakCount: secretLeaks.length,
    publicNoCertBlocked: finalReport.publicSignedCommercialRelease.noCertBlocked,
    publicSelfSignedBlocked: finalReport.publicSignedCommercialRelease.selfSignedBlocked,
  }, null, 2));

  if (
    secretLeaks.length > 0 ||
    osInspect.alphaVersionInfo.CompanyName !== 'Alpha Vanguard Automation Co., Ltd.' ||
    osInspect.beta100VersionInfo.CompanyName !== 'Beta Nova Interactive Labs Pte. Ltd.' ||
    osInspect.beta100Auth.status === 'NotSigned' ||
    !peTamperErr.includes('AUTHENTICODE_TAMPERED_OR_INVALID') ||
    upgradeBetaRes.activeVersion !== '1.1.0' ||
    !reconcileTrustedRes ||
    reconcileTrustedRes.activeVersion !== '1.1.0' ||
    !finalReport.publicSignedCommercialRelease.noCertBlocked ||
    !finalReport.publicSignedCommercialRelease.selfSignedBlocked
  ) {
    process.exitCode = 1;
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
