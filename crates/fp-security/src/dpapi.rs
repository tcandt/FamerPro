use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

#[cfg(windows)]
mod win_dpapi {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    struct DATA_BLOB {
        cb_data: u32,
        pb_data: *mut u8,
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x00000001;

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            p_data_in: *const DATA_BLOB,
            sz_data_descr: *const u16,
            p_optional_entropy: *const DATA_BLOB,
            pv_reserved: *const core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DATA_BLOB,
        ) -> i32;

        fn CryptUnprotectData(
            p_data_in: *const DATA_BLOB,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DATA_BLOB,
            pv_reserved: *const core::ffi::c_void,
            p_prompt_struct: *const core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DATA_BLOB,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn protect(plaintext: &[u8], entropy: Option<&[u8]>, description: &str) -> Result<Vec<u8>> {
        if plaintext.is_empty() {
            bail!("DPAPI protect refused empty plaintext");
        }
        let mut in_buf = plaintext.to_vec();
        let in_blob = DATA_BLOB {
            cb_data: u32::try_from(in_buf.len()).context("Plaintext too large for DPAPI")?,
            pb_data: in_buf.as_mut_ptr(),
        };

        let mut entropy_buf = entropy.map(|e| e.to_vec());
        let entropy_blob = entropy_buf.as_mut().map(|buf| DATA_BLOB {
            cb_data: buf.len() as u32,
            pb_data: buf.as_mut_ptr(),
        });

        let desc_w = to_wide(description);
        let mut out_blob = DATA_BLOB {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };

        let ok = unsafe {
            CryptProtectData(
                &in_blob,
                desc_w.as_ptr(),
                entropy_blob
                    .as_ref()
                    .map(|b| b as *const DATA_BLOB)
                    .unwrap_or(std::ptr::null()),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        // Best-effort zeroize temporary plaintext buffer
        in_buf.fill(0);

        if ok == 0 {
            let err = unsafe { GetLastError() };
            bail!("Win32 CryptProtectData failed (GetLastError={})", err);
        }

        let protected = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pb_data, out_blob.cb_data as usize);
            let vec = slice.to_vec();
            let _ = LocalFree(out_blob.pb_data as *mut core::ffi::c_void);
            vec
        };

        Ok(protected)
    }

    pub fn unprotect(ciphertext: &[u8], entropy: Option<&[u8]>) -> Result<Vec<u8>> {
        if ciphertext.is_empty() {
            bail!("DPAPI unprotect refused empty ciphertext");
        }
        let mut in_buf = ciphertext.to_vec();
        let in_blob = DATA_BLOB {
            cb_data: u32::try_from(in_buf.len()).context("Ciphertext too large for DPAPI")?,
            pb_data: in_buf.as_mut_ptr(),
        };

        let mut entropy_buf = entropy.map(|e| e.to_vec());
        let entropy_blob = entropy_buf.as_mut().map(|buf| DATA_BLOB {
            cb_data: buf.len() as u32,
            pb_data: buf.as_mut_ptr(),
        });

        let mut out_blob = DATA_BLOB {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };

        let ok = unsafe {
            CryptUnprotectData(
                &in_blob,
                std::ptr::null_mut(),
                entropy_blob
                    .as_ref()
                    .map(|b| b as *const DATA_BLOB)
                    .unwrap_or(std::ptr::null()),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };

        if ok == 0 {
            let err = unsafe { GetLastError() };
            bail!(
                "Win32 CryptUnprotectData failed (GetLastError={}) — blob corrupted or encrypted by a different Windows user",
                err
            );
        }

        let plaintext = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pb_data, out_blob.cb_data as usize);
            let vec = slice.to_vec();
            std::ptr::write_bytes(out_blob.pb_data, 0, out_blob.cb_data as usize);
            let _ = LocalFree(out_blob.pb_data as *mut core::ffi::c_void);
            vec
        };

        Ok(plaintext)
    }
}

pub fn protect_bytes(
    plaintext: &[u8],
    optional_entropy: Option<&[u8]>,
    description: &str,
) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        win_dpapi::protect(plaintext, optional_entropy, description)
    }
    #[cfg(not(windows))]
    {
        let _ = (plaintext, optional_entropy, description);
        bail!("Windows DPAPI is only supported on Windows targets");
    }
}

pub fn unprotect_bytes(ciphertext: &[u8], optional_entropy: Option<&[u8]>) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        win_dpapi::unprotect(ciphertext, optional_entropy)
    }
    #[cfg(not(windows))]
    {
        let _ = (ciphertext, optional_entropy);
        bail!("Windows DPAPI is only supported on Windows targets");
    }
}

pub fn protect_base64(
    plaintext: &[u8],
    optional_entropy: Option<&[u8]>,
    description: &str,
) -> Result<String> {
    let cipher = protect_bytes(plaintext, optional_entropy, description)?;
    Ok(B64.encode(cipher))
}

pub fn unprotect_base64(ciphertext_b64: &str, optional_entropy: Option<&[u8]>) -> Result<Vec<u8>> {
    let cipher = B64
        .decode(ciphertext_b64.trim())
        .context("Invalid base64 DPAPI ciphertext")?;
    unprotect_bytes(&cipher, optional_entropy)
}
