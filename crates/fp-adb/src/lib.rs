use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use fp_domain::{
    ActionReceipt, ConnectionState, DeviceAdapterKind, DeviceDriver, DeviceHealthReport,
    DevicePortMapping, DeviceSummary, Frame, ScreenGeometry, SwipePath,
};
use std::time::{Duration, Instant};
use tokio::process::Command;

pub fn validate_adb_token(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("ADB_INVALID_ARG: {} cannot be empty", label);
    }
    if trimmed.contains('\0')
        || trimmed.contains('\r')
        || trimmed.contains('\n')
        || trimmed.chars().any(|c| c.is_whitespace())
        || trimmed.contains(';')
        || trimmed.contains('&')
        || trimmed.contains('|')
        || trimmed.contains('`')
    {
        bail!(
            "ADB_INVALID_ARG: Unsafe characters rejected in {}: '{}'",
            label,
            value
        );
    }
    Ok(trimmed.to_string())
}

pub fn infer_local_emulator_canonical_key(serial: &str) -> Option<String> {
    let trimmed = serial.trim();
    if let Some(rest) = trimmed.strip_prefix("emulator-") {
        if let Ok(port) = rest.parse::<u16>() {
            if port >= 5554 && port % 2 == 0 {
                return Some(format!("local-emu:{}", port));
            }
        }
    }
    let port_opt = trimmed
        .strip_prefix("127.0.0.1:")
        .or_else(|| trimmed.strip_prefix("localhost:"))
        .and_then(|p| p.parse::<u16>().ok());
    if let Some(port) = port_opt {
        // Android Emulator & LDPlayer expose ADB TCP on console_port + 1 (odd ports 5555..=5699)
        if (5555..=5699).contains(&port) && port % 2 == 1 {
            return Some(format!("local-emu:{}", port - 1));
        }
    }
    None
}

pub fn serial_transport_priority(serial: &str) -> u8 {
    let trimmed = serial.trim();
    if trimmed.starts_with("127.0.0.1:") || trimmed.starts_with("localhost:") {
        3
    } else if trimmed.contains(':') {
        2
    } else if trimmed.starts_with("emulator-") {
        1
    } else {
        0
    }
}

pub fn coalesce_device_summaries(
    entries: Vec<(DeviceSummary, Option<String>)>,
) -> Vec<DeviceSummary> {
    let mut best_by_key: std::collections::HashMap<String, DeviceSummary> =
        std::collections::HashMap::new();
    let mut ordered_keys: Vec<String> = Vec::new();

    let rank = |d: &DeviceSummary| -> (u8, u8, String) {
        let ready_rank = if d.connection_state == ConnectionState::Ready {
            0
        } else {
            1
        };
        (
            ready_rank,
            serial_transport_priority(&d.adb_serial),
            d.adb_serial.clone(),
        )
    };

    for (summary, observed_identity) in entries {
        let canonical_key = observed_identity
            .filter(|k| !k.trim().is_empty())
            .or_else(|| infer_local_emulator_canonical_key(&summary.adb_serial))
            .unwrap_or_else(|| format!("serial:{}", summary.adb_serial));

        match best_by_key.get_mut(&canonical_key) {
            Some(existing) => {
                if rank(&summary) < rank(existing) {
                    *existing = summary;
                }
            }
            None => {
                ordered_keys.push(canonical_key.clone());
                best_by_key.insert(canonical_key, summary);
            }
        }
    }

    ordered_keys
        .into_iter()
        .filter_map(|k| best_by_key.remove(&k))
        .collect()
}

pub fn classify_device_adapter(serial: &str, model: Option<&str>) -> DeviceAdapterKind {
    let s = serial.to_lowercase();
    let m = model.unwrap_or("").to_lowercase();
    if s.starts_with("fake")
        || s.starts_with("demo-fake")
        || s.starts_with("testkit")
        || s.starts_with("synthetic")
    {
        return DeviceAdapterKind::Synthetic;
    }
    if m.contains("nox") || s.contains("620") {
        return DeviceAdapterKind::Nox;
    }
    if m.contains("bluestacks") || m.contains("bst") {
        return DeviceAdapterKind::BlueStacks;
    }
    if s.starts_with("emulator-")
        || m.contains("ldplayer")
        || s.contains(":555")
        || s.contains(":556")
    {
        return DeviceAdapterKind::LdPlayer;
    }
    DeviceAdapterKind::PhysicalAdb
}

