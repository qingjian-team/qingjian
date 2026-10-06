//! 本机动态库寿命；Windows 使用受限 DLL 搜索，只解析显式的绝对路径。
use super::RimeError;
use std::ffi::c_int;
use std::path::Path;

pub(super) struct Library {
    #[cfg(windows)]
    handle: usize,

    #[cfg(not(windows))]
    inner: libloading::Library,
}

impl Library {
    pub unsafe fn new(path: &Path) -> Result<Self, RimeError> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let handle = unsafe { LoadLibraryExW(name.as_ptr(), 0, 0x100 | 0x800) };
            if handle == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(Self { handle })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {
                inner: unsafe { libloading::Library::new(path)? },
            })
        }
    }

    pub unsafe fn api_getter(&self) -> Result<unsafe extern "C" fn() -> *const c_int, RimeError> {
        #[cfg(windows)]
        {
            let function = unsafe { GetProcAddress(self.handle, c"rime_get_api".as_ptr().cast()) };
            if function.is_null() {
                return Err(RimeError::Api("rime_get_api"));
            }
            Ok(unsafe {
                std::mem::transmute::<*const std::ffi::c_void, unsafe extern "C" fn() -> *const c_int>(
                    function,
                )
            })
        }
        #[cfg(not(windows))]
        {
            Ok(*unsafe {
                self.inner
                    .get::<unsafe extern "C" fn() -> *const c_int>(b"rime_get_api\0")?
            })
        }
    }
}

#[cfg(windows)]
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.handle);
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(path: *const u16, file: usize, flags: u32) -> usize;
    fn GetProcAddress(module: usize, name: *const u8) -> *const std::ffi::c_void;
    fn FreeLibrary(module: usize) -> i32;
}
