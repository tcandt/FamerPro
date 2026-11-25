use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const CRED_MGR_URI_PREFIX: &str = "CRED_MGR://";
pub const MAX_CREDENTIAL_BLOB_BYTES: usize = 2560;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", content = "slug", rename_all = "snake_case")]
pub enum SecretNamespace {
    Project(String),
    MiniTool(String),
    Account(String),
    System,
}

impl SecretNamespace {
    pub fn prefix(&self) -> Result<String> {
        match self {
            SecretNamespace::Project(slug) => {
                validate_segment(slug, "project_slug")?;
                Ok(format!("FarmerPro/project/{}/", slug))
            }
            SecretNamespace::MiniTool(slug) => {
                validate_segment(slug, "tool_slug")?;
                Ok(format!("FarmerPro/minitool/{}/", slug))
            }
            SecretNamespace::Account(project_slug) => {
                validate_segment(project_slug, "project_slug")?;
                Ok(format!("FarmerPro/account/{}/", project_slug))
            }
            SecretNamespace::System => Ok("FarmerPro/system/".to_string()),
        }
    }

    pub fn label(&self) -> String {
        match self {
            SecretNamespace::Project(slug) => format!("project:{}", slug),
            SecretNamespace::MiniTool(slug) => format!("minitool:{}", slug),
            SecretNamespace::Account(slug) => format!("account:{}", slug),
            SecretNamespace::System => "system".to_string(),
        }
    }

    pub fn target_name(&self, key: &str) -> Result<String> {
        validate_segment(key, "secret_key")?;
        Ok(format!("{}{}", self.prefix()?, key))
    }

    pub fn uri(&self, key: &str) -> Result<String> {
        Ok(format!("{}{}", CRED_MGR_URI_PREFIX, self.target_name(key)?))
    }
}

pub fn validate_segment(value: &str, field_name: &str) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{} cannot be empty", field_name);
    }
    if trimmed != value {
        bail!(
            "{} must not contain leading/trailing whitespace",
            field_name
        );
    }
    if trimmed.contains("..") {
        bail!("{} must not contain '..'", field_name);
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        bail!(
            "{} '{}' contains invalid characters (allowed: [a-zA-Z0-9._-])",
            field_name,
            value
        );
    }
    Ok(())
}