#[derive(Debug, Clone)]
pub struct AdbRunner {
    pub adb_path: String,
    pub timeout: Duration,
}

impl Default for AdbRunner {
    fn default() -> Self {
        Self {
            adb_path: Self::discover_adb_path(),
            timeout: Duration::from_secs(15),
        }
    }
}

impl AdbRunner {
    pub fn discover_adb_path() -> String {
        // 1. Check FARMERPRO_ADB_PATH env var
        if let Ok(env_path) = std::env::var("FARMERPRO_ADB_PATH") {
            if !env_path.trim().is_empty() && std::path::Path::new(&env_path).exists() {
                return env_path;
            }
        }

        let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };

        // 2. Check relative to current executable: <exe_dir>/bin/adb.exe
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let bin_adb = exe_dir.join("bin").join(adb_name);
                if bin_adb.exists() {
                    return bin_adb.to_string_lossy().to_string();
                }
                let adjacent_adb = exe_dir.join(adb_name);
                if adjacent_adb.exists() {
                    return adjacent_adb.to_string_lossy().to_string();
                }
            }
        }

        // 3. Check current working directory: ./bin/adb.exe
        let cwd_bin = std::path::Path::new("bin").join(adb_name);
        if cwd_bin.exists() {
            if let Ok(canonical) = cwd_bin.canonicalize() {
                return canonical.to_string_lossy().to_string();
            }
            return cwd_bin.to_string_lossy().to_string();
        }

        // 4. Check common LDPlayer and Android SDK paths on Windows
        #[cfg(windows)]
        {
            let common_candidates = [
                r"C:\LDPlayer\LDPlayer14\adb.exe",
                r"C:\LDPlayer\LDPlayer9\adb.exe",
                r"C:\LDPlayer\LDPlayer4\adb.exe",
                r"D:\LDPlayer\LDPlayer14\adb.exe",
                r"D:\LDPlayer\LDPlayer9\adb.exe",
                r"E:\LDPlayer\LDPlayer14\adb.exe",
            ];
            for candidate in common_candidates {
                if std::path::Path::new(candidate).exists() {
                    return candidate.to_string();
                }
            }
        }

        // 5. Fallback to system adb on PATH
        "adb".to_string()
    }

    pub fn new(adb_path: Option<String>) -> Self {
        Self {
            adb_path: adb_path.unwrap_or_else(Self::discover_adb_path),
            timeout: Duration::from_secs(15),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn run_args(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.adb_path);
        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.args(args);

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "ADB_TIMEOUT: ADB command {:?} timed out after {:?}",
                    args,
                    self.timeout
                )
            })?
            .context("ADB_EXEC_FAILED: Failed to execute ADB command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            if msg.contains("offline") || msg.contains("not found") {
                bail!("ADB_DEVICE_OFFLINE: {}", msg);
            }
            if msg.contains("unauthorized") {
                bail!("ADB_UNAUTHORIZED: {}", msg);
            }
            bail!("ADB error (exit code {:?}): {}", output.status.code(), msg);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn run_raw_bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        let mut cmd = Command::new(&self.adb_path);
        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.args(args);

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .context("ADB command timed out")?
            .context("Failed to execute ADB command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ADB error: {}", stderr.trim());
        }

        Ok(output.stdout)
    }

    pub async fn list_devices_raw(&self) -> Result<Vec<DeviceSummary>> {
        let output = self.run_args(&["devices", "-l"]).await?;
        Self::parse_devices_output(&output)
    }

    /// Probes the live OS boot instance & hardware serial so multi-transport
    /// aliases (e.g. `emulator-5554` + `127.0.0.1:5555`, or USB + WiFi ADB)
    /// resolve to one canonical Android instance identity.
    pub async fn probe_canonical_device_identity(&self, serial: &str) -> Option<String> {
        let s = validate_adb_token("serial", serial).ok()?;
        let boot_id_out = self
            .run_args(&["-s", &s, "shell", "cat", "/proc/sys/kernel/random/boot_id"])
            .await
            .ok();
        let boot_id = boot_id_out
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty() && !v.to_lowercase().contains("no such file"))
            .map(str::to_string);

        if let Some(bid) = boot_id {
            let boot_sn = self
                .run_args(&["-s", &s, "shell", "getprop", "ro.boot.serialno"])
                .await
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_default();
            return Some(format!("android-boot:{}:{}", boot_sn, bid));
        }

        infer_local_emulator_canonical_key(&s)
    }

    pub async fn list_devices(&self) -> Result<Vec<DeviceSummary>> {
        let raw = self.list_devices_raw().await?;
        if raw.len() <= 1 {
            return Ok(raw);
        }

        let mut entries: Vec<(DeviceSummary, Option<String>)> = Vec::with_capacity(raw.len());
        let mut set = tokio::task::JoinSet::new();
        for (idx, dev) in raw.into_iter().enumerate() {
            let runner = self.clone();
            set.spawn(async move {
                let canonical = if dev.connection_state == ConnectionState::Ready {
                    runner
                        .probe_canonical_device_identity(&dev.adb_serial)
                        .await
                } else {
                    infer_local_emulator_canonical_key(&dev.adb_serial)
                };
                (idx, dev, canonical)
            });
        }

        let mut indexed = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(item) = res {
                indexed.push(item);
            }
        }
        indexed.sort_by_key(|(idx, _, _)| *idx);
        for (_, dev, canonical) in indexed {
            entries.push((dev, canonical));
        }

        Ok(coalesce_device_summaries(entries))
    }

    pub async fn connect(&self, endpoint: &str) -> Result<String> {
        self.run_args(&["connect", endpoint]).await
    }

    pub async fn disconnect(&self, endpoint: &str) -> Result<String> {
        self.run_args(&["disconnect", endpoint]).await
    }

    pub async fn list_packages(&self, serial: &str, user_only: bool) -> Result<Vec<String>> {
        let args = if user_only {
            vec!["-s", serial, "shell", "pm", "list", "packages", "-3"]
        } else {
            vec!["-s", serial, "shell", "pm", "list", "packages"]
        };
        let out = self.run_args(&args).await?;
        let mut packages = Vec::new();
        for line in out.lines() {
            let line = line.trim();
            if let Some(pkg) = line.strip_prefix("package:") {
                let pkg_clean = pkg.trim();
                if !pkg_clean.is_empty() {
                    packages.push(pkg_clean.to_string());
                }
            }
        }
        Ok(packages)
    }

    pub fn parse_port_mappings(
        output: &str,
        direction: &str,
        filter_serial: Option<&str>,
    ) -> Vec<DevicePortMapping> {
        let mut list = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let serial = parts[0].to_string();
                if let Some(target) = filter_serial {
                    if serial != target {
                        continue;
                    }
                }
                // `adb forward --list` emits: `<serial> <local_host> <remote_device>`
                // `adb reverse --list` emits: `<host_transport> <remote_device> <local_host>`
                let (local, remote) = if direction == "reverse" {
                    (parts[2].to_string(), parts[1].to_string())
                } else {
                    (parts[1].to_string(), parts[2].to_string())
                };
                list.push(DevicePortMapping {
                    direction: direction.to_string(),
                    serial,
                    local,
                    remote,
                });
            }
        }
        list
    }

    pub async fn forward_add(
        &self,
        serial: &str,
        local: &str,
        remote: &str,
    ) -> Result<DevicePortMapping> {
        let s = validate_adb_token("serial", serial)?;
        let l = validate_adb_token("local_spec", local)?;
        let r = validate_adb_token("remote_spec", remote)?;
        self.run_args(&["-s", &s, "forward", &l, &r]).await?;
        Ok(DevicePortMapping {
            direction: "forward".to_string(),
            serial: s,
            local: l,
            remote: r,
        })
    }

    pub async fn forward_list(&self, serial: Option<&str>) -> Result<Vec<DevicePortMapping>> {
        let out = self.run_args(&["forward", "--list"]).await?;
        Ok(Self::parse_port_mappings(&out, "forward", serial))
    }

    pub async fn forward_remove(&self, serial: &str, local: &str) -> Result<()> {
        let s = validate_adb_token("serial", serial)?;
        let l = validate_adb_token("local_spec", local)?;
        self.run_args(&["-s", &s, "forward", "--remove", &l])
            .await?;
        Ok(())
    }

    pub async fn reverse_add(
        &self,
        serial: &str,
        remote: &str,
        local: &str,
    ) -> Result<DevicePortMapping> {
        let s = validate_adb_token("serial", serial)?;
        let r = validate_adb_token("remote_spec", remote)?;
        let l = validate_adb_token("local_spec", local)?;
        self.run_args(&["-s", &s, "reverse", &r, &l]).await?;
        Ok(DevicePortMapping {
            direction: "reverse".to_string(),
            serial: s,
            local: l,
            remote: r,
        })
    }

    pub async fn reverse_list(&self, serial: &str) -> Result<Vec<DevicePortMapping>> {
        let s = validate_adb_token("serial", serial)?;
        let out = self.run_args(&["-s", &s, "reverse", "--list"]).await?;
        let mut mappings = Self::parse_port_mappings(&out, "reverse", None);
        for m in &mut mappings {
            // Preserve the target device serial rather than ADB's internal `host-XX` transport token
            m.serial = s.clone();
        }
        Ok(mappings)
    }

    pub async fn reverse_remove(&self, serial: &str, remote: &str) -> Result<()> {
        let s = validate_adb_token("serial", serial)?;
        let r = validate_adb_token("remote_spec", remote)?;
        self.run_args(&["-s", &s, "reverse", "--remove", &r])
            .await?;
        Ok(())
    }

    pub async fn forward_remove_all_for_serial(&self, serial: &str) -> Result<usize> {
        let s = validate_adb_token("serial", serial)?;
        let active = self.forward_list(Some(&s)).await?;
        let mut removed = 0usize;
        for m in active {
            self.forward_remove(&s, &m.local).await?;
            removed += 1;
        }
        Ok(removed)
    }

    pub async fn reverse_remove_all(&self, serial: &str) -> Result<()> {
        let s = validate_adb_token("serial", serial)?;
        self.run_args(&["-s", &s, "reverse", "--remove-all"])
            .await?;
        Ok(())
    }

    pub async fn probe_health(&self, serial: &str) -> DeviceHealthReport {
        let start = Instant::now();
        let s = match validate_adb_token("serial", serial) {
            Ok(v) => v,
            Err(e) => {
                return DeviceHealthReport {
                    serial: serial.to_string(),
                    reachable: false,
                    connection_state: ConnectionState::Blocked,
                    width_px: 0,
                    height_px: 0,
                    rotation_deg: 0,
                    latency_ms: 0,
                    error: Some(e.to_string()),
                };
            }
        };

        let devices = match self.list_devices_raw().await {
            Ok(d) => d,
            Err(e) => {
                return DeviceHealthReport {
                    serial: s,
                    reachable: false,
                    connection_state: ConnectionState::Disconnected,
                    width_px: 0,
                    height_px: 0,
                    rotation_deg: 0,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                };
            }
        };

        let found = devices.into_iter().find(|d| d.adb_serial == s);
        let Some(summary) = found else {
            return DeviceHealthReport {
                serial: s,
                reachable: false,
                connection_state: ConnectionState::Disconnected,
                width_px: 0,
                height_px: 0,
                rotation_deg: 0,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some("ADB_DEVICE_NOT_ATTACHED".to_string()),
            };
        };

        if summary.connection_state != ConnectionState::Ready {
            return DeviceHealthReport {
                serial: s,
                reachable: false,
                connection_state: summary.connection_state,
                width_px: summary.width_px,
                height_px: summary.height_px,
                rotation_deg: summary.rotation_deg,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some("Device attached but not in ready 'device' state".to_string()),
            };
        }

        let drv = AdbDeviceDriver::new(s.clone(), self.clone());
        match drv.geometry().await {
            Ok(geom) => DeviceHealthReport {
                serial: s,
                reachable: true,
                connection_state: ConnectionState::Ready,
                width_px: geom.width_px,
                height_px: geom.height_px,
                rotation_deg: geom.rotation_deg,
                latency_ms: start.elapsed().as_millis() as u64,
                error: None,
            },
            Err(e) => DeviceHealthReport {
                serial: s,
                reachable: false,
                connection_state: ConnectionState::Recovering,
                width_px: 0,
                height_px: 0,
                rotation_deg: 0,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
            },
        }
    }

    pub async fn reconnect(&self, endpoint: &str) -> Result<DeviceHealthReport> {
        let ep = validate_adb_token("endpoint", endpoint)?;
        let _ = self.disconnect(&ep).await;
        let conn_out = self.connect(&ep).await?;
        let mut rep = self.probe_health(&ep).await;
        if !rep.reachable && rep.error.is_none() {
            rep.error = Some(conn_out);
        }
        Ok(rep)
    }

    pub fn parse_devices_output(output: &str) -> Result<Vec<DeviceSummary>> {
        let mut devices = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let serial = parts[0].to_string();
            let state_str = if parts.len() > 1 { parts[1] } else { "offline" };

            let connection_state = match state_str {
                "device" => ConnectionState::Ready,
                "unauthorized" => ConnectionState::Unauthorized,
                "authorizing" => ConnectionState::Authorizing,
                "offline" => ConnectionState::Disconnected,
                _ => ConnectionState::Blocked,
            };

            let mut model = None;
            for part in &parts[2..] {
                if let Some(m) = part.strip_prefix("model:") {
                    model = Some(m.to_string());
                }
            }

            let alias = model.clone().unwrap_or_else(|| serial.clone());

            devices.push(DeviceSummary {
                id: serial.clone(),
                adb_serial: serial,
                alias,
                android_api: None,
                model,
                width_px: 1080,
                height_px: 1920,
                rotation_deg: 0,
                connection_state,
                updated_at: chrono::Utc::now(),
            });
        }

        Ok(devices)
    }
}

