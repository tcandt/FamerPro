use anyhow::{bail, Context, Result};
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_exe, PE_ED25519_PUBKEY_RESOURCE_ID, PE_RT_RCDATA_TYPE_ID,
};
use std::path::Path;

#[cfg(windows)]
mod win_pe {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut core::ffi::c_void;
    type HMODULE = *mut core::ffi::c_void;
    type HRSRC = *mut core::ffi::c_void;
    type BOOL = i32;
    type WORD = u16;
    type DWORD = u32;
    type LPCWSTR = *const u16;

    const RT_ICON_ID: usize = 3;
    const RT_GROUP_ICON_ID: usize = 14;
    const LOAD_LIBRARY_AS_DATAFILE: DWORD = 0x00000002;

    #[link(name = "kernel32")]
    extern "system" {
        fn BeginUpdateResourceW(pFileName: LPCWSTR, bDeleteExistingResources: BOOL) -> HANDLE;
        fn UpdateResourceW(
            hUpdate: HANDLE,
            lpType: LPCWSTR,
            lpName: LPCWSTR,
            wLanguage: WORD,
            lpData: *const core::ffi::c_void,
            cb: DWORD,
        ) -> BOOL;
        fn EndUpdateResourceW(hUpdate: HANDLE, fDiscard: BOOL) -> BOOL;
        fn LoadLibraryExW(lpLibFileName: LPCWSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE;
        fn FindResourceW(hModule: HMODULE, lpName: LPCWSTR, lpType: LPCWSTR) -> HRSRC;
        fn SizeofResource(hModule: HMODULE, hResInfo: HRSRC) -> DWORD;
        fn FreeLibrary(hLibModule: HMODULE) -> BOOL;
    }

    fn to_wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    const fn make_int_resource(id: usize) -> LPCWSTR {
        id as LPCWSTR
    }

    /// Injects the producer's 32-byte Ed25519 Public Key (64 hex chars) into the target PE `.exe`
    /// as a Windows `RT_RCDATA` (#10) resource at ID `7777` (`PE_ED25519_PUBKEY_RESOURCE_ID`).
    /// This anchors the Root of Trust inside the executable binary so an attacker cannot swap
    /// `bundle/trusted_publisher.pub` on disk.
    pub fn inject_ed25519_pubkey_win32(exe_path: &Path, pubkey_hex: &str) -> Result<()> {
        let clean_hex = pubkey_hex.trim();
        if clean_hex.len() != 64 {
            bail!(
                "Invalid Ed25519 public key hex length for PE resource injection: {}",
                clean_hex.len()
            );
        }
        let exe_wide = to_wide(exe_path.as_os_str());
        let payload = clean_hex.as_bytes();

        unsafe {
            let h_update = BeginUpdateResourceW(exe_wide.as_ptr(), 0);
            if h_update.is_null() {
                bail!(
                    "BeginUpdateResourceW(RT_RCDATA pubkey) failed on {}",
                    exe_path.display()
                );
            }
            let ok = UpdateResourceW(
                h_update,
                make_int_resource(PE_RT_RCDATA_TYPE_ID),
                make_int_resource(PE_ED25519_PUBKEY_RESOURCE_ID),
                0x0409,
                payload.as_ptr() as *const _,
                payload.len() as DWORD,
            );
            if ok == 0 {
                let _ = EndUpdateResourceW(h_update, 1);
                bail!(
                    "UpdateResourceW(RT_RCDATA #{}) failed",
                    PE_ED25519_PUBKEY_RESOURCE_ID
                );
            }
            if EndUpdateResourceW(h_update, 0) == 0 {
                bail!("EndUpdateResourceW failed to commit PE Ed25519 public key resource");
            }
        }
        Ok(())
    }

    /// Parses a standard Windows `.ico` file and injects `RT_ICON` + `RT_GROUP_ICON` resources
    /// directly into the target PE `.exe` file BEFORE any Authenticode signing step.
    pub fn inject_ico_win32(exe_path: &Path, ico_path: &Path) -> Result<usize> {
        let ico_bytes = std::fs::read(ico_path)
            .with_context(|| format!("Failed to read .ico file: {}", ico_path.display()))?;
        if ico_bytes.len() < 6 {
            bail!("Invalid .ico file (too small): {}", ico_path.display());
        }

        let reserved = u16::from_le_bytes([ico_bytes[0], ico_bytes[1]]);
        let ico_type = u16::from_le_bytes([ico_bytes[2], ico_bytes[3]]);
        let count = u16::from_le_bytes([ico_bytes[4], ico_bytes[5]]) as usize;
        if reserved != 0 || ico_type != 1 || count == 0 {
            bail!("Invalid .ico header in {}", ico_path.display());
        }
        if ico_bytes.len() < 6 + count * 16 {
            bail!("Truncated .ico directory in {}", ico_path.display());
        }

        // Build GRPICONDIR (6 bytes header + count * 14 bytes GRPICONDIRENTRY)
        let mut grp_data = Vec::with_capacity(6 + count * 14);
        grp_data.extend_from_slice(&ico_bytes[0..6]);

        let mut icon_payloads: Vec<(u16, Vec<u8>)> = Vec::with_capacity(count);

        for i in 0..count {
            let entry_off = 6 + i * 16;
            let entry = &ico_bytes[entry_off..entry_off + 16];
            let bytes_in_res =
                u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
            let image_offset =
                u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;

            if image_offset + bytes_in_res > ico_bytes.len() {
                bail!("Corrupt .ico entry #{} out of bounds", i);
            }

            let icon_id = (i + 1) as u16;
            // First 12 bytes of GRPICONDIRENTRY are identical to ICONDIRENTRY
            grp_data.extend_from_slice(&entry[0..12]);
            // Last 2 bytes are the resource ID (nID)
            grp_data.extend_from_slice(&icon_id.to_le_bytes());

            let payload = ico_bytes[image_offset..image_offset + bytes_in_res].to_vec();
            icon_payloads.push((icon_id, payload));
        }

        let exe_wide = to_wide(exe_path.as_os_str());
        unsafe {
            let h_update = BeginUpdateResourceW(exe_wide.as_ptr(), 0);
            if h_update.is_null() {
                bail!("BeginUpdateResourceW failed on {}", exe_path.display());
            }

            // Write each RT_ICON layer (language 0x0409 = en-US, standard neutral/US resource slot)
            for (icon_id, payload) in &icon_payloads {
                let ok = UpdateResourceW(
                    h_update,
                    make_int_resource(RT_ICON_ID),
                    make_int_resource(*icon_id as usize),
                    0x0409,
                    payload.as_ptr() as *const _,
                    payload.len() as DWORD,
                );
                if ok == 0 {
                    let _ = EndUpdateResourceW(h_update, 1);
                    bail!("UpdateResourceW(RT_ICON #{}) failed", icon_id);
                }
            }

            // Write RT_GROUP_ICON at slot 1 and slot 32512 (IDI_APPLICATION standard Windows icon slots)
            for group_id in [1usize, 32512usize] {
                let ok = UpdateResourceW(
                    h_update,
                    make_int_resource(RT_GROUP_ICON_ID),
                    make_int_resource(group_id),
                    0x0409,
                    grp_data.as_ptr() as *const _,
                    grp_data.len() as DWORD,
                );
                if ok == 0 {
                    let _ = EndUpdateResourceW(h_update, 1);
                    bail!("UpdateResourceW(RT_GROUP_ICON #{}) failed", group_id);
                }
            }

            if EndUpdateResourceW(h_update, 0) == 0 {
                bail!("EndUpdateResourceW failed to commit PE icon resources");
            }
        }

        Ok(count)
    }

    /// Opens the compiled `.exe` as a Win32 datafile module and verifies the embedded `RT_GROUP_ICON`
    /// and `RT_ICON` resources exist and have non-zero byte sizes.
    pub fn verify_pe_icon_win32(exe_path: &Path) -> Result<(u32, u32)> {
        let exe_wide = to_wide(exe_path.as_os_str());
        unsafe {
            let h_mod = LoadLibraryExW(
                exe_wide.as_ptr(),
                core::ptr::null_mut(),
                LOAD_LIBRARY_AS_DATAFILE,
            );
            if h_mod.is_null() {
                bail!("LoadLibraryExW failed on {}", exe_path.display());
            }

            let h_grp = FindResourceW(
                h_mod,
                make_int_resource(1),
                make_int_resource(RT_GROUP_ICON_ID),
            );
            let grp_size = if !h_grp.is_null() {
                SizeofResource(h_mod, h_grp)
            } else {
                0
            };

            let h_ico1 = FindResourceW(h_mod, make_int_resource(1), make_int_resource(RT_ICON_ID));
            let ico1_size = if !h_ico1.is_null() {
                SizeofResource(h_mod, h_ico1)
            } else {
                0
            };

            let _ = FreeLibrary(h_mod);

            if grp_size == 0 || ico1_size == 0 {
                bail!(
                    "PE Icon verification failed on {}: grp_size={}, ico1_size={}",
                    exe_path.display(),
                    grp_size,
                    ico1_size
                );
            }
            Ok((grp_size, ico1_size))
        }
    }
}

pub fn inject_ico_into_pe_exe(exe_path: &Path, ico_path: &Path) -> Result<usize> {
    #[cfg(windows)]
    {
        win_pe::inject_ico_win32(exe_path, ico_path)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe_path, ico_path);
        Ok(0)
    }
}