pub fn parse_cred_uri_to_target(uri_or_target: &str) -> Result<String> {
    let raw = uri_or_target.trim();
    if raw.is_empty() {
        bail!("Credential target or URI cannot be empty");
    }
    let target = raw.strip_prefix(CRED_MGR_URI_PREFIX).unwrap_or(raw);
    if target.contains('*') || target.contains('?') || target.contains("..") {
        bail!("Invalid credential target '{}'", target);
    }
    if !target.starts_with("FarmerPro/") {
        return Ok(format!("FarmerPro/legacy/{}", target));
    }
    Ok(target.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialMetadata {
    pub target_name: String,
    pub cred_uri: String,
    pub namespace: String,
    pub key: String,
    pub backend: String,
}

#[cfg(windows)]
pub mod os_wincred {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;
    const ERROR_NOT_FOUND: u32 = 1168;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct FILETIME {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }

    #[repr(C)]
    struct CREDENTIALW {
        flags: u32,
        cred_type: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: FILETIME,
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut core::ffi::c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const CREDENTIALW, flags: u32) -> i32;
        fn CredReadW(
            target_name: *const u16,
            cred_type: u32,
            flags: u32,
            credential: *mut *mut CREDENTIALW,
        ) -> i32;
        fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> i32;
        fn CredEnumerateW(
            filter: *const u16,
            flags: u32,
            count: *mut u32,
            credentials: *mut *mut *mut CREDENTIALW,
        ) -> i32;
        fn CredFree(buffer: *const core::ffi::c_void);
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }

    pub fn write_generic_secret(
        target_name: &str,
        user_name: &str,
        comment: &str,
        secret: &str,
    ) -> Result<()> {
        let secret_bytes = secret.as_bytes();
        if secret_bytes.len() > MAX_CREDENTIAL_BLOB_BYTES {
            bail!(
                "Secret exceeds Windows Credential Manager limit ({} > {} bytes)",
                secret_bytes.len(),
                MAX_CREDENTIAL_BLOB_BYTES
            );
        }

        let mut target_w = to_wide(target_name);
        let mut user_w = to_wide(user_name);
        let mut comment_w = to_wide(comment);
        let mut blob = secret_bytes.to_vec();

        let cred = CREDENTIALW {
            flags: 0,
            cred_type: CRED_TYPE_GENERIC,
            target_name: target_w.as_mut_ptr(),
            comment: comment_w.as_mut_ptr(),
            last_written: FILETIME {
                dw_low_date_time: 0,
                dw_high_date_time: 0,
            },
            credential_blob_size: blob.len() as u32,
            credential_blob: if blob.is_empty() {
                std::ptr::null_mut()
            } else {
                blob.as_mut_ptr()
            },
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: std::ptr::null_mut(),
            target_alias: std::ptr::null_mut(),
            user_name: user_w.as_mut_ptr(),
        };

        let ok = unsafe { CredWriteW(&cred, 0) };
        blob.fill(0);

        if ok == 0 {
            let err = unsafe { GetLastError() };
            bail!(
                "Win32 CredWriteW failed for '{}' (GetLastError={})",
                target_name,
                err
            );
        }
        Ok(())
    }

    pub fn read_generic_secret(target_name: &str) -> Result<Option<String>> {
        let target_w = to_wide(target_name);
        let mut p_cred: *mut CREDENTIALW = std::ptr::null_mut();

        let ok = unsafe { CredReadW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p_cred) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(None);
            }
            bail!(
                "Win32 CredReadW failed for '{}' (GetLastError={})",
                target_name,
                err
            );
        }

        let res = unsafe {
            let cred = &*p_cred;
            if cred.credential_blob.is_null() || cred.credential_blob_size == 0 {
                Ok(Some(String::new()))
            } else {
                let bytes = std::slice::from_raw_parts(
                    cred.credential_blob,
                    cred.credential_blob_size as usize,
                );
                Ok(Some(String::from_utf8(bytes.to_vec())?))
            }
        };

        unsafe {
            CredFree(p_cred as *const core::ffi::c_void);
        }
        res
    }

    pub fn delete_generic_secret(target_name: &str) -> Result<bool> {
        let target_w = to_wide(target_name);
        let ok = unsafe { CredDeleteW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(false);
            }
            bail!(
                "Win32 CredDeleteW failed for '{}' (GetLastError={})",
                target_name,
                err
            );
        }
        Ok(true)
    }

    pub fn enumerate_generic_targets(prefix_filter: &str) -> Result<Vec<String>> {
        let filter_pattern = if prefix_filter.ends_with('*') {
            prefix_filter.to_string()
        } else {
            format!("{}*", prefix_filter)
        };
        let filter_w = to_wide(&filter_pattern);
        let mut count: u32 = 0;
        let mut p_creds: *mut *mut CREDENTIALW = std::ptr::null_mut();

        let ok = unsafe { CredEnumerateW(filter_w.as_ptr(), 0, &mut count, &mut p_creds) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(Vec::new());
            }
            bail!(
                "Win32 CredEnumerateW failed for '{}' (GetLastError={})",
                filter_pattern,
                err
            );
        }

        let mut targets = Vec::with_capacity(count as usize);
        unsafe {
            let slice = std::slice::from_raw_parts(p_creds, count as usize);
            for &cred_ptr in slice {
                if !cred_ptr.is_null() {
                    let cred = &*cred_ptr;
                    if cred.cred_type == CRED_TYPE_GENERIC && !cred.target_name.is_null() {
                        targets.push(wide_ptr_to_string(cred.target_name));
                    }
                }
            }
            CredFree(p_creds as *const core::ffi::c_void);
        }

        targets.sort();
        Ok(targets)
    }
}