pub struct AdbDeviceDriver {
    pub serial: String,
    pub runner: AdbRunner,
}

impl AdbDeviceDriver {
    pub fn new(serial: String, runner: AdbRunner) -> Self {
        Self { serial, runner }
    }
}

#[async_trait]
impl DeviceDriver for AdbDeviceDriver {
    async fn snapshot(&self) -> Result<Frame> {
        let bytes = self
            .runner
            .run_raw_bytes(&["-s", &self.serial, "exec-out", "screencap", "-p"])
            .await?;
        let geom = self.geometry().await?;
        Ok(Frame {
            bytes,
            geometry: geom,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    async fn geometry(&self) -> Result<ScreenGeometry> {
        let wm_size = self
            .runner
            .run_args(&["-s", &self.serial, "shell", "wm", "size"])
            .await
            .unwrap_or_default();

        let mut width = 1080;
        let mut height = 1920;

        for line in wm_size.lines() {
            if let Some(pos) = line.find(':') {
                let res_str = line[pos + 1..].trim();
                let dims: Vec<&str> = res_str.split('x').collect();
                if dims.len() == 2 {
                    if let (Ok(w), Ok(h)) = (dims[0].trim().parse(), dims[1].trim().parse()) {
                        width = w;
                        height = h;
                    }
                }
            }
        }

        // Detect device rotation (0, 90, 180, 270)
        let rotation_out = self
            .runner
            .run_args(&["-s", &self.serial, "shell", "dumpsys", "window"])
            .await
            .unwrap_or_default();

        let mut rotation_deg = 0;
        for line in rotation_out.lines() {
            if line.contains("mCurrentRotation") || line.contains("SurfaceOrientation") {
                if line.contains("ROTATION_90") || line.contains("=1") || line.contains(": 1") {
                    rotation_deg = 90;
                    break;
                } else if line.contains("ROTATION_180")
                    || line.contains("=2")
                    || line.contains(": 2")
                {
                    rotation_deg = 180;
                    break;
                } else if line.contains("ROTATION_270")
                    || line.contains("=3")
                    || line.contains(": 3")
                {
                    rotation_deg = 270;
                    break;
                }
            }
        }

        Ok(ScreenGeometry::new(width, height, rotation_deg))
    }

    async fn tap(&self, x: u32, y: u32) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let x_str = x.to_string();
        let y_str = y.to_string();
        self.runner
            .run_args(&["-s", &self.serial, "shell", "input", "tap", &x_str, &y_str])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: Some(fp_domain::Point {
                x: x as f32,
                y: y as f32,
            }),
            verify_passed: false,
        })
    }

