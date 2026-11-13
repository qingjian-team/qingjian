//! Windows 上 Server 与 DLL 之间那条命名管道：名字、安全描述符、两端互相认人（#699）。
//!
//! 管道名带用户 SID 与登录会话号，不同用户、同一用户的不同登录会话各连各的 Server；
//! 安全描述符只给本用户全权，AppContainer 只能读写数据、建不了实例（抢不走别人的连接）。
//! 连上之后 Server 用 [`client_is_same_user`] 认客户端，DLL 用 [`server_is_trusted`] 认 Server。

mod identity;
mod user_sid;

use std::io;

use windows::Win32::Storage::FileSystem::{
    FILE_GENERIC_READ, FILE_WRITE_DATA, SECURITY_IDENTIFICATION,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::GetCurrentProcessId;

pub use identity::{client_is_same_user, server_is_trusted};
pub use user_sid::UserSid;

/// 老 DLL 连的固定管道名。升级后没重启的应用里还是老 DLL，Server 过渡期在这上面也服务（见 `docs/plan/todo.md`）。
pub const LEGACY_PIPE_NAME: &str = r"\\.\pipe\qingjian";

/// 老管道的 SDDL：老 DLL 按 `GENERIC_READ | GENERIC_WRITE` 打开，只能照旧放行 Everyone 与 AppContainer 读写。
pub const LEGACY_PIPE_SDDL: &str =
    "D:(A;;FRFW;;;WD)(A;;FRFW;;;AC)(A;;FRFW;;;S-1-15-2-2)S:(ML;;NW;;;LW)";

/// DLL 打开管道要的权限：读 + 写数据。不要 `GENERIC_WRITE`，那里面的 `FILE_APPEND_DATA`
/// 对管道就是「建实例」，安全描述符不给 AppContainer，要了会被拒。
pub const CLIENT_ACCESS: u32 = FILE_GENERIC_READ.0 | FILE_WRITE_DATA.0;

/// DLL 打开管道时给 Server 的模拟级别：识别级，Server 能查是谁、不能拿这个身份做事。
pub const CLIENT_QOS: u32 = SECURITY_IDENTIFICATION.0;

/// 当前用户、当前登录会话的管道名：`\\.\pipe\qingjian-<SID>-<会话号>`。Server 与 DLL 各按自己的令牌算，
/// 同一用户同一会话必然同名。
pub fn pipe_name(user: &UserSid) -> io::Result<String> {
    let mut session = 0u32;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) }
        .map_err(io::Error::other)?;
    Ok(format!(r"\\.\pipe\qingjian-{}-{session}", user.to_sddl()?))
}

/// 管道的 SDDL：本用户全权（Server 要建后续实例）；AppContainer 只给 [`CLIENT_ACCESS`]。
/// AppContainer 进程过访问检查要用户与 AppContainer 两条都满足，所以别的用户的商店应用也连不上。
/// 完整性标 Low：AppContainer 是 Low，标成 Medium 它就写不进来。
pub fn pipe_sddl(user: &UserSid) -> io::Result<String> {
    let user = user.to_sddl()?;
    Ok(format!(
        "D:(A;;GA;;;{user})(A;;{CLIENT_ACCESS:#x};;;AC)(A;;{CLIENT_ACCESS:#x};;;S-1-15-2-2)S:(ML;;NW;;;LW)"
    ))
}

#[cfg(test)]
mod tests;
