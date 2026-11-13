//! 令牌里的用户 SID，拷成自己持有的字节，便于比较和格式化。

use std::fmt;
use std::io;

use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetLengthSid, GetTokenInformation, IsValidSid, PSID, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PWSTR;

/// 一个用户 SID。按字节相等即同一个 SID（与 `EqualSid` 等价）。
#[derive(Clone, PartialEq, Eq)]
pub struct UserSid(Vec<u8>);

impl UserSid {
    /// 当前进程的用户。AppContainer 与提升过的进程里也是登录用户本人的 SID。
    pub fn current() -> io::Result<Self> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(io::Error::other)?;
        let sid = Self::of_token(token);
        let _ = unsafe { CloseHandle(token) };
        sid
    }

    /// 令牌（要有 `TOKEN_QUERY`）的用户。
    pub fn of_token(token: HANDLE) -> io::Result<Self> {
        let mut needed = 0u32;
        // 第一次只问要多少字节，总是报缓冲区不够。
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut needed) };
        // 按 u64 分配，保证 TOKEN_USER 里的指针对齐。
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                needed,
                &mut needed,
            )
        }
        .map_err(io::Error::other)?;
        let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        Self::copy(user.User.Sid).ok_or_else(|| io::Error::other("token user SID is invalid"))
    }

    /// 拷一份系统给的 SID；无效时为 `None`。
    pub fn copy(sid: PSID) -> Option<Self> {
        if sid.is_invalid() || !unsafe { IsValidSid(sid) }.as_bool() {
            return None;
        }
        let len = unsafe { GetLengthSid(sid) } as usize;
        let bytes = unsafe { std::slice::from_raw_parts(sid.0.cast::<u8>(), len) };
        Some(Self(bytes.to_vec()))
    }

    /// 字符串形式（`S-1-5-21-…`），写进管道名与 SDDL。
    pub fn to_sddl(&self) -> io::Result<String> {
        let mut text = PWSTR::null();
        unsafe { ConvertSidToStringSidW(PSID(self.0.as_ptr().cast_mut().cast()), &mut text) }
            .map_err(io::Error::other)?;
        let converted = unsafe { text.to_string() };
        let _ = unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
        converted.map_err(io::Error::other)
    }
}

impl fmt::Debug for UserSid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_sddl() {
            Ok(text) => f.write_str(&text),
            Err(_) => write!(f, "UserSid({} bytes)", self.0.len()),
        }
    }
}