    async fn swipe(&self, path: SwipePath) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let s_x = path.start_x.to_string();
        let s_y = path.start_y.to_string();
        let e_x = path.end_x.to_string();
        let e_y = path.end_y.to_string();
        let dur = path.duration_ms.to_string();

        self.runner
            .run_args(&[
                "-s",
                &self.serial,
                "shell",
                "input",
                "swipe",
                &s_x,
                &s_y,
                &e_x,
                &e_y,
                &dur,
            ])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn key_event(&self, keycode: u32) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let code_str = keycode.to_string();
        self.runner
            .run_args(&["-s", &self.serial, "shell", "input", "keyevent", &code_str])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn launch_app(&self, package: &str) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let monkey_res = self
            .runner
            .run_args(&[
                "-s",
                &self.serial,
                "shell",
                "monkey",
                "-p",
                package,
                "-c",
                "android.intent.category.LAUNCHER",
                "1",
            ])
            .await;

        let need_fallback = match &monkey_res {
            Ok(output) => {
                let lower = output.to_lowercase();
                lower.contains("no activities found")
                    || lower.contains("aborted")
                    || lower.contains("** error")
                    || lower.contains("events injected: 0")
            }
            Err(_) => true,
        };

        if need_fallback {
            // Android 14+ / emulator fallback: resolve launcher activity and use am start
            let resolve_out = self
                .runner
                .run_args(&[
                    "-s",
                    &self.serial,
                    "shell",
                    "cmd",
                    "package",
                    "resolve-activity",
                    "--brief",
                    package,
                ])
                .await
                .unwrap_or_default();

            let mut component = None;
            for line in resolve_out.lines() {
                let trimmed = line.trim();
                if trimmed.contains('/') && !trimmed.starts_with("priority=") {
                    component = Some(trimmed.to_string());
                    break;
                }
            }

            if let Some(cmp) = component {
                self.runner
                    .run_args(&["-s", &self.serial, "shell", "am", "start", "-n", &cmp])
                    .await?;
            } else {
                self.runner
                    .run_args(&[
                        "-s",
                        &self.serial,
                        "shell",
                        "am",
                        "start",
                        "-a",
                        "android.intent.action.MAIN",
                        "-c",
                        "android.intent.category.LAUNCHER",
                        "-p",
                        package,
                    ])
                    .await?;
            }
        }

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn stop_app(&self, package: &str) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        self.runner
            .run_args(&["-s", &self.serial, "shell", "am", "force-stop", package])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn get_ui_tree(&self) -> Result<String> {
        let dump_path = "/sdcard/window_dump.xml";
        self.runner
            .run_args(&[
                "-s",
                &self.serial,
                "shell",
                "uiautomator",
                "dump",
                dump_path,
            ])
            .await?;
        let xml = self
            .runner
            .run_args(&["-s", &self.serial, "shell", "cat", dump_path])
            .await?;
        Ok(xml)
    }

