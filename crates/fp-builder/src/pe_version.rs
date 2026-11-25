use anyhow::{bail, Result};
use fp_bundle_contract::PeVersionInfoSpec;
use std::path::Path;

fn pad_align4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

fn push_utf16_z(buf: &mut Vec<u8>, s: &str) {
    for u in s.encode_utf16() {
        buf.extend_from_slice(&u.to_le_bytes());
    }
    buf.extend_from_slice(&0u16.to_le_bytes());
}

fn patch_u16_at(buf: &mut [u8], offset: usize, val: u16) {
    let bytes = val.to_le_bytes();
    buf[offset] = bytes[0];
    buf[offset + 1] = bytes[1];
}

fn append_version_string_entry(buf: &mut Vec<u8>, key: &str, value: &str) {
    pad_align4(buf);
    let start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes()); // wLength placeholder
    let val_utf16_len = (value.encode_utf16().count() + 1) as u16;
    buf.extend_from_slice(&val_utf16_len.to_le_bytes()); // wValueLength in WORDs
    buf.extend_from_slice(&1u16.to_le_bytes()); // wType = 1 (text)
    push_utf16_z(buf, key);
    pad_align4(buf);
    push_utf16_z(buf, value);
    let len = (buf.len() - start) as u16;
    patch_u16_at(buf, start, len);
}

/// Encodes a spec-compliant Win32 binary `VS_VERSIONINFO` (`RT_VERSION = 16`, ID `1`) payload.
pub fn encode_vs_versioninfo_bytes(spec: &PeVersionInfoSpec) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1024);

    // 1. Root VS_VERSIONINFO header
    let root_start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes()); // wLength placeholder
    buf.extend_from_slice(&52u16.to_le_bytes()); // wValueLength = sizeof(VS_FIXEDFILEINFO)
    buf.extend_from_slice(&0u16.to_le_bytes()); // wType = 0 (binary fixed info)
    push_utf16_z(&mut buf, "VS_VERSION_INFO");
    pad_align4(&mut buf);

    // VS_FIXEDFILEINFO (52 bytes = 13 DWORDs)
    let fv = PeVersionInfoSpec::parse_quad_version(&spec.file_version);
    let pv = PeVersionInfoSpec::parse_quad_version(&spec.product_version);
    let fv_ms = ((fv[0] as u32) << 16) | (fv[1] as u32);
    let fv_ls = ((fv[2] as u32) << 16) | (fv[3] as u32);
    let pv_ms = ((pv[0] as u32) << 16) | (pv[1] as u32);
    let pv_ls = ((pv[2] as u32) << 16) | (pv[3] as u32);

    for dw in [
        0xFEEF_04BDu32, // dwSignature
        0x0001_0000u32, // dwStrucVersion
        fv_ms,          // dwFileVersionMS
        fv_ls,          // dwFileVersionLS
        pv_ms,          // dwProductVersionMS
        pv_ls,          // dwProductVersionLS
        0x0000_003Fu32, // dwFileFlagsMask
        0x0000_0000u32, // dwFileFlags
        0x0004_0004u32, // dwFileOS = VOS_NT_WINDOWS32
        0x0000_0001u32, // dwFileType = VFT_APP
        0x0000_0000u32, // dwFileSubtype
        0x0000_0000u32, // dwFileDateMS
        0x0000_0000u32, // dwFileDateLS
    ] {
        buf.extend_from_slice(&dw.to_le_bytes());
    }

    // 2. StringFileInfo
    pad_align4(&mut buf);
    let sfi_start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    push_utf16_z(&mut buf, "StringFileInfo");
    pad_align4(&mut buf);

    // StringTable ("040904B0" = en-US Unicode)
    let st_start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    push_utf16_z(&mut buf, "040904B0");

    append_version_string_entry(&mut buf, "CompanyName", &spec.company_name);
    append_version_string_entry(&mut buf, "FileDescription", &spec.file_description);
    append_version_string_entry(&mut buf, "FileVersion", &spec.file_version);
    append_version_string_entry(&mut buf, "InternalName", &spec.internal_name);
    append_version_string_entry(&mut buf, "LegalCopyright", &spec.legal_copyright);
    append_version_string_entry(&mut buf, "OriginalFilename", &spec.original_filename);
    append_version_string_entry(&mut buf, "ProductName", &spec.product_name);
    append_version_string_entry(&mut buf, "ProductVersion", &spec.product_version);

    let st_len = (buf.len() - st_start) as u16;
    patch_u16_at(&mut buf, st_start, st_len);
    let sfi_len = (buf.len() - sfi_start) as u16;
    patch_u16_at(&mut buf, sfi_start, sfi_len);

    // 3. VarFileInfo ("Translation" -> 0x0409 0x04B0)
    pad_align4(&mut buf);
    let vfi_start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    push_utf16_z(&mut buf, "VarFileInfo");
    pad_align4(&mut buf);

    let var_start = buf.len();
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes()); // wValueLength = 4 bytes
    buf.extend_from_slice(&0u16.to_le_bytes()); // wType = 0 (binary)
    push_utf16_z(&mut buf, "Translation");
    pad_align4(&mut buf);
    buf.extend_from_slice(&0x0409u16.to_le_bytes());
    buf.extend_from_slice(&0x04B0u16.to_le_bytes());

    let var_len = (buf.len() - var_start) as u16;
    patch_u16_at(&mut buf, var_start, var_len);
    let vfi_len = (buf.len() - vfi_start) as u16;
    patch_u16_at(&mut buf, vfi_start, vfi_len);

    // Root length
    let root_len = (buf.len() - root_start) as u16;
    patch_u16_at(&mut buf, root_start, root_len);
    buf
}

#[cfg(windows)]
mod win_ver_res {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    type HANDLE = *mut core::ffi::c_void;
    type BOOL = i32;
    type WORD = u16;
    type DWORD = u32;
    type LPCWSTR = *const u16;

    const RT_VERSION_ID: usize = 16;

    #[link(name = "kernel32")]
    unsafe extern "system" {
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
    }

    pub fn inject_version_info_win32(exe_path: &Path, spec: &PeVersionInfoSpec) -> Result<()> {
        let payload = encode_vs_versioninfo_bytes(spec);
        let wide: Vec<u16> = OsStr::new(exe_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let h_upd = BeginUpdateResourceW(wide.as_ptr(), 0);
            if h_upd.is_null() {
                bail!(
                    "BeginUpdateResourceW(RT_VERSION) failed on {}",
                    exe_path.display()
                );
            }
            let ok = UpdateResourceW(
                h_upd,
                RT_VERSION_ID as LPCWSTR,
                1usize as LPCWSTR,
                0x0409,
                payload.as_ptr() as *const _,
                payload.len() as DWORD,
            );
            if ok == 0 {
                let _ = EndUpdateResourceW(h_upd, 1);
                bail!(
                    "UpdateResourceW(RT_VERSION #1) failed on {}",
                    exe_path.display()
                );
            }
            if EndUpdateResourceW(h_upd, 0) == 0 {
                bail!(
                    "EndUpdateResourceW(RT_VERSION) failed on {}",
                    exe_path.display()
                );
            }
        }
        Ok(())
    }
}

/// Injects Windows PE `VERSIONINFO` (`RT_VERSION = 16`, ID `1`) into `exe_path` BEFORE Authenticode signing.
pub fn inject_pe_version_info(exe_path: &Path, spec: &PeVersionInfoSpec) -> Result<()> {
    #[cfg(windows)]
    {
        win_ver_res::inject_version_info_win32(exe_path, spec)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe_path, spec);
        bail!("PE VERSIONINFO injection requires Windows")
    }
}
