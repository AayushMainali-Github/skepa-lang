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

impl RtForeignSymbol {
    pub fn call_0_int(&self) -> i64 {
        // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
        let function: unsafe extern "C" fn() -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { function() }
    }

    pub fn call_1_int(&self, value: i64) -> i64 {
        // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
        let function: unsafe extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(self.ptr) };
        unsafe { function(value) }
    }

    pub fn call_1_string_int(&self, value: &str) -> Result<i64, String> {
        let c_value = CString::new(value).map_err(|_| "string argument contains NUL byte")?;
        #[cfg(windows)]
        {
            // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
            let function: unsafe extern "system" fn(*const i8) -> i32 =
                unsafe { std::mem::transmute(self.ptr) };
            Ok(unsafe { function(c_value.as_ptr()) as i64 })
        }
        #[cfg(unix)]
        {
            // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
            let function: unsafe extern "C" fn(*const c_char) -> usize =
                unsafe { std::mem::transmute(self.ptr) };
            Ok(unsafe { function(c_value.as_ptr()) as i64 })
        }
    }

    pub fn call_1_bytes_int(&self, value: &[u8]) -> i64 {
        #[cfg(windows)]
        {
            // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
            let function: unsafe extern "system" fn(*const u8, i64) -> i32 =
                unsafe { std::mem::transmute(self.ptr) };
            unsafe { function(value.as_ptr(), value.len() as i64) as i64 }
        }
        #[cfg(unix)]
        {
            // SAFETY: caller guarantees the symbol uses the expected ABI/signature.
            let function: unsafe extern "C" fn(*const u8, usize) -> usize =
                unsafe { std::mem::transmute(self.ptr) };
            unsafe { function(value.as_ptr(), value.len()) as i64 }
        }
    }
}

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
