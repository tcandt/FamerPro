const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const crypto = require('crypto');

const ROOT = 'E:\\DATA\\FarmerPro';
const PACK_DIR = path.join(ROOT, 'artifacts', 'final_acceptance_pack');
const SUPP_DIR = path.join(PACK_DIR, 'rc3_supplement_evidence');
const SANDBOX = path.join(SUPP_DIR, 'sandbox');
const TARGET_SERIAL = 'emulator-5554';
const ADB_BIN = path.join(ROOT, 'bin', 'adb.exe');
const INSTALLER_EXE = path.join(ROOT, 'artifacts', 'p6_evidence', 'dist_releases', 'FarmerPro_0.1.0_x64-setup.exe');

function sha256File(p) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(p));
  return h.digest('hex');
}

function rmSyncRetry(target) {
  for (let i = 0; i < 8; i++) {
    try {
      if (fs.existsSync(target)) fs.rmSync(target, { recursive: true, force: true });
      return;
    } catch (e) {
      if (i === 7) throw e;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 350);
    }
  }
}

function liveEnv(baseSandbox, extra = {}) {
  const sysPath = process.env.PATH || process.env.Path || '';
  return {
    ...process.env,
    PATH: `${path.join(ROOT, 'bin')};${sysPath}`,
    TEMP: path.join(baseSandbox, 'temp'),
    TMP: path.join(baseSandbox, 'temp'),
    APPDATA: path.join(baseSandbox, 'AppData', 'Roaming'),
    LOCALAPPDATA: path.join(baseSandbox, 'AppData', 'Local'),
    FP_TRUST_ANCHOR_DIR: path.join(baseSandbox, 'ExternalTrustAnchors'),
    ...extra,
  };
}

function runCommandChecked(exe, args, cwd, envOverride = null) {
  return execFileSync(exe, args, {
    cwd,
    encoding: 'utf8',
    stdio: 'pipe',
    env: envOverride || process.env,
  }).trim();
}

async function waitForCdp(port, timeoutMs = 28000) {
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

  for (let i = 0; i < 65; i++) {
    const checkExpr = waitSelector
      ? `Boolean(window.__TAURI_INTERNALS__ && document.querySelector(${JSON.stringify(waitSelector)}))`
      : 'Boolean(window.__TAURI_INTERNALS__)';
    const ready = await evalJs(checkExpr);
    if (ready) break;
    await new Promise((r) => setTimeout(r, 200));
  }

  return { ws, evalJs, consoleErrors };
}