pub fn verify_pe_icon_resource(exe_path: &Path) -> Result<(u32, u32)> {
    #[cfg(windows)]
    {
        win_pe::verify_pe_icon_win32(exe_path)
    }
    #[cfg(not(windows))]
    {
        let _ = exe_path;
        Ok((0, 0))
    }
}

pub fn inject_ed25519_pubkey_into_pe_exe(exe_path: &Path, pubkey_hex: &str) -> Result<()> {
    #[cfg(windows)]
    {
        win_pe::inject_ed25519_pubkey_win32(exe_path, pubkey_hex)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe_path, pubkey_hex);
        Ok(())
    }
}

pub fn verify_pe_embedded_pubkey_resource(
    exe_path: &Path,
    expected_pubkey_hex: &str,
) -> Result<String> {
    let embedded = read_pe_embedded_pubkey_from_exe(exe_path)?;
    if !embedded.eq_ignore_ascii_case(expected_pubkey_hex.trim()) {
        bail!(
            "PE_ROOT_OF_TRUST_VERIFY_MISMATCH: Embedded RT_RCDATA #{} key '{}' != expected '{}'",
            PE_ED25519_PUBKEY_RESOURCE_ID,
            embedded,
            expected_pubkey_hex.trim()
        );
    }
    Ok(embedded)
}
