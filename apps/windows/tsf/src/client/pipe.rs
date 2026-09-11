//! `cfg(windows)`：用 `CreateFileW` 打开 Server 建的命名管道，得到一条双工流交给 [`EngineClient`](super::EngineClient)。

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

/// 管道实例都被占着时等一个可用实例的超时（毫秒）与重试次数。
/// 要短：这里在应用的 UI 线程上，等久了 TSF 看门狗会把输入法切走。
const BUSY_WAIT_MS: u32 = 300;
const BUSY_RETRIES: u32 = 1;

/// 连好的命名管道，析构关句柄。
pub struct PipeStream(HANDLE);

impl Drop for PipeStream {
    fn drop(&mut self) {
        // SAFETY: 句柄由 CreateFileW 得来、本对象独占，只在这里关一次。
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub fn connect_default() -> io::Result<PipeStream> {
    connect(DEFAULT_PIPE_NAME)
}

/// 连 Server 的命名管道。忙时最多等 [`BUSY_RETRIES`] 轮、每轮 [`BUSY_WAIT_MS`] 毫秒。
pub fn connect(name: &str) -> io::Result<PipeStream> {
    let wide = HSTRING::from(name);
    let mut attempts = 0;
    loop {
        // SAFETY: wide 以 0 结尾；打开既有管道，安全属性 / 模板句柄传 None。
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
                // SAFETY: wide 有效；超时返回 false，下一轮 CreateFileW 再报忙。
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
        match unsafe { ReadFile(self.0, Some(buf), Some(&mut read), None) } {
            Ok(()) => Ok(read as usize),
            // 对端关闭当作 EOF，codec 据此在帧边界干净结束。
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

    /// WriteFile 已把字节交给内核；管道上 FlushFileBuffers 会阻塞到对端读完，不做。
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn is_win32(error: &WinError, code: u32) -> bool {
    error.code() == HRESULT::from_win32(code)
}
