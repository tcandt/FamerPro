use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Metadata stored at byte offset >= 64 inside the per-device OS advisory lockfile.
/// Bytes `[0..64)` are held exclusively via Win32 `LockFileEx` for the full lifetime of the job,
/// leaving `[64..)` readable by competing processes so conflict errors report exact owner details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsLockOwnerMetadata {
    pub pid: u32,
    pub exe_path: String,
    pub job_id: String,
    pub device_id: String,
    pub canonical_key: String,
    pub generation: u64,
    pub acquired_at_utc: String,
    pub expires_at_utc: String,
}

#[derive(Debug, Clone)]
pub enum ConflictPolicy {
    FailImmediately,
    WaitUpTo(std::time::Duration),
}

impl Default for ConflictPolicy {
    fn default() -> Self {
        Self::FailImmediately
    }
}

pub fn canonical_lock_hash(canonical_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_key.trim().as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

pub fn lock_file_path_for_canonical_key(lock_dir: &Path, canonical_key: &str) -> PathBuf {
    lock_dir.join(format!("dev_{}.lock", canonical_lock_hash(canonical_key)))
}

pub fn resolve_default_lock_dir() -> Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os("FARMERPRO_LOCK_DIR") {
        let p = PathBuf::from(override_dir);
        if !p.as_os_str().is_empty() {
            return ensure_safe_lock_dir(&p);
        }
    }

    #[cfg(windows)]
    if let Some(local_app) = std::env::var_os("LOCALAPPDATA") {
        let p = PathBuf::from(local_app).join("FarmerPro").join("locks");
        if !p.as_os_str().is_empty() {
            return ensure_safe_lock_dir(&p);
        }
    }

    if let Some(temp_env) = std::env::var_os("TEMP") {
        let p = PathBuf::from(temp_env).join("FarmerPro").join("locks");
        if !p.as_os_str().is_empty() {
            return ensure_safe_lock_dir(&p);
        }
    }

    let fallback = std::env::temp_dir().join("FarmerPro").join("locks");
    ensure_safe_lock_dir(&fallback)
}

pub fn ensure_safe_lock_dir(dir: &Path) -> Result<PathBuf> {
    if dir.as_os_str().is_empty() {
        bail!("DEVICE_LOCK_INIT_FAILED: lock directory path cannot be empty");
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        bail!(
            "DEVICE_LOCK_INIT_FAILED: failed creating OS device lock directory '{}': {}",
            dir.display(),
            e
        );
    }
    ensure_not_reparse_or_symlink(dir)?;
    if !dir.is_dir() {
        bail!(
            "DEVICE_LOCK_INIT_FAILED: lock path '{}' is not a directory",
            dir.display()
        );
    }
    Ok(dir.to_path_buf())
}

