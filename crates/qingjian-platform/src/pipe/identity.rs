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
/// 管道的所有者必须是 `me`，且对面进程的映像必须是 `expected_exe`（DLL 同目录的 `qingjian-server.exe`）。
/// AppContainer 宿主（商店应用、任务栏搜索）打不开桌面进程、查不了映像，只验所有者。
pub fn server_is_trusted(pipe: HANDLE, expected_exe: &Path, me: &UserSid) -> bool {
    if pipe_owner(pipe).as_ref() != Some(me) {
        return false;
    }
    match server_image(pipe) {
        Some(image) => same_file(&image, expected_exe),
        None => current_is_app_container(),
    }
}

/// 管道对象的所有者。
fn pipe_owner(pipe: HANDLE) -> Option<UserSid> {
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

/// 同一个文件：能解析就比解析后的路径，否则不分大小写比字面。
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase(),
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
