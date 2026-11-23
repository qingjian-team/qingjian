//! 连上之后两端互相认人：Server 认客户端是不是同一个用户，DLL 认对面是不是我们的 Server。

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
use windows::Win32::Security::{
    GetTokenInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, RevertToSelf,
    TOKEN_QUERY, TokenIsAppContainer,
};
use windows::Win32::System::Pipes::{GetNamedPipeServerProcessId, ImpersonateNamedPipeClient};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, OpenProcess, OpenProcessToken, OpenThreadToken,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

use super::UserSid;

/// Server 侧：管道对面的客户端是不是 `me` 这个用户。
///
/// 在这条连接上读到过数据之后再调：文档要求先读过数据，`ImpersonateNamedPipeClient` 才认得客户端。
/// 新 DLL 只给识别级（`SECURITY_IDENTIFICATION`）模拟，够查令牌、不够拿它做事。
pub fn client_is_same_user(pipe: HANDLE, me: &UserSid) -> bool {
    if unsafe { ImpersonateNamedPipeClient(pipe) }.is_err() {
        return false;
    }
    let mut token = HANDLE::default();
    // 识别级模拟下要以自己的身份开线程令牌，否则报 ERROR_BAD_IMPERSONATION_LEVEL。
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token) };
    if unsafe { RevertToSelf() }.is_err() {
        // 退不回自己的身份就只能整个进程退出：接着跑等于用客户端的身份干活。
        std::process::abort();
    }
    if opened.is_err() {
        return false;
    }
    let user = UserSid::of_token(token);
    let _ = unsafe { CloseHandle(token) };
    user.is_ok_and(|user| user == *me)
}

/// DLL 侧：管道对面是不是我们的 Server。
///
/// 管道的所有者必须是 `me`，对面进程的映像必须是 `expected_exe`（DLL 同目录的 `qingjian-server.exe`），
/// 且这个 exe 要有受信任的 Authenticode 签名：安装目录可写时（开发安装、被改过的安装），光比路径
/// 挡不住别人把同目录的 exe 换掉、冒充 Server 收走击键。debug 构建与本机调试用
/// `QINGJIAN_ALLOW_UNSIGNED_SERVER` 放行。
/// AppContainer 宿主（商店应用、任务栏搜索）打不开桌面进程、查不了映像，只验所有者。
pub fn server_is_trusted(pipe: HANDLE, expected_exe: &Path, me: &UserSid) -> bool {
    if pipe_owner(pipe).as_ref() != Some(me) {
        return false;
    }
    match server_image(pipe) {
        Some(image) => same_file(&image, expected_exe) && server_exe_trusted(expected_exe),
        None => current_is_app_container(),
    }
}

/// 同目录的 `qingjian-server.exe` 是否过 Authenticode。debug 构建是开发用，不签；
/// 发布版（release）必须由受信任证书签名，否则不认。
fn server_exe_trusted(path: &Path) -> bool {
    if cfg!(debug_assertions) || std::env::var_os("QINGJIAN_ALLOW_UNSIGNED_SERVER").is_some() {
        return true;
    }
    verify_authenticode(path)
}

#[cfg(windows)]
fn verify_authenticode(path: &Path) -> bool {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::Security::WinTrust::{
        WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO, WinVerifyTrust,
    };
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(wide.as_ptr()),
        hFile: HANDLE::default(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut data: WINTRUST_DATA = unsafe { std::mem::zeroed() };
    data.cbStruct = size_of::<WINTRUST_DATA>() as u32;
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data.dwUnionChoice = WTD_CHOICE_FILE;
    data.dwStateAction = WTD_STATEACTION_VERIFY;
    // SAFETY: union 里只有 pFile 会被读，指向上面这个仍然活着的结构。
    data.Anonymous.pFile = &raw mut file;
    let hwnd = HWND::default();
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let status = unsafe { WinVerifyTrust(hwnd, &raw mut action, (&raw mut data).cast()) };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe { WinVerifyTrust(hwnd, &raw mut action, (&raw mut data).cast()) };
    status == 0
}

/// 管道对象的所有者。
pub(super) fn pipe_owner(pipe: HANDLE) -> Option<UserSid> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            pipe,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let sid = UserSid::copy(owner);
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    sid
}

/// 管道服务端进程的映像路径；进程打不开（AppContainer 里的常态）时为 `None`。
fn server_image(pipe: HANDLE) -> Option<PathBuf> {
    let mut pid = 0u32;
    unsafe { GetNamedPipeServerProcessId(pipe, &mut pid) }.ok()?;
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    let queried = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
    };
    let _ = unsafe { CloseHandle(process) };
    queried.ok()?;
    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..len as usize],
    )))
}

/// 同一个文件：两边都解析得出真实路径才比较；解析不出就不认。
/// 以前解析失败时按字面小写比较，`C:\a\qingjian-server.exe` 与 `C:\b\qingjian-server.exe` 会被当成同一个。
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// 当前进程跑在 AppContainer 里。读不到按不是：只有确认是 AppContainer 才放宽校验。
fn current_is_app_container() -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut value = 0u32;
    let mut len = 0u32;
    let queried = unsafe {
        GetTokenInformation(
            token,
            TokenIsAppContainer,
            Some((&raw mut value).cast()),
            size_of::<u32>() as u32,
            &mut len,
        )
    };
    let _ = unsafe { CloseHandle(token) };
    queried.is_ok() && value != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("qingjian-pipe-identity-tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn same_file_requires_both_paths_to_resolve() {
        let dir = temp_dir("same-file");
        let a = dir.join("qingjian-server.exe");
        let b = dir.join("qingjian-server.exe");
        std::fs::write(&a, b"").unwrap();
        assert!(same_file(&a, &b));

        let other = temp_dir("same-file-other").join("qingjian-server.exe");
        std::fs::write(&other, b"").unwrap();
        // 同名但不同路径：不能因为字面（小写）相同就认。
        assert!(!same_file(&a, &other));

        // 不存在的路径不认。
        assert!(!same_file(&a, &dir.join("missing.exe")));
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(other.parent().unwrap());
    }
}
