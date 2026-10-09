//! 连不上 Server 时自己拉起它。
//!
//! Server 只在登录时由「启动」文件夹的快捷方式拉起（Explorer 走 `ShellExecute`，见 `qingjian.iss`），
//! 中途挂了（崩溃 / 被杀 / 装完没重启）以前 DLL 只能静默吞键到下次登录。这里在连接失败后起一次与
//! DLL 同目录的 `qingjian-server.exe`：用 `ShellExecuteW` 而不是 `CreateProcess`——`uiAccess=true`
//! 的 exe 用 `CreateProcess` 拉不起来（报 740），ShellExecute 等同双击，两种构建都行。
//!
//! 两道闸门防重复启动：进程内的冷却时间（同一应用连敲只试一次），与跨进程的命名互斥体
//! （多个应用同时发现 Server 不在，只起一个）。找不到 exe / 起失败 / 宿主不对记日志返回
//! [`LaunchOutcome::Refused`]，调用方不再吞键、字母直接放行（见 `key_sink`）。
//!
//! 另一道门是宿主的完整性级别：输入法会被加载进登录界面（LogonUI）、UAC 的 consent.exe 这类
//! 以 SYSTEM 跑在安全桌面上的进程，也会进 AppContainer 的商店应用（Low）。在这些宿主里
//! `ShellExecute` 要么起出一个 SYSTEM 权限、挂在安全桌面上的 Server，要么干脆起不来——
//! 只在普通桌面应用（Medium）里拉，其余照旧退避重连。
//!
//! 安装 / 卸载程序运行期间也不拉：它刚结束 Server 要替换 exe，这时拉起会占住文件（错误代码 5），
//! 安装程序持有命名互斥体 [`INSTALLER_MUTEX`]（见 `qingjian.iss`），装完它自己起新 Server。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE,
};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, OpenMutexW, OpenProcessToken, SYNCHRONIZATION_SYNCHRONIZE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

use crate::com::log::log;
use crate::com::module_path;

/// 同一进程内两次尝试拉起 Server 的最短间隔：连接失败每个键都会走到这里，别把应用砸出一串 Server。
const LAUNCH_COOLDOWN: Duration = Duration::from_secs(5);

/// 跨进程互斥体：多个应用的 DLL 同时发现 Server 不在时只起一个（第二个起来的抢不到管道会自己退出）。
const LAUNCH_MUTEX: windows::core::PCWSTR = w!("Local\\QingjianServerLaunch");

/// 安装 / 卸载程序运行期间持有的互斥体，名字与 `qingjian.iss` 的 `HoldInstallerMutex` 一致。
const INSTALLER_MUTEX: windows::core::PCWSTR = w!("Global\\QingjianInstaller");

/// 普通桌面应用的完整性级别 RID（UAC 未提升的用户进程）。低一档是 AppContainer / 浏览器沙箱，
/// 高一档是管理员提升、SYSTEM 与安全桌面上的进程——那些里都不拉 Server。
const MEDIUM_INTEGRITY_RID: u32 = 0x2000;

/// 上次尝试拉起的时间；本进程内所有文本服务实例共用。
static LAST_LAUNCH: Mutex<Option<Instant>> = Mutex::new(None);

/// 连不上 Server 时尝试拉起它的结果，调用方据此决定断连期间还吞不吞拼音键。
pub(super) enum LaunchOutcome {
    /// 拉起请求刚发出：Server 一两秒内就该监听，宽限期内继续吞「可能是拼音」的键。
    Requested,
    /// 别处已在拉（本进程冷却未过 / 另一进程握着互斥体）：Server 也在路上，与 [`Self::Requested`] 同待遇，
    /// 只是宽限不重新计时（冷却每 5 秒就会回到这个分支，重置宽限会让「超时放行」永远轮不到）。
    InFlight,
    /// 这个宿主里起不出 Server（完整性级别不对 / 安装程序占用 / 找不到 exe）：短期不会有 Server，
    /// 继续吞键就是死键盘，调用方应直接放行字母。
    Refused,
}