fn ensure_not_reparse_or_symlink(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let meta = std::fs::symlink_metadata(path).map_err(|e| {
        anyhow::anyhow!(
            "DEVICE_LOCK_INIT_FAILED: failed inspecting metadata at '{}': {}",
            path.display(),
            e
        )
    })?;
    if meta.file_type().is_symlink() {
        bail!(
            "DEVICE_LOCK_INIT_FAILED: symlink detected at device lock path '{}'",
            path.display()
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            bail!(
                "DEVICE_LOCK_INIT_FAILED: NTFS junction/reparse point forbidden at '{}'",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::Mutex;

    type HANDLE = *mut core::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;
    type LPCWSTR = *const u16;

    const GENERIC_READ: DWORD = 0x8000_0000;
    const GENERIC_WRITE: DWORD = 0x4000_0000;
    const FILE_SHARE_READ: DWORD = 0x0000_0001;
    const FILE_SHARE_WRITE: DWORD = 0x0000_0002;
    const OPEN_ALWAYS: DWORD = 4;
    const FILE_ATTRIBUTE_NORMAL: DWORD = 0x80;
    const FILE_BEGIN: DWORD = 0;

    const LOCKFILE_FAIL_IMMEDIATELY: DWORD = 0x0000_0001;
    const LOCKFILE_EXCLUSIVE_LOCK: DWORD = 0x0000_0002;
    const ERROR_SHARING_VIOLATION: DWORD = 32;
    const ERROR_LOCK_VIOLATION: DWORD = 33;

    const LOCK_RANGE_BYTES: DWORD = 64;
    const METADATA_OFFSET: i64 = 64;

    #[repr(C)]
    #[derive(Default)]
    struct Overlapped {
        internal: usize,
        internal_high: usize,
        offset: DWORD,
        offset_high: DWORD,
        h_event: HANDLE,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            lpFileName: LPCWSTR,
            dwDesiredAccess: DWORD,
            dwShareMode: DWORD,
            lpSecurityAttributes: *const core::ffi::c_void,
            dwCreationDisposition: DWORD,
            dwFlagsAndAttributes: DWORD,
            hTemplateFile: HANDLE,
        ) -> HANDLE;
        fn LockFileEx(
            hFile: HANDLE,
            dwFlags: DWORD,
            dwReserved: DWORD,
            nNumberOfBytesToLockLow: DWORD,
            nNumberOfBytesToLockHigh: DWORD,
            lpOverlapped: *mut Overlapped,
        ) -> BOOL;
        fn UnlockFileEx(
            hFile: HANDLE,
            dwReserved: DWORD,
            nNumberOfBytesToUnlockLow: DWORD,
            nNumberOfBytesToUnlockHigh: DWORD,
            lpOverlapped: *mut Overlapped,
        ) -> BOOL;
        fn SetFilePointerEx(
            hFile: HANDLE,
            liDistanceToMove: i64,
            lpNewFilePointer: *mut i64,
            dwMoveMethod: DWORD,
        ) -> BOOL;
        fn SetEndOfFile(hFile: HANDLE) -> BOOL;
        fn WriteFile(
            hFile: HANDLE,
            lpBuffer: *const u8,
            nNumberOfBytesToWrite: DWORD,
            lpNumberOfBytesWritten: *mut DWORD,
            lpOverlapped: *mut Overlapped,
        ) -> BOOL;
        fn ReadFile(
            hFile: HANDLE,
            lpBuffer: *mut u8,
            nNumberOfBytesToRead: DWORD,
            lpNumberOfBytesRead: *mut DWORD,
            lpOverlapped: *mut Overlapped,
        ) -> BOOL;
        fn FlushFileBuffers(hFile: HANDLE) -> BOOL;
        fn CloseHandle(hObject: HANDLE) -> BOOL;
        fn GetLastError() -> DWORD;
    }

    fn invalid_handle() -> HANDLE {
        (-1isize) as HANDLE
    }

    fn to_wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    unsafe fn write_metadata_at_offset(h: HANDLE, meta: &OsLockOwnerMetadata) -> Result<()> {
        let payload = serde_json::to_vec(meta)?;
        let mut new_pos: i64 = 0;
        if SetFilePointerEx(h, METADATA_OFFSET, &mut new_pos, FILE_BEGIN) == 0 {
            bail!(
                "DEVICE_LOCK_INIT_FAILED: SetFilePointerEx failed (win32 error {})",
                GetLastError()
            );
        }
        let mut written: DWORD = 0;
        if !payload.is_empty()
            && WriteFile(
                h,
                payload.as_ptr(),
                payload.len() as DWORD,
                &mut written,
                core::ptr::null_mut(),
            ) == 0
        {
            bail!(
                "DEVICE_LOCK_INIT_FAILED: WriteFile failed (win32 error {})",
                GetLastError()
            );
        }
        let _ = SetEndOfFile(h);
        let _ = FlushFileBuffers(h);
        Ok(())
    }

    unsafe fn read_metadata_at_offset(h: HANDLE) -> Option<OsLockOwnerMetadata> {
        let mut new_pos: i64 = 0;
        if SetFilePointerEx(h, METADATA_OFFSET, &mut new_pos, FILE_BEGIN) == 0 {
            return None;
        }
        let mut buf = vec![0u8; 8192];
        let mut bytes_read: DWORD = 0;
        if ReadFile(
            h,
            buf.as_mut_ptr(),
            buf.len() as DWORD,
            &mut bytes_read,
            core::ptr::null_mut(),
        ) == 0
        {
            return None;
        }
        if bytes_read == 0 {
            return None;
        }
        let raw = std::str::from_utf8(&buf[..bytes_read as usize])
            .ok()?
            .trim();
        if raw.is_empty() {
            return None;
        }
        serde_json::from_str::<OsLockOwnerMetadata>(raw).ok()
    }

    unsafe fn open_lockfile_handle(lock_path: &Path) -> Result<HANDLE> {
        ensure_not_reparse_or_symlink(lock_path)?;
        let wide = to_wide(lock_path.as_os_str());
        let h = CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            core::ptr::null(),
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            core::ptr::null_mut(),
        );
        if h == invalid_handle() {
            let err = GetLastError();
            if err == ERROR_SHARING_VIOLATION {
                bail!(
                    "DEVICE_LEASE_CONFLICT: OS lockfile '{}' is held with incompatible sharing (win32 error {})",
                    lock_path.display(),
                    err
                );
            }
            bail!(
                "DEVICE_LOCK_INIT_FAILED: CreateFileW('{}') failed (win32 error {})",
                lock_path.display(),
                err
            );
        }
        Ok(h)
    }

    #[derive(Debug)]
    pub struct OsDeviceLockGuard {
        handle: Mutex<Option<HANDLE>>,
        lock_path: PathBuf,
        metadata: Mutex<OsLockOwnerMetadata>,
    }

    // Windows file handles returned by CreateFileW are kernel-object handles valid across
    // any thread inside the owning process; all mutations/close happen behind Mutex / Drop.
    unsafe impl Send for OsDeviceLockGuard {}
    unsafe impl Sync for OsDeviceLockGuard {}

    impl OsDeviceLockGuard {
        pub fn acquire_once(lock_dir: &Path, metadata: OsLockOwnerMetadata) -> Result<Self> {
            let safe_dir = ensure_safe_lock_dir(lock_dir)?;
            let lock_path = lock_file_path_for_canonical_key(&safe_dir, &metadata.canonical_key);

            unsafe {
                let h = open_lockfile_handle(&lock_path)?;
                let mut ov = Overlapped::default();
                let locked = LockFileEx(
                    h,
                    LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                    0,
                    LOCK_RANGE_BYTES,
                    0,
                    &mut ov,
                );
                if locked == 0 {
                    let err = GetLastError();
                    if err == ERROR_LOCK_VIOLATION || err == ERROR_SHARING_VIOLATION {
                        let holder = read_metadata_at_offset(h);
                        let _ = CloseHandle(h);
                        if let Some(owner) = holder {
                            bail!(
                                "DEVICE_LEASE_CONFLICT: Device '{}' (canonical '{}') is exclusively locked by pid={} job='{}' exe='{}' (until {}) [lockfile={}]",
                                metadata.device_id,
                                metadata.canonical_key,
                                owner.pid,
                                owner.job_id,
                                owner.exe_path,
                                owner.expires_at_utc,
                                lock_path.display()
                            );
                        } else {
                            bail!(
                                "DEVICE_LEASE_CONFLICT: Device '{}' (canonical '{}') is exclusively locked by another active process (win32 error {}) [lockfile={}]",
                                metadata.device_id,
                                metadata.canonical_key,
                                err,
                                lock_path.display()
                            );
                        }
                    }
                    let _ = CloseHandle(h);
                    bail!(
                        "DEVICE_LOCK_INIT_FAILED: LockFileEx failed at '{}' (win32 error {})",
                        lock_path.display(),
                        err
                    );
                }

                if let Err(e) = write_metadata_at_offset(h, &metadata) {
                    let mut ov_u = Overlapped::default();
                    let _ = UnlockFileEx(h, 0, LOCK_RANGE_BYTES, 0, &mut ov_u);
                    let _ = CloseHandle(h);
                    return Err(e);
                }

                Ok(Self {
                    handle: Mutex::new(Some(h)),
                    lock_path,
                    metadata: Mutex::new(metadata),
                })
            }
        }

        pub fn update_expiry(&self, new_expires_at: DateTime<Utc>) -> Result<()> {
            let guard = self.handle.lock().unwrap();
            let Some(h) = *guard else {
                bail!("DEVICE_LOCK_INIT_FAILED: OS lock handle already closed");
            };
            let mut meta = self.metadata.lock().unwrap();
            meta.expires_at_utc = new_expires_at.to_rfc3339();
            unsafe { write_metadata_at_offset(h, &meta) }
        }

        pub fn lock_path(&self) -> &Path {
            &self.lock_path
        }

        pub fn metadata_snapshot(&self) -> OsLockOwnerMetadata {
            self.metadata.lock().unwrap().clone()
        }
    }

    impl Drop for OsDeviceLockGuard {
        fn drop(&mut self) {
            let mut guard = self.handle.lock().unwrap();
            if let Some(h) = guard.take() {
                unsafe {
                    // Truncate owner payload back to offset 64 while STILL holding the exclusive lock,
                    // then unlock byte range [0..64) and close handle. NEVER unlink the lockfile so
                    // another process opening it concurrently cannot hit an unlink/recreate race.
                    let mut new_pos: i64 = 0;
                    if SetFilePointerEx(h, METADATA_OFFSET, &mut new_pos, FILE_BEGIN) != 0 {
                        let _ = SetEndOfFile(h);
                        let _ = FlushFileBuffers(h);
                    }
                    let mut ov = Overlapped::default();
                    let _ = UnlockFileEx(h, 0, LOCK_RANGE_BYTES, 0, &mut ov);
                    let _ = CloseHandle(h);
                }
            }
        }
    }

    /// Probes whether another process currently holds the OS lock for `canonical_key`.
    /// Returns `Ok(Some(owner))` if locked by another process/handle, or `Ok(None)` if unlocked.
    pub fn inspect_live_lock_holder(
        lock_dir: &Path,
        canonical_key: &str,
    ) -> Result<Option<OsLockOwnerMetadata>> {
        let safe_dir = ensure_safe_lock_dir(lock_dir)?;
        let lock_path = lock_file_path_for_canonical_key(&safe_dir, canonical_key);
        if !lock_path.exists() {
            return Ok(None);
        }
        unsafe {
            let h = match open_lockfile_handle(&lock_path) {
                Ok(h) => h,
                Err(e) => {
                    if e.to_string().contains("DEVICE_LEASE_CONFLICT") {
                        return Ok(Some(OsLockOwnerMetadata {
                            pid: 0,
                            exe_path: "unknown".to_string(),
                            job_id: "unknown".to_string(),
                            device_id: canonical_key.to_string(),
                            canonical_key: canonical_key.to_string(),
                            generation: 0,
                            acquired_at_utc: String::new(),
                            expires_at_utc: String::new(),
                        }));
                    }
                    return Err(e);
                }
            };
            let mut ov = Overlapped::default();
            let locked = LockFileEx(
                h,
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                LOCK_RANGE_BYTES,
                0,
                &mut ov,
            );
            if locked != 0 {
                // Not currently held by another process/handle -> unlock immediately and return None
                let mut ov_u = Overlapped::default();
                let _ = UnlockFileEx(h, 0, LOCK_RANGE_BYTES, 0, &mut ov_u);
                let _ = CloseHandle(h);
                return Ok(None);
            }
            let err = GetLastError();
            if err == ERROR_LOCK_VIOLATION || err == ERROR_SHARING_VIOLATION {
                let meta = read_metadata_at_offset(h);
                let _ = CloseHandle(h);
                return Ok(meta);
            }
            let _ = CloseHandle(h);
            bail!(
                "DEVICE_LOCK_INIT_FAILED: LockFileEx probe failed (win32 error {}) at '{}'",
                err,
                lock_path.display()
            )
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug)]
    pub struct OsDeviceLockGuard {
        lock_path: PathBuf,
        metadata: Mutex<OsLockOwnerMetadata>,
    }

    impl OsDeviceLockGuard {
        pub fn acquire_once(lock_dir: &Path, metadata: OsLockOwnerMetadata) -> Result<Self> {
            let safe_dir = ensure_safe_lock_dir(lock_dir)?;
            let lock_path = lock_file_path_for_canonical_key(&safe_dir, &metadata.canonical_key);
            Ok(Self {
                lock_path,
                metadata: Mutex::new(metadata),
            })
        }

        pub fn update_expiry(&self, new_expires_at: DateTime<Utc>) -> Result<()> {
            self.metadata.lock().unwrap().expires_at_utc = new_expires_at.to_rfc3339();
            Ok(())
        }

        pub fn lock_path(&self) -> &Path {
            &self.lock_path
        }

        pub fn metadata_snapshot(&self) -> OsLockOwnerMetadata {
            self.metadata.lock().unwrap().clone()
        }
    }

    pub fn inspect_live_lock_holder(
        _lock_dir: &Path,
        _canonical_key: &str,
    ) -> Result<Option<OsLockOwnerMetadata>> {
        Ok(None)
    }
}

pub use platform::{inspect_live_lock_holder, OsDeviceLockGuard};

pub fn acquire_os_device_lock_with_policy(
    lock_dir: &Path,
    metadata: OsLockOwnerMetadata,
    policy: ConflictPolicy,
) -> Result<OsDeviceLockGuard> {
    match policy {
        ConflictPolicy::FailImmediately => OsDeviceLockGuard::acquire_once(lock_dir, metadata),
        ConflictPolicy::WaitUpTo(timeout) => {
            let deadline = std::time::Instant::now() + timeout;
            loop {
                match OsDeviceLockGuard::acquire_once(lock_dir, metadata.clone()) {
                    Ok(guard) => return Ok(guard),
                    Err(e) => {
                        let msg = e.to_string();
                        if !msg.starts_with("DEVICE_LEASE_CONFLICT:") {
                            // Fail closed immediately on init/permission/reparse errors
                            return Err(e);
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err(e).context(
                                "Timed out waiting for competing process device lock release",
                            );
                        }
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                }
            }
        }
    }
}

pub fn current_exe_display() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
