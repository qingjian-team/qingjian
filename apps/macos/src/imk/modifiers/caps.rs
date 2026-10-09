//! IOKit 修改系统 Caps Lock 状态，不注入键盘事件；写入后回读确认。

use std::ffi::{c_char, c_int, c_void};

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(port: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(service: u32, task: u32, kind: u32, connect: *mut u32) -> c_int;
    fn IOObjectRelease(object: u32) -> c_int;
    fn IOServiceClose(connect: u32) -> c_int;
    fn IOHIDGetModifierLockState(connect: u32, selector: c_int, state: *mut bool) -> c_int;
    fn IOHIDSetModifierLockState(connect: u32, selector: c_int, state: bool) -> c_int;
}

unsafe extern "C" {
    static mach_task_self_: u32;
}

pub(super) fn set(on: bool) -> Result<(), String> {
    // SAFETY: matching 字典归 GetMatchingService 消耗；服务和连接在全部返回路径释放。
    unsafe {
        let matching = IOServiceMatching(c"IOHIDSystem".as_ptr());
        if matching.is_null() {
            return Err("找不到 IOHIDSystem 匹配字典".into());
        }
        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return Err("找不到 IOHIDSystem 服务".into());
        }
        let mut connect = 0;
        let result = IOServiceOpen(service, mach_task_self_, 1, &mut connect);
        IOObjectRelease(service);
        if result != 0 {
            return Err(format!("打开 Caps 控制连接失败：{result:#x}"));
        }
        let mut actual = false;
        let get = IOHIDGetModifierLockState(connect, 1, &mut actual);
        let write = if get == 0 && actual != on {
            IOHIDSetModifierLockState(connect, 1, on)
        } else {
            get
        };
        let confirm = if write == 0 {
            IOHIDGetModifierLockState(connect, 1, &mut actual)
        } else {
            write
        };
        IOServiceClose(connect);
        if confirm != 0 {
            return Err(format!("同步 Caps 状态失败：{confirm:#x}"));
        }
        if actual != on {
            return Err("Caps 状态回读与目标不一致".into());
        }
        Ok(())
    }
}