/// 连不上 Server 时拉起它。
pub(super) fn launch_server() -> LaunchOutcome {
    if !cooldown_passed() {
        return LaunchOutcome::InFlight;
    }
    if !host_is_plain_desktop_app() {
        log("宿主进程不是普通桌面应用（完整性级别非 Medium），不拉起 Server");
        return LaunchOutcome::Refused;
    }
    if installer_running() {
        log("安装程序正在运行，不拉起 Server");
        return LaunchOutcome::Refused;
    }
    let Some(exe) = server_exe() else {
        log("找不到与 DLL 同目录的 qingjian-server.exe，不拉起");
        return LaunchOutcome::Refused;
    };
    let Some(_mutex) = launch_mutex() else {
        // 别的进程正在起：不重复起
        return LaunchOutcome::InFlight;
    };
    let file = HSTRING::from(exe.as_os_str());
    let workdir = exe
        .parent()
        .map(|dir| HSTRING::from(dir.as_os_str()))
        .unwrap_or_default();
    // 起完就返回：Server 一百多毫秒就监听管道，阻塞在应用 UI 线程上会被 TSF 看门狗切走输入法。
    let code = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR(workdir.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW 返回值 > 32 才算成功。
    if (code.0 as isize) > 32 {
        log("已请求启动 qingjian-server");
        LaunchOutcome::Requested
    } else {
        log(&format!(
            "启动 qingjian-server 失败，返回值 {}",
            code.0 as isize
        ));
        LaunchOutcome::Refused
    }
}

/// 进程内冷却：距上次尝试不到 [`LAUNCH_COOLDOWN`] 就不试。时间在尝试**之前**记，失败也冷却。
fn cooldown_passed() -> bool {
    let Ok(mut last) = LAST_LAUNCH.lock() else {
        return false;
    };
    if last.is_some_and(|at| at.elapsed() < LAUNCH_COOLDOWN) {
        return false;
    }
    *last = Some(Instant::now());
    true
}

/// 宿主进程是不是普通桌面应用（完整性级别为 Medium）。
///
/// 读当前进程令牌的 `TokenIntegrityLevel`，SID 最后一个子授权值就是级别 RID。
/// 读不到一律当成「不是」：宁可不拉（退回退避重连，下次登录时由启动文件夹的快捷方式起），
/// 也不能在安全桌面上起出一个 SYSTEM 权限的 Server。
fn host_is_plain_desktop_app() -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut needed = 0u32;
    // 两段式：第一次只问要多少字节（总是报缓冲区不够，属正常）。
    unsafe {
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
    }
    let mut buffer = vec![0u8; needed as usize];
    let queried = needed > 0
        && unsafe {
            GetTokenInformation(
                token,
                TokenIntegrityLevel,
                Some(buffer.as_mut_ptr().cast()),
                needed,
                &mut needed,
            )
        }
        .is_ok();
    let _ = unsafe { CloseHandle(token) };
    if !queried {
        return false;
    }
    let label = unsafe {
        buffer
            .as_ptr()
            .cast::<TOKEN_MANDATORY_LABEL>()
            .read_unaligned()
    };
    let sid = label.Label.Sid;
    let count_ptr = unsafe { GetSidSubAuthorityCount(sid) };
    if count_ptr.is_null() {
        return false;
    }
    let count = unsafe { *count_ptr };
    if count == 0 {
        return false;
    }
    let rid_ptr = unsafe { GetSidSubAuthority(sid, u32::from(count) - 1) };
    !rid_ptr.is_null() && unsafe { *rid_ptr } == MEDIUM_INTEGRITY_RID
}

/// 安装程序的互斥体在不在。它由提升的安装程序建，普通应用打开会被拒（`ERROR_ACCESS_DENIED`），拒也说明存在。
fn installer_running() -> bool {
    match unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, INSTALLER_MUTEX) } {
        Ok(handle) => {
            let _ = unsafe { CloseHandle(handle) };
            true
        }
        Err(error) => error.code() == ERROR_ACCESS_DENIED.to_hresult(),
    }
}

/// 跨进程互斥体；已被别的进程持有（对方正在起 Server）返回 `None`。
fn launch_mutex() -> Option<LaunchMutex> {
    let handle = unsafe { CreateMutexW(None, false, LAUNCH_MUTEX) }.ok()?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let _ = unsafe { CloseHandle(handle) };
        return None;
    }
    Some(LaunchMutex(handle))
}

/// 持有互斥体的句柄，析构时关闭。
struct LaunchMutex(HANDLE);

impl Drop for LaunchMutex {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn server_exe() -> Option<PathBuf> {
    let module = module_path().ok()?;
    Some(server_exe_path(Path::new(&module.to_string())))
}

/// 与 DLL 同目录的 `qingjian-server.exe`（安装器把两者装在同一目录）。
fn server_exe_path(module: &Path) -> PathBuf {
    module.with_file_name("qingjian-server.exe")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::server_exe_path;

    #[test]
    fn server_exe_sits_next_to_the_dll() {
        assert_eq!(
            server_exe_path(Path::new(
                r"D:\Program Files\Qingjian\qingjian_tsf-0.1.0-alpha.15-dev.dll"
            )),
            Path::new(r"D:\Program Files\Qingjian\qingjian-server.exe")
        );
    }
}
