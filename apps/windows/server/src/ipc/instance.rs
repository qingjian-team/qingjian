//! 单实例闸门：进程一开始就占一个命名互斥体，已有 Server 在跑就不装配、直接退出。
//!
//! 命名管道的 `FILE_FLAG_FIRST_PIPE_INSTANCE` 也能拦第二个 Server，但管道要等 Engine 装配完（一秒多）才建，
//! 这期间被拉起的第二个（登录时启动文件夹的快捷方式与 DLL 各拉一个、几个应用的 DLL 先后拉）会把词库与本地模型
//! 整套加载一遍再退出，白占一份内存与 CPU。

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;

/// 占着就说明本进程是唯一的 Server；进程结束时系统收回。
pub struct Instance(HANDLE);

impl Instance {
    /// 已有 Server 占着返回 `None`。互斥体建不出来（不该发生）时也放行，交给管道那道闸门。
    pub fn acquire() -> Option<Self> {
        let Ok(handle) = (unsafe { CreateMutexW(None, false, w!("Local\\QingjianServer")) }) else {
            return Some(Self(HANDLE::default()));
        };
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let _ = unsafe { CloseHandle(handle) };
            return None;
        }
        Some(Self(handle))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}