    async fn input_text(&self, text: &str) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let escaped = text.replace(' ', "%s");
        self.runner
            .run_args(&["-s", &self.serial, "shell", "input", "text", &escaped])
            .await?;
        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adb_devices_output() {
        let mock_out = r#"
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:Pixel_6 device:emu64x transport_id:1
192.168.1.50:5555      unauthorized transport_id:2
        "#;
        let devices = AdbRunner::parse_devices_output(mock_out).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].adb_serial, "emulator-5554");
        assert_eq!(devices[0].model.as_deref(), Some("Pixel_6"));
        assert_eq!(devices[0].connection_state, ConnectionState::Ready);
        assert_eq!(devices[1].adb_serial, "192.168.1.50:5555");
        assert_eq!(devices[1].connection_state, ConnectionState::Unauthorized);
    }

    #[test]
    fn test_discover_adb_path() {
        let path = AdbRunner::discover_adb_path();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_parse_forward_reverse_and_validate_specs() {
        let fwd =
            "emulator-5554 tcp:18080 tcp:8080\nemulator-5556 tcp:18081 localabstract:scrcpy\n";
        let all = AdbRunner::parse_port_mappings(fwd, "forward", None);
        assert_eq!(all.len(), 2);
        let one = AdbRunner::parse_port_mappings(fwd, "forward", Some("emulator-5554"));
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].local, "tcp:18080");
        assert_eq!(one[0].remote, "tcp:8080");

        let rev = "host-22 tcp:19557 tcp:19558\n";
        let rev_parsed = AdbRunner::parse_port_mappings(rev, "reverse", None);
        assert_eq!(rev_parsed.len(), 1);
        assert_eq!(rev_parsed[0].remote, "tcp:19557");
        assert_eq!(rev_parsed[0].local, "tcp:19558");
        assert_eq!(rev_parsed[0].serial, "host-22");

        // Empty port mapping output and filtered serial mismatch return cleanly
        assert!(AdbRunner::parse_port_mappings("", "reverse", None).is_empty());
        assert!(
            AdbRunner::parse_port_mappings("   \n\n", "forward", Some("emulator-5554")).is_empty()
        );
        assert!(AdbRunner::parse_port_mappings(fwd, "forward", Some("host-22")).is_empty());

        assert!(validate_adb_token("local", "tcp:18080").is_ok());
        assert!(validate_adb_token("local", "tcp:8080; calc.exe").is_err());
        assert!(validate_adb_token("serial", "").is_err());
        assert_eq!(
            classify_device_adapter("emulator-5554", Some("LDPlayer")),
            DeviceAdapterKind::LdPlayer
        );
        assert_eq!(
            classify_device_adapter("127.0.0.1:62001", Some("Nox")),
            DeviceAdapterKind::Nox
        );
    }

    #[test]
    fn test_coalesce_duplicate_emulator_and_multitransport_identities() {
        let mock_out = r#"
List of devices attached
127.0.0.1:5555         device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k transport_id:2
emulator-5554          device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k transport_id:1
emulator-5556          device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k transport_id:3
USB-PIXEL-9            device product:tokay model:Pixel_9 transport_id:4
192.168.1.44:5555      device product:tokay model:Pixel_9 transport_id:5
        "#;
        let raw = AdbRunner::parse_devices_output(mock_out).unwrap();
        assert_eq!(raw.len(), 5);

        let entries = raw
            .into_iter()
            .map(|d| {
                let obs = match d.adb_serial.as_str() {
                    "127.0.0.1:5555" | "emulator-5554" => {
                        Some("android-boot:EMULATOR37X1X11X0:boot-avd-1".to_string())
                    }
                    "emulator-5556" => {
                        Some("android-boot:EMULATOR37X1X11X0:boot-avd-2".to_string())
                    }
                    "USB-PIXEL-9" | "192.168.1.44:5555" => {
                        Some("android-boot:3A191FDJH:boot-pixel-9".to_string())
                    }
                    _ => None,
                };
                (d, obs)
            })
            .collect();

        let deduped = coalesce_device_summaries(entries);
        let serials: Vec<&str> = deduped.iter().map(|d| d.adb_serial.as_str()).collect();
        assert_eq!(
            serials,
            vec!["emulator-5554", "emulator-5556", "USB-PIXEL-9"]
        );

        // Also works deterministically via fallback inference without shell boot_id probe
        assert_eq!(
            infer_local_emulator_canonical_key("emulator-5554"),
            infer_local_emulator_canonical_key("127.0.0.1:5555")
        );
        assert_ne!(
            infer_local_emulator_canonical_key("emulator-5554"),
            infer_local_emulator_canonical_key("emulator-5556")
        );

        // Two distinct devices sharing identical model/product or lacking boot_id MUST NEVER coalesce
        let unknown_out = r#"
List of devices attached
192.168.1.80:5555      device product:phone model:SameModel transport_id:10
192.168.1.81:5555      device product:phone model:SameModel transport_id:11
        "#;
        let unknown_raw = AdbRunner::parse_devices_output(unknown_out).unwrap();
        let unknown_dedup =
            coalesce_device_summaries(unknown_raw.into_iter().map(|d| (d, None)).collect());
        assert_eq!(unknown_dedup.len(), 2);
        assert_eq!(unknown_dedup[0].adb_serial, "192.168.1.80:5555");
        assert_eq!(unknown_dedup[1].adb_serial, "192.168.1.81:5555");
    }
}
