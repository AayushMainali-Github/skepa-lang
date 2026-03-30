#[cfg(unix)]
use std::ffi::{c_char, c_int, c_void, CStr, CString};
#[cfg(windows)]
use std::ffi::{c_void, CString};

pub struct RtForeignLibrary {
    handle: *mut c_void,
}

impl RtForeignLibrary {
    pub fn open(path: &str) -> Result<Self, String> {
        #[cfg(unix)]
        {
            let c_path = CString::new(path).map_err(|_| "library path contains NUL byte")?;
            // SAFETY: c_path is a valid NUL-terminated string for libc.
            let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW) };
            if handle.is_null() {
                return Err(last_error_message("failed to load shared library"));
            }
            Ok(Self { handle })
        }
        #[cfg(windows)]
        {
            let c_path = CString::new(path).map_err(|_| "library path contains NUL byte")?;
            // SAFETY: c_path is a valid NUL-terminated string for the OS loader.
            let handle = unsafe { LoadLibraryA(c_path.as_ptr().cast()) };
            if handle.is_null() {
                return Err("failed to load shared library".to_string());
            }
            Ok(Self {
                handle: handle.cast(),
            })
        }
    }

    pub fn bind(&self, symbol: &str) -> Result<*mut c_void, String> {
        #[cfg(unix)]
        {
            let c_symbol = CString::new(symbol).map_err(|_| "symbol name contains NUL byte")?;
            // SAFETY: self.handle came from dlopen and c_symbol is valid.
            let ptr = unsafe { dlsym(self.handle, c_symbol.as_ptr()) };
            if ptr.is_null() {
                return Err(last_error_message("failed to bind symbol"));
            }
            Ok(ptr)
        }
        #[cfg(windows)]
        {
            let c_symbol = CString::new(symbol).map_err(|_| "symbol name contains NUL byte")?;
            // SAFETY: self.handle came from LoadLibraryA and c_symbol is valid.
            let ptr = unsafe { GetProcAddress(self.handle.cast(), c_symbol.as_ptr().cast()) };
            if ptr.is_null() {
                return Err("failed to bind symbol".to_string());
            }
            Ok(ptr.cast())
        }
    }
}

impl Drop for RtForeignLibrary {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        #[cfg(unix)]
        unsafe {
            let _ = dlclose(self.handle);
        }
        #[cfg(windows)]
        unsafe {
            let _ = FreeLibrary(self.handle.cast());
        }
    }
}

unsafe impl Send for RtForeignLibrary {}
unsafe impl Sync for RtForeignLibrary {}

pub struct RtForeignSymbol {
    pub library_handle: usize,
    pub ptr: *mut c_void,
}

unsafe impl Send for RtForeignSymbol {}
unsafe impl Sync for RtForeignSymbol {}

#[cfg(unix)]
const RTLD_NOW: c_int = 2;

#[cfg(all(unix, not(target_os = "macos")))]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlerror() -> *const c_char;
}

#[cfg(unix)]
fn last_error_message(default: &str) -> String {
    // SAFETY: dlerror returns either null or a valid C string for the current thread.
    let err = unsafe { dlerror() };
    if err.is_null() {
        default.to_string()
    } else {
        // SAFETY: non-null error pointer refers to a NUL-terminated string.
        unsafe { CStr::from_ptr(err) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn LoadLibraryA(lp_lib_file_name: *const u8) -> *mut c_void;
    fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
}
