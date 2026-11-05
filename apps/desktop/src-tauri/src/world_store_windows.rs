//! Windows-only OS primitives. No prompts, machine-scope flag, custom key or
//! credential-manager entry. API errors contain only stage and numeric code.
use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::null_mut,
};
use zeroize::{Zeroize, Zeroizing};
#[repr(C)]
struct Blob {
    len: u32,
    data: *mut u8,
}
#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        input: *const Blob,
        description: *const u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *const Blob,
        description: *mut *mut u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(p: *mut c_void) -> *mut c_void;
    fn GetLastError() -> u32;
    fn CreateMutexW(attributes: *mut c_void, owner: i32, name: *const u16) -> *mut c_void;
    fn WaitForSingleObject(handle: *mut c_void, ms: u32) -> u32;
    fn ReleaseMutex(handle: *mut c_void) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
}
#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        id: *const Guid,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> i32;
}
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(p: *mut c_void);
}
fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain(Some(0)).collect()
}
fn last(stage: &str) -> String {
    let code = unsafe { GetLastError() };
    format!("World store {stage} failed (os={code})")
}
struct Allocation(Blob);
impl Drop for Allocation {
    fn drop(&mut self) {
        if !self.0.data.is_null() {
            unsafe {
                std::slice::from_raw_parts_mut(self.0.data, self.0.len as usize).zeroize();
                LocalFree(self.0.data.cast());
            }
        }
    }
}
pub(super) fn crypt(
    input: &[u8],
    entropy: &[u8],
    protect: bool,
) -> Result<Zeroizing<Vec<u8>>, String> {
    let source = Blob {
        len: input
            .len()
            .try_into()
            .map_err(|_| "World DPAPI input bound")?,
        data: input.as_ptr().cast_mut(),
    };
    let entropy = Blob {
        len: entropy
            .len()
            .try_into()
            .map_err(|_| "World DPAPI entropy bound")?,
        data: entropy.as_ptr().cast_mut(),
    };
    let mut out = Allocation(Blob {
        len: 0,
        data: null_mut(),
    });
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &source,
                std::ptr::null(),
                &entropy,
                null_mut(),
                null_mut(),
                1,
                &mut out.0,
            )
        } else {
            CryptUnprotectData(
                &source,
                null_mut(),
                &entropy,
                null_mut(),
                null_mut(),
                1,
                &mut out.0,
            )
        }
    };
    if ok == 0 {
        return Err(last(if protect {
            "DPAPI protect"
        } else {
            "DPAPI unprotect"
        }));
    }
    if out.0.data.is_null() || out.0.len == 0 || out.0.len as usize > super::MAX_CIPHER {
        return Err("World DPAPI output bound".into());
    }
    Ok(Zeroizing::new(
        unsafe { std::slice::from_raw_parts(out.0.data, out.0.len as usize) }.to_vec(),
    ))
}
pub(super) struct Lock(*mut c_void);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}
pub(super) fn lock(name: &str) -> Result<Lock, String> {
    let handle = unsafe { CreateMutexW(null_mut(), 0, wide(std::ffi::OsStr::new(name)).as_ptr()) };
    if handle.is_null() {
        return Err(last("mutex create"));
    }
    let wait = unsafe { WaitForSingleObject(handle, 2000) };
    if matches!(wait, 0 | 0x80) {
        Ok(Lock(handle))
    } else {
        let error = if wait == 258 {
            "World store lock deadline".into()
        } else {
            last("mutex wait")
        };
        unsafe { CloseHandle(handle) };
        Err(error)
    }
}
pub(super) fn publish(stage: &Path, target: &Path) -> Result<(), String> {
    // Same-directory rename, no COPY_ALLOWED and no delete of the committed
    // target. WRITE_THROUGH is a supported MoveFileExW flag, not ReplaceFileW's
    // unsupported flag. Staged file data is separately flushed before this call.
    if unsafe {
        MoveFileExW(
            wide(stage.as_os_str()).as_ptr(),
            wide(target.as_os_str()).as_ptr(),
            1 | 8,
        )
    } == 0
    {
        return Err(last("atomic publication"));
    }
    Ok(())
}
pub(super) fn user_root() -> Result<PathBuf, String> {
    let local = Guid {
        a: 0xf1b32785,
        b: 0x6fba,
        c: 0x4fcf,
        d: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
    };
    let mut path = null_mut();
    let code = unsafe { SHGetKnownFolderPath(&local, 0, null_mut(), &mut path) };
    if code < 0 || path.is_null() {
        return Err(format!("World local app data unavailable (hr={code})"));
    }
    let mut len = 0;
    unsafe {
        while *path.add(len) != 0 && len < 32768 {
            len += 1;
        }
    }
    let value = if len < 32768 {
        String::from_utf16(unsafe { std::slice::from_raw_parts(path, len) })
            .map(PathBuf::from)
            .map_err(|_| "World local app data encoding".to_string())
    } else {
        Err("World local app data bound".into())
    };
    unsafe { CoTaskMemFree(path.cast()) };
    Ok(value?.join("NoitaTogether2").join("lc1-world-records-v1"))
}
