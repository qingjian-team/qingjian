//! `cfg(windows)`：连 Server 的命名管道客户端。
//!
//! Server（`qingjian-windows`）用 `CreateNamedPipeW` 在 `\\.\pipe\qingjian` 上建管道；这里用
//! `CreateFileW` 打开同名管道，得到一条双工流交给 [`EngineClient`](super::EngineClient)。字节模式，
//! 帧由协议层 codec 切。Server 一次只服务一个客户端，管道忙时 [`connect`] 等一个可用实例再试。

use std::io::{self, Read, Write};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING, ReadFile, WriteFile,
};
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows::core::{Error as WinError, HRESULT, HSTRING};

use qingjian_platform::protocol::DEFAULT_PIPE_NAME;

/// 管道忙时等一个可用实例的超时（毫秒）与最多重试次数。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

/// 连好的命名管道流，析构关句柄。impl Read + Write，直接交给 [`EngineClient`](super::EngineClient)。
pub struct PipeStream(HANDLE);

impl Drop for PipeStream {
    fn drop(&mut self) {
        // SAFETY: 句柄由 CreateFileW 得来、本对象独占，只在这里关一次。
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// 用缺省管道名 [`DEFAULT_PIPE_NAME`] 连 Server。
pub fn connect_default() -> io::Result<PipeStream> {
    connect(DEFAULT_PIPE_NAME)
}

/// 连 Server 的命名管道。管道忙（Server 正服务上一个客户端）时最多等 [`BUSY_RETRIES`] 轮、
/// 每轮 [`BUSY_WAIT_MS`] 毫秒；一直忙或出别的错就返回 [`io::Error`]。
pub fn connect(name: &str) -> io::Result<PipeStream> {
    let wide = HSTRING::from(name);
    let mut attempts = 0;
    loop {
        // SAFETY: wide 是有效的以 0 结尾宽字符串；打开既有管道，安全属性 / 模板句柄传 None。
        let handle = unsafe {
            CreateFileW(
                &wide,
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        match handle {
            Ok(handle) => return Ok(PipeStream(handle)),
            Err(error) if is_win32(&error, ERROR_PIPE_BUSY.0) && attempts < BUSY_RETRIES => {
                attempts += 1;
                // SAFETY: wide 有效；等一个可用实例（超时返回 false，下一轮 CreateFileW 再报忙）。
                let _ = unsafe { WaitNamedPipeW(&wide, BUSY_WAIT_MS) };
            }
            Err(error) => return Err(io::Error::other(error)),
        }
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0u32;
        // SAFETY: 句柄有效，buf 可写，read 是有效指针，无 overlapped。
        let result = unsafe { ReadFile(self.0, Some(buf), Some(&mut read), None) };
        match result {
            Ok(()) => Ok(read as usize),
            // 对端关闭：当作 EOF（codec 据此在帧边界干净结束）。
            Err(error) if is_win32(&error, ERROR_BROKEN_PIPE.0) => Ok(0),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut written = 0u32;
        // SAFETY: 句柄有效，buf 可读，written 是有效指针，无 overlapped。
        unsafe { WriteFile(self.0, Some(buf), Some(&mut written), None) }
            .map_err(io::Error::other)?;
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        // WriteFile 已把字节交给内核；管道无需 FlushFileBuffers（那会阻塞到对端读完）。
        Ok(())
    }
}

/// 这个 windows 错误是不是指定的 Win32 错误码。
fn is_win32(error: &WinError, code: u32) -> bool {
    error.code() == HRESULT::from_win32(code)
}
