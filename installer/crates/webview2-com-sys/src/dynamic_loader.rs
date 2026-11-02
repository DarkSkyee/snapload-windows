use std::ffi::c_void;
use std::sync::OnceLock;

#[cfg(target_arch = "x86_64")]
const WEBVIEW2_LOADER_BYTES: &[u8] = include_bytes!("../x64/WebView2Loader.dll");

#[cfg(target_arch = "x86")]
const WEBVIEW2_LOADER_BYTES: &[u8] = include_bytes!("../x86/WebView2Loader.dll");

#[cfg(target_arch = "aarch64")]
const WEBVIEW2_LOADER_BYTES: &[u8] = include_bytes!("../arm64/WebView2Loader.dll");

extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn ensure_loader_loaded() -> Option<*mut c_void> {
    let dll_name_wide = to_wide("WebView2Loader.dll");

    // 1. Check if already loaded in this process
    let h = unsafe { GetModuleHandleW(dll_name_wide.as_ptr()) };
    if !h.is_null() {
        return Some(h);
    }

    // 2. Try loading from standard search path (e.g. if placed next to exe)
    let h = unsafe { LoadLibraryW(dll_name_wide.as_ptr()) };
    if !h.is_null() {
        return Some(h);
    }

    // 3. Extract embedded bytes to %TEMP%\SnapLoadSetup\WebView2Loader.dll
    let temp_dir = std::env::temp_dir().join("SnapLoadSetup");
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_dll = temp_dir.join("WebView2Loader.dll");

    let should_write = match std::fs::metadata(&target_dll) {
        Ok(meta) => meta.len() != WEBVIEW2_LOADER_BYTES.len() as u64,
        Err(_) => true,
    };

    if should_write {
        if let Err(_) = std::fs::write(&target_dll, WEBVIEW2_LOADER_BYTES) {
            let alt_dll = temp_dir.join(format!("WebView2Loader_{}.dll", std::process::id()));
            if std::fs::write(&alt_dll, WEBVIEW2_LOADER_BYTES).is_ok() {
                let alt_wide = to_wide(&alt_dll.to_string_lossy());
                let h = unsafe { LoadLibraryW(alt_wide.as_ptr()) };
                if !h.is_null() {
                    return Some(h);
                }
            }
        }
    }

    let target_wide = to_wide(&target_dll.to_string_lossy());
    let h = unsafe { LoadLibraryW(target_wide.as_ptr()) };
    if !h.is_null() {
        let dir_wide = to_wide(&temp_dir.to_string_lossy());
        unsafe { SetDllDirectoryW(dir_wide.as_ptr()) };
        return Some(h);
    }

    None
}

fn get_loader_module() -> Option<*mut c_void> {
    static MODULE: OnceLock<usize> = OnceLock::new();
    let mod_val = *MODULE.get_or_init(|| match ensure_loader_loaded() {
        Some(ptr) => ptr as usize,
        None => 0,
    });
    if mod_val != 0 {
        Some(mod_val as *mut c_void)
    } else {
        None
    }
}

pub unsafe fn get_proc_address(fn_name: &str) -> Option<*mut c_void> {
    let module = get_loader_module()?;
    let c_name = std::ffi::CString::new(fn_name).ok()?;
    let proc = GetProcAddress(module, c_name.as_ptr() as *const u8);
    if !proc.is_null() {
        Some(proc)
    } else {
        None
    }
}