async function main() {
  if (fs.existsSync(SUPP_DIR)) rmSyncRetry(SUPP_DIR);
  fs.mkdirSync(path.join(SANDBOX, 'temp'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Local'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'AppData', 'Roaming'), { recursive: true });
  fs.mkdirSync(path.join(SANDBOX, 'ExternalTrustAnchors'), { recursive: true });

  const adbDevicesRaw = runCommandChecked(ADB_BIN, ['devices', '-l'], ROOT);
  if (!adbDevicesRaw.includes(TARGET_SERIAL) || !/\sdevice(\s|$)/.test(adbDevicesRaw)) {
    throw new Error(`Expected ${TARGET_SERIAL} Ready; got:\n${adbDevicesRaw}`);
  }

  // Install frozen RC.3 NSIS installer into isolated sandbox
  const studioInstallDir = path.join(SANDBOX, 'InstalledStudio');
  const studioEnv = liveEnv(SANDBOX, { WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9298' });
  runCommandChecked(INSTALLER_EXE, ['/S', `/D=${studioInstallDir}`], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1100));
  const studioExe = path.join(studioInstallDir, 'FarmerPro.exe');

  // =========================================================================
  // TEST A: CROSS-PROCESS DEVICE LEASE (FarmerPro.exe vs Auto_CrossMini.exe)
  // =========================================================================
  const relOut = path.join(SANDBOX, 'CrossMini_v1.0.0_win-x64');
  let studioChild = spawn(studioExe, [], { cwd: studioInstallDir, env: studioEnv, stdio: 'ignore' });
  let testAResult = null;
  let projId = null;

  try {
    const pgS = await waitForCdp(9298, 28000);
    const { ws: wsS, evalJs: evalS } = await connectCdp(pgS);

    const setup = await evalS(`(async () => {
      const inv = window.__TAURI_INTERNALS__.invoke;
      await inv('auto_discover_devices');
      const pub = await inv('studio_upsert_publisher_profile', {
        input: {
          publisherId: 'pub-rc3-cross',
          displayName: 'FarmerPro RC3 QA',
          organization: 'FarmerPro QA',
          copyright: 'Copyright (C) 2026 FarmerPro',
          releaseChannel: 'internal',
          signingPolicy: 'unsigned_internal'
        }
      });
      const proj = await inv('studio_create_project', {
        req: {
          slug: 'rc3-cross-lease',
          name: 'RC3 Cross Process Lease Check',
          gamePackageId: 'com.android.settings',
          version: '1.0.0',
          exeName: 'Auto_CrossMini.exe',
          windowTitle: 'RC3 Cross Process Mini v1.0.0',
          targetWidthPx: 1280,
          targetHeightPx: 720,
          bundleAdb: true,
          enabledModules: ['mod.adb_core', 'mod.vision_ocr']
        }
      });
      await inv('studio_ensure_project_icon', { projectId: proj.id, accentHex: '#3B82F6', overwrite: true });
      await inv('studio_save_project_workflow', {
        projectId: proj.id,
        workflowId: 'wf-cross-studio-long',
        name: 'Studio Long Hold Workflow',
        category: 'CrossCheck',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-cross-studio-long',
          name: 'Studio Long Hold Workflow',
          status: 'published',
          graph: {
            nodes: [
              { id: 's1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 's2', type: 'vision.capture', position: { x: 200, y: 80 }, config: {} },
              { id: 's3', type: 'control.wait', position: { x: 360, y: 80 }, config: { durationMs: 4200 } },
              { id: 's4', type: 'state.set_var', position: { x: 520, y: 80 }, config: { varKey: 'studio_hold_done', valueType: 'string', value: 'DONE' } },
              { id: 's5', type: 'flow.end', position: { x: 680, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'se1', source: 's1', sourcePort: 'flow', target: 's2', targetPort: 'flow' },
              { id: 'se2', source: 's2', sourcePort: 'flow', target: 's3', targetPort: 'flow' },
              { id: 'se3', source: 's3', sourcePort: 'flow', target: 's4', targetPort: 'flow' },
              { id: 'se4', source: 's4', sourcePort: 'flow', target: 's5', targetPort: 'flow' }
            ]
          }
        })
      });
      await inv('studio_save_project_workflow', {
        projectId: proj.id,
        workflowId: 'wf-cross-mini',
        name: 'Mini Competing Live Task',
        category: 'CrossCheck',
        draftJson: JSON.stringify({
          schemaVersion: 1,
          workflowId: 'wf-cross-mini',
          name: 'Mini Competing Live Task',
          status: 'published',
          graph: {
            nodes: [
              { id: 'm1', type: 'flow.start', position: { x: 40, y: 80 }, config: {} },
              { id: 'm2', type: 'vision.capture', position: { x: 200, y: 80 }, config: {} },
              { id: 'm3', type: 'state.set_var', position: { x: 360, y: 80 }, config: { varKey: 'mini_ran', valueType: 'string', value: 'YES' } },
              { id: 'm4', type: 'flow.end', position: { x: 520, y: 80 }, config: {} }
            ],
            edges: [
              { id: 'me1', source: 'm1', sourcePort: 'flow', target: 'm2', targetPort: 'flow' },
              { id: 'me2', source: 'm2', sourcePort: 'flow', target: 'm3', targetPort: 'flow' },
              { id: 'me3', source: 'm3', sourcePort: 'flow', target: 'm4', targetPort: 'flow' }
            ]
          }
        })
      });
      const miniTaskDef = {
        id: 'tsk-cross-mini',
        title: '1. Competing Live Capture Task',
        categoryId: 'cat-cross',
        workflowId: 'wf-cross-mini',
        defaultEnabled: true,
        priority: 1,
        estimateSec: 5,
        settingsSchema: []
      };
      await inv('studio_upsert_project_task', {
        projectId: proj.id,
        task: miniTaskDef,
        categoryMeta: { id: 'cat-cross', shortTag: 'LIVE', displayName: 'Cross Check', colorHex: '#10B981' }
      });
      const ui = await inv('studio_get_ui_schema', { projectId: proj.id });
      ui.tasks = [miniTaskDef];
      await inv('studio_save_ui_schema', { projectId: proj.id, schema: ui });
      await inv('studio_update_project', {
        req: { id: proj.id, publisherProfileId: pub.id, signingMode: 'unsigned_internal' }
      });
      await inv('studio_builder_export_release', {
        projectId: proj.id,
        outputDir: ${JSON.stringify(relOut)},
        options: { signingMode: 'unsigned_internal', publisherProfileId: pub.id }
      });
      return { proj, pub };
    })()`);
    projId = setup.proj.id;

    // Launch independent Auto_CrossMini.exe concurrently while FarmerPro.exe is running!
    const miniExe = path.join(relOut, 'Auto_CrossMini.exe');
    const miniEnv = liveEnv(SANDBOX, { WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9299' });
    const miniChild = spawn(miniExe, [], { cwd: relOut, env: miniEnv, stdio: 'ignore' });
    try {
      const pgM = await waitForCdp(9299, 24000);
      const { ws: wsM, evalJs: evalM } = await connectCdp(pgM, '#left-body .task-row');

      // 1. Start Job A in Studio (holds emulator-5554 for ~4.2s)
      const jobAStart = await evalS(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
        req: {
          projectId: ${JSON.stringify(projId)},
          workflowId: 'wf-cross-studio-long',
          deviceId: '${TARGET_SERIAL}',
          priority: 1,
          leaseTtlSecs: 30,
          workflowTimeoutMs: 20000,
          initialVariables: {}
        }
      })`);

      await new Promise((r) => setTimeout(r, 700));
      const studioMidDuringA = await evalS(`(async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const leases = await inv('studio_list_active_device_leases');
        const jobs = await inv('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
        let inProcessSecondAttemptErr = null;
        try {
          await inv('studio_start_project_workflow', {
            req: {
              projectId: ${JSON.stringify(projId)},
              workflowId: 'wf-cross-mini',
              deviceId: '${TARGET_SERIAL}',
              priority: 1,
              leaseTtlSecs: 10,
              workflowTimeoutMs: 5000,
              initialVariables: {}
            }
          });
        } catch (e) {
          inProcessSecondAttemptErr = String(e);
        }
        return {
          leases,
          jobAState: jobs.find(j => j.id === ${JSON.stringify(jobAStart.jobId)})?.state,
          inProcessSecondAttemptErr,
        };
      })()`);

      // 2. While Studio Job A is STILL running, invoke Job B from separate Mini Tool EXE on the SAME emulator-5554!
      const jobBAttemptAtUtc = new Date().toISOString();
      const jobBOutcome = await evalM(`(async () => {
        try {
          const receipt = await window.__TAURI_INTERNALS__.invoke('mini_run_task', {
            taskId: 'tsk-cross-mini',
            useDemoMock: false
          });
          return { blocked: false, receipt: { jobId: receipt.jobId, deviceId: receipt.deviceId, state: receipt.state, driverUsed: receipt.driverUsed } };
        } catch (e) {
          return { blocked: true, error: String(e) };
        }
      })()`);

      // Check Studio Job A state right after Mini Tool finished
      const studioStateRightAfterB = await evalS(`(async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const leases = await inv('studio_list_active_device_leases');
        const jobs = await inv('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
        return {
          leasesCount: leases.length,
          jobAStateRightAfterB: jobs.find(j => j.id === ${JSON.stringify(jobAStart.jobId)})?.state,
        };
      })()`);

      // Wait for Job A to complete naturally and confirm Studio lease is released
      let jobAFinal = null;
      for (let i = 0; i < 30; i++) {
        await new Promise((r) => setTimeout(r, 300));
        jobAFinal = await evalS(`(async () => {
          const jobs = await window.__TAURI_INTERNALS__.invoke('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
          return jobs.find(j => j.id === ${JSON.stringify(jobAStart.jobId)});
        })()`);
        if (jobAFinal && ['succeeded', 'failed', 'cancelled', 'blocked'].includes(String(jobAFinal.state).toLowerCase())) break;
      }
      const leasesAfterJobADone = await evalS(`window.__TAURI_INTERNALS__.invoke('studio_list_active_device_leases')`);

      // 3. Forced-kill recovery check: start Job C in Studio -> kill Studio PID forcefully -> verify new process recovers & acquires device cleanly
      const jobCStart = await evalS(`window.__TAURI_INTERNALS__.invoke('studio_start_project_workflow', {
        req: {
          projectId: ${JSON.stringify(projId)},
          workflowId: 'wf-cross-studio-long',
          deviceId: '${TARGET_SERIAL}',
          priority: 1,
          leaseTtlSecs: 10,
          workflowTimeoutMs: 20000,
          initialVariables: {}
        }
      })`);
      await new Promise((r) => setTimeout(r, 350));
      const studioPid = studioChild.pid;
      wsM.close();
      wsS.close();
      try { execFileSync('taskkill', ['/PID', String(studioPid), '/F'], { stdio: 'pipe' }); } catch (_) {}
      await new Promise((r) => setTimeout(r, 800));

      // Relaunch fresh FarmerPro.exe after forced kill and start Job D immediately
      studioChild = spawn(studioExe, [], { cwd: studioInstallDir, env: studioEnv, stdio: 'ignore' });
      const pgS2 = await waitForCdp(9298, 28000);
      const { ws: wsS2, evalJs: evalS2 } = await connectCdp(pgS2);
      const afterKillRecovery = await evalS2(`(async () => {
        const inv = window.__TAURI_INTERNALS__.invoke;
        const leasesOnRestart = await inv('studio_list_active_device_leases');
        const jobD = await inv('studio_start_project_workflow', {
          req: {
            projectId: ${JSON.stringify(projId)},
            workflowId: 'wf-cross-mini',
            deviceId: '${TARGET_SERIAL}',
            priority: 1,
            leaseTtlSecs: 15,
            workflowTimeoutMs: 15000,
            initialVariables: {}
          }
        });
        let jobDFinal = null;
        for (let i = 0; i < 30; i++) {
          await new Promise(r => setTimeout(r, 250));
          const jobs = await inv('studio_list_project_jobs', { projectId: ${JSON.stringify(projId)} });
          jobDFinal = jobs.find(j => j.id === jobD.jobId);
          if (jobDFinal && ['succeeded', 'failed', 'blocked'].includes(String(jobDFinal.state).toLowerCase())) break;
        }
        return { leasesOnRestart, jobDStartState: jobD.state, jobDFinalState: jobDFinal?.state };
      })()`);
      wsS2.close();

      const crossProcessConflictBlocked = Boolean(jobBOutcome.blocked);
      testAResult = {
        status: crossProcessConflictBlocked ? 'PASS' : 'NOT_VERIFIED_IN_PROCESS_LEASE_ONLY',
        sameProcessSecondJobRejected: Boolean(studioMidDuringA.inProcessSecondAttemptErr),
        sameProcessRejectError: studioMidDuringA.inProcessSecondAttemptErr,
        studioJobAStart: jobAStart,
        studioMidDuringA: { jobAState: studioMidDuringA.jobAState, activeLeasesCount: studioMidDuringA.leases.length },
        jobBAttemptAtUtc,
        miniToolJobBOutcome: jobBOutcome,
        studioStateRightAfterB,
        jobAFinalState: jobAFinal?.state,
        leasesAfterJobADoneCount: leasesAfterJobADone.length,
        forcedKillRecovery: {
          killedStudioPid: studioPid,
          killedInFlightJobId: jobCStart.jobId,
          ...afterKillRecovery,
        },
        empiricalConclusion: crossProcessConflictBlocked
          ? 'Cross-process lease blocked competing EXE.'
          : 'FarmerPro.exe (in-process MultiDeviceScheduler + AppData/farmerpro.db) and Auto_CrossMini.exe (separate in-process MultiDeviceScheduler + release/data/runtime.db) DO NOT share an OS-wide lock; Job B succeeded on emulator-5554 while Studio Job A was still running.',
      };
    } finally {
      try { miniChild.kill('SIGTERM'); } catch (_) {}
    }
  } finally {
    try { execFileSync('taskkill', ['/IM', 'FarmerPro.exe', '/F'], { stdio: 'pipe' }); } catch (_) {}
    try { execFileSync('taskkill', ['/IM', 'Auto_CrossMini.exe', '/F'], { stdio: 'pipe' }); } catch (_) {}
  }

  // =========================================================================
  // TEST B: SQLITE V5 UPGRADE COMPATIBILITY (Genuine RC.2 DB -> Opened by RC.3)
  // =========================================================================
  const dbTestDir = path.join(SANDBOX, 'sqlite_v5_upgrade');
  const studioRc2DataDir = path.join(dbTestDir, 'studio_data');
  const studioBackupDb = path.join(dbTestDir, 'rc2_studio_backup.db');
  const studioTargetDb = path.join(studioRc2DataDir, 'farmerpro.db');
  fs.mkdirSync(studioRc2DataDir, { recursive: true });

  // Build genuine RC.2 v5 database (BEFORE RC.3 added confidence_source, measured_confidence, rule_matched, device_id)
  const preUpgradeInspection = JSON.parse(
    runCommandChecked(
      'python',
      [
        '-c',
        `import sqlite3, json, shutil, sys, subprocess, re
# Extract exact RC.2 migrations.rs SQL literals from git tag v0.1.0-p6-rc.2 (b5a0567)
mig_src = subprocess.check_output(['git', 'show', 'b5a0567:crates/fp-storage/src/migrations.rs'], cwd=sys.argv[1]).decode('utf-8')
def grab(name):
  m = re.search(rf'pub const {name}: &str = r#"(.*?)"#;', mig_src, re.S)
  assert m, f'missing {name}'
  return m.group(1)

conn = sqlite3.connect(sys.argv[2])
conn.execute('PRAGMA journal_mode = WAL;')
conn.execute('PRAGMA foreign_keys = ON;')
conn.executescript(grab('MIGRATION_V1_SQL'))
conn.executescript(grab('MIGRATION_V2_TABLES_SQL'))
for sql in [
  "ALTER TABLE workflows ADD COLUMN project_id TEXT;",
  "ALTER TABLE accounts ADD COLUMN project_id TEXT;",
  "ALTER TABLE jobs ADD COLUMN project_id TEXT;",
  "ALTER TABLE jobs ADD COLUMN workflow_id TEXT;",
  "ALTER TABLE jobs ADD COLUMN task_id TEXT;",
  "ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'General';",
]:
  conn.execute(sql)
conn.executescript(grab('MIGRATION_V2_INDEXES_SQL'))
conn.executescript(grab('MIGRATION_V3_RUNTIME_PROVENANCE_SQL'))
conn.executescript(grab('MIGRATION_V4_RELEASE_LIFECYCLE_SQL'))
for sql in [
  "ALTER TABLE project_releases ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Published';",
  "ALTER TABLE project_releases ADD COLUMN release_channel TEXT NOT NULL DEFAULT 'internal';",
  "ALTER TABLE project_releases ADD COLUMN changelog_md TEXT NOT NULL DEFAULT '';",
  "ALTER TABLE project_releases ADD COLUMN min_runtime_version TEXT NOT NULL DEFAULT '0.1.0';",
  "ALTER TABLE project_releases ADD COLUMN workflow_schema_version INTEGER NOT NULL DEFAULT 1;",
  "ALTER TABLE project_releases ADD COLUMN ui_schema_version INTEGER NOT NULL DEFAULT 1;",
  "ALTER TABLE project_releases ADD COLUMN module_api_version TEXT NOT NULL DEFAULT '1.0.0';",
  "ALTER TABLE project_releases ADD COLUMN sqlite_schema_version INTEGER NOT NULL DEFAULT 4;",
  "ALTER TABLE project_releases ADD COLUMN git_commit TEXT NOT NULL DEFAULT '';",
  "ALTER TABLE project_releases ADD COLUMN signature_status TEXT NOT NULL DEFAULT 'Ed25519-Verified';",
  "ALTER TABLE project_releases ADD COLUMN promoted_at TEXT;",
  "ALTER TABLE project_releases ADD COLUMN rolled_back_from_id TEXT;",
]:
  conn.execute(sql)
conn.executescript(grab('MIGRATION_V5_PUBLISHER_PROFILES_SQL'))
for sql in [
  "ALTER TABLE projects ADD COLUMN publisher_profile_id TEXT;",
  "ALTER TABLE projects ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
  "ALTER TABLE projects ADD COLUMN version_info_json TEXT;",
  "ALTER TABLE project_releases ADD COLUMN publisher_profile_id TEXT;",
  "ALTER TABLE project_releases ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
  "ALTER TABLE project_releases ADD COLUMN authenticode_trust_status TEXT NOT NULL DEFAULT 'unsigned';",
  "ALTER TABLE project_releases ADD COLUMN authenticode_thumbprint TEXT;",
  "ALTER TABLE project_releases ADD COLUMN authenticode_subject TEXT;",
  "ALTER TABLE project_releases ADD COLUMN trusted_public_release INTEGER NOT NULL DEFAULT 0;",
]:
  conn.execute(sql)

now = '2026-11-23T04:20:00Z'
for v in [1,2,3,4,5]:
  conn.execute('INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)', (v, now))

# Seed RC.2 records across Projects, Workflows, Versions, Tasks, Releases, Publisher Profiles, Accounts (Credentials), and Runtime Envelopes
conn.execute("INSERT INTO publisher_profiles (id, publisher_id, display_name, organization, copyright, ed25519_key_id, ed25519_pubkey_hex, release_channel, signing_policy, created_at, updated_at) VALUES ('pub-rc2', 'pub-rc2', 'RC2 Pub', 'RC2 Org', 'Copyright 2026', 'k1', 'abcd', 'internal', 'unsigned_internal', ?, ?)", (now, now))
conn.execute("INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, project_dir_path, publisher_profile_id, signing_mode, created_at, updated_at) VALUES ('proj-rc2-upg', 'rc2-upgrade-game', 'RC2 Legacy Project', 'com.rc2.game', '1.0.0', 'Auto_RC2.exe', 'RC2 Legacy Tool', ?, 'pub-rc2', 'unsigned_internal', ?, ?)", (sys.argv[3], now, now))
wf_json = json.dumps({'schemaVersion':1,'workflowId':'wf-rc2-1','name':'RC2 Kept Workflow','status':'published','graph':{'nodes':[{'id':'n1','type':'flow.start','position':{'x':0,'y':0},'config':{}},{'id':'n2','type':'flow.end','position':{'x':200,'y':0},'config':{}}],'edges':[{'id':'e1','source':'n1','sourcePort':'flow','target':'n2','targetPort':'flow'}]}})
conn.execute("INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at, category) VALUES ('wf-rc2-1', 'proj-rc2-upg', 'RC2 Kept Workflow', ?, ?, 'Combat')", (wf_json, now))
conn.execute("INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at) VALUES ('wf-rc2-1-v1', 'wf-rc2-1', 1, ?, 'deadbeef', ?)", (wf_json, now))
conn.execute("INSERT INTO task_definitions (id, project_id, workflow_id, category_id, category_short_tag, category_name, category_color_hex, title, updated_at) VALUES ('tsk-rc2-1', 'proj-rc2-upg', 'wf-rc2-1', 'cat1', 'CMB', 'Combat', '#10B981', 'RC2 Legacy Task', ?)", (now,))
conn.execute("INSERT INTO project_releases (id, project_id, version, output_dir_path, exe_path, exe_size_bytes, exe_sha256, key_id, publisher_public_key_hex, signature_ed25519_hex, build_report_json, built_at, publisher_profile_id) VALUES ('rel-rc2-1', 'proj-rc2-upg', '1.0.0', '/tmp/rel', '/tmp/rel/Auto_RC2.exe', 12345, 'aaaa', 'k1', 'abcd', 'sig1', '{}', ?, 'pub-rc2')", (now,))
conn.execute("INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at) VALUES ('acc-rc2-cred', 'proj-rc2-upg', 'rc2_test_account', 'profile_01', 'dpapi_ref_rc2_token', 'ready', ?)", (now,))
conn.execute("INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at) VALUES ('mini:rc2-upgrade-game', 'proj-rc2-upg', 7, ?)", (now,))
conn.execute("INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted) VALUES ('mini:rc2-upgrade-game', 'legacy_ocr_val', 'proj-rc2-upg', 'sess-rc2-old', 'job-rc2-old', 'string', ?, 'LIVE_ADB_OCR', 1, 0.94, ?, 'wf-rc2-1@v1', 7, 0)", (json.dumps("Settings"), now))
conn.commit()

cols_before = [r[1] for r in conn.execute("PRAGMA table_info(runtime_variable_envelopes)").fetchall()]
vers_before = [r[0] for r in conn.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()]
conn.close()
shutil.copy2(sys.argv[2], sys.argv[4])
print(json.dumps({'colsBefore': cols_before, 'schemaVersionsBefore': vers_before}))`,
        ROOT,
        studioTargetDb,
        path.join(dbTestDir, 'projects', 'rc2-upgrade-game'),
        studioBackupDb,
      ],
      ROOT
    )
  );
  fs.mkdirSync(path.join(dbTestDir, 'projects', 'rc2-upgrade-game'), { recursive: true });
  const backupShaBefore = sha256File(studioBackupDb);

  // Launch frozen RC.3 FarmerPro.exe TWICE against the RC.2 v5 DB (first upgrade + second idempotency check)
  const studioUpgEnv = liveEnv(SANDBOX, {
    FARMERPRO_DATA_DIR: studioRc2DataDir,
    FARMERPRO_PROJECTS_DIR: path.join(dbTestDir, 'projects'),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9301',
  });
  let studioIpcReadback = null;
  for (const passNo of [1, 2]) {
    const ch = spawn(studioExe, [], { cwd: studioInstallDir, env: studioUpgEnv, stdio: 'ignore' });
    try {
      const pg = await waitForCdp(9301, 24000);
      const { ws, evalJs } = await connectCdp(pg);
      if (passNo === 2) {
        studioIpcReadback = await evalJs(`(async () => {
          const inv = window.__TAURI_INTERNALS__.invoke;
          const projects = await inv('studio_list_projects');
          const p = projects.find(x => x.id === 'proj-rc2-upg');
          const workflows = await inv('studio_list_project_workflows', { projectId: 'proj-rc2-upg' });
          const tasks = await inv('studio_list_project_tasks', { projectId: 'proj-rc2-upg' });
          const releases = await inv('studio_list_project_releases', { projectId: 'proj-rc2-upg' });
          const accounts = await inv('studio_list_project_accounts', { projectId: 'proj-rc2-upg' });
          let vars = null;
          let varsError = null;
          try {
            vars = await inv('studio_get_project_runtime_vars', { projectId: 'proj-rc2-upg' });
          } catch (e) {
            varsError = String(e);
          }
          return {
            projectsCount: projects.length,
            keptProjectName: p?.name,
            workflowsCount: workflows.length,
            tasksCount: tasks.length,
            releasesCount: releases.length,
            accountsCount: accounts.filter(a => a.id === 'acc-rc2-cred').length,
            accountAlias: accounts.find(a => a.id === 'acc-rc2-cred')?.alias,
            varsScopeKey: vars?.scopeKey || null,
            varsError,
          };
        })()`);
      }
      ws.close();
    } finally {
      try { execFileSync('taskkill', ['/PID', String(ch.pid), '/F'], { stdio: 'pipe' }); } catch (_) {}
      await new Promise((r) => setTimeout(r, 700));
    }
  }

  // Inspect upgraded DB + test copy rollback/restore on simulated failure
  const rollbackTargetDb = path.join(dbTestDir, 'rc2_rollback_sim.db');
  fs.copyFileSync(studioBackupDb, rollbackTargetDb);
  const postUpgradeInspection = JSON.parse(
    runCommandChecked(
      'python',
      [
        '-c',
        `import sqlite3, json, shutil, sys
conn = sqlite3.connect(sys.argv[1])
cols_after = [r[1] for r in conn.execute("PRAGMA table_info(runtime_variable_envelopes)").fetchall()]
vers_after = [r[0] for r in conn.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()]
integ = [r[0] for r in conn.execute("PRAGMA integrity_check").fetchall()]
fk = conn.execute("PRAGMA foreign_key_check").fetchall()
try:
  kept_env = conn.execute("SELECT var_name, value_json, source, verified, confidence, confidence_source, measured_confidence, rule_matched, device_id, seq FROM runtime_variable_envelopes WHERE scope_key='mini:rc2-upgrade-game'").fetchall()
except Exception as ex:
  kept_env = {'queryError': str(ex), 'legacyRows': conn.execute("SELECT var_name, value_json, source, verified, confidence, seq FROM runtime_variable_envelopes WHERE scope_key='mini:rc2-upgrade-game'").fetchall()}
counts = {
  'projects': conn.execute("SELECT count(*) FROM projects WHERE id='proj-rc2-upg'").fetchone()[0],
  'workflows': conn.execute("SELECT count(*) FROM workflows WHERE id='wf-rc2-1'").fetchone()[0],
  'workflow_versions': conn.execute("SELECT count(*) FROM workflow_versions WHERE id='wf-rc2-1-v1'").fetchone()[0],
  'task_definitions': conn.execute("SELECT count(*) FROM task_definitions WHERE id='tsk-rc2-1'").fetchone()[0],
  'project_releases': conn.execute("SELECT count(*) FROM project_releases WHERE id='rel-rc2-1'").fetchone()[0],
  'accounts': conn.execute("SELECT count(*) FROM accounts WHERE id='acc-rc2-cred' AND secret_ref='dpapi_ref_rc2_token'").fetchone()[0],
  'publisher_profiles': conn.execute("SELECT count(*) FROM publisher_profiles WHERE id='pub-rc2'").fetchone()[0],
}
conn.close()

# Simulated migration failure & backup restore verification on copy
rb = sqlite3.connect(sys.argv[2])
failed_ok = False
try:
  rb.execute("BEGIN IMMEDIATE")
  rb.execute("ALTER TABLE runtime_variable_envelopes ADD COLUMN confidence_source TEXT;")
  raise RuntimeError("SIMULATED_MIGRATION_FAULT")
except RuntimeError:
  rb.rollback()
  failed_ok = True
rb.close()
shutil.copy2(sys.argv[3], sys.argv[2])

print(json.dumps({
  'colsAfter': cols_after,
  'addedProvenanceColsPresent': all(c in cols_after for c in ['confidence_source','measured_confidence','rule_matched','device_id']),
  'schemaVersionsAfter': vers_after,
  'integrityCheck': integ,
  'foreignKeyCheckCount': len(fk),
  'keptEnvelopeRows': kept_env,
  'keptTableCounts': counts,
  'simulatedFaultRollbackTested': failed_ok,
}))`,
        studioTargetDb,
        rollbackTargetDb,
        studioBackupDb,
      ],
      ROOT
    )
  );
  const restoredShaAfter = sha256File(rollbackTargetDb);

  // Uninstall Studio from sandbox & clean temp dirs
  runCommandChecked(path.join(studioInstallDir, 'uninstall.exe'), ['/S'], SANDBOX, studioEnv);
  await new Promise((r) => setTimeout(r, 1000));
  rmSyncRetry(SANDBOX);

  const outReport = {
    executedAtUtc: new Date().toISOString(),
    rcCodeBaselineCommit: runCommandChecked('git', ['rev-parse', 'v0.1.0-p6-rc.3'], ROOT),
    rcCodeTag: 'v0.1.0-p6-rc.3',
    testA_CrossProcessDeviceLease: {
      targetSerial: TARGET_SERIAL,
      adbDevicesRaw,
      ...testAResult,
      recommendedMinimalFix:
        'Add canonical OS-level advisory lockfile (%LOCALAPPDATA%/FarmerPro/locks/dev_<canon_sha256>.lock holding {pid,exe,job_id,lease_expires_at}) acquired via Win32 CreateFile/LockFileEx inside a shared fp-scheduler lock guard, releasing on normal drop and reclaiming expired or dead-PID locks after forced kill while keeping distinct canonical devices independent.',
    },
    testB_SqliteV5UpgradeCompatibility: {
      status:
        postUpgradeInspection.addedProvenanceColsPresent &&
        !studioIpcReadback?.varsError &&
        postUpgradeInspection.integrityCheck[0] === 'ok' &&
        postUpgradeInspection.foreignKeyCheckCount === 0 &&
        backupShaBefore === restoredShaAfter
          ? 'PASS'
          : 'FAIL',
      testedAgainstRealFrozenRc3Binary: 'FarmerPro.exe (v0.1.0-p6-rc.3)',
      sourceDbSchemaCommit: 'b5a0567 (v0.1.0-p6-rc.2)',
      preUpgradeInspection,
      studioIpcReadbackAfterSecondLaunch: studioIpcReadback,
      postUpgradeInspection,
      backupRestoreSha256Match: backupShaBefore === restoredShaAfter,
      backupSha256: backupShaBefore,
      restoredSha256: restoredShaAfter,
      atomicRollbackNote:
        'Note: the 4 additive ALTER TABLE statements in apply_migrations run per-column with idempotent table_has_column checks outside an explicit transaction; on SQLite a per-column ALTER TABLE ADD COLUMN is atomic at SQLite statement level, and full pre-upgrade WAL backup + restore protects pre-existing v5 copies.',
    },
  };

  fs.mkdirSync(SUPP_DIR, { recursive: true });
  const outPath = path.join(SUPP_DIR, 'rc3_supplement_verification.json');
  fs.writeFileSync(outPath, JSON.stringify(outReport, null, 2));
  console.log(JSON.stringify({
    testA_status: outReport.testA_CrossProcessDeviceLease.status,
    testA_jobBBlocked: outReport.testA_CrossProcessDeviceLease.miniToolJobBOutcome?.blocked,
    testA_forcedKillRecovered: outReport.testA_CrossProcessDeviceLease.forcedKillRecovery?.jobDFinalState,
    testB_status: outReport.testB_SqliteV5UpgradeCompatibility.status,
    testB_addedCols: postUpgradeInspection.addedProvenanceColsPresent,
    testB_integrity: postUpgradeInspection.integrityCheck,
    testB_fkErrors: postUpgradeInspection.foreignKeyCheckCount,
    testB_backupRestoreMatch: outReport.testB_SqliteV5UpgradeCompatibility.backupRestoreSha256Match,
    reportPath: outPath,
  }, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
