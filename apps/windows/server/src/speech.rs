//! 朗读英文候选：Windows 上用系统自带的 SAPI（`ISpVoice`）合成，不联网、不进安装包。
//!
//! 一个专用线程持有同一个声音实例，同步朗读（这个线程只为朗读活着，阻塞到读完为止）；
//! 异步 `Speak` 要靠调用线程的消息泵推进，而我们阻塞在收词上，第二条起就不出声了。
//! 没有可用语音时只记一条警告，输入法照常工作。

#[cfg(all(windows, not(test)))]
use std::sync::OnceLock;
#[cfg(all(windows, not(test)))]
use std::sync::mpsc::{self, Sender};

#[cfg(all(windows, not(test)))]
use windows::Win32::Media::Speech::{ISpVoice, SPF_IS_NOT_XML, SpVoice};
#[cfg(all(windows, not(test)))]
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
#[cfg(all(windows, not(test)))]
use windows::core::PCWSTR;

#[cfg(all(windows, not(test)))]
static SENDER: OnceLock<Sender<String>> = OnceLock::new();

/// 朗读一个词：立刻返回，合成在后台线程里排队。
pub fn speak(word: &str) {
    #[cfg(all(windows, not(test)))]
    speak_windows(word);

    // 其他平台与单元测试里不发声
    #[cfg(any(not(windows), test))]
    let _ = word;
}

#[cfg(all(windows, not(test)))]
fn speak_windows(word: &str) {
    let sender = SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("speak".to_owned())
            .spawn(move || worker(receiver))
            .expect("朗读线程");
        sender
    });
    // 线程已经退出（例如 SAPI 不可用）就不再排队
    if sender.send(word.to_owned()).is_err() {
        tracing::warn!("朗读线程已退出，本次不发声");
    }
}

#[cfg(all(windows, not(test)))]
fn worker(receiver: mpsc::Receiver<String>) {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED);
        tracing::info!(?initialized, "朗读线程启动");
        let voice: ISpVoice = match CoCreateInstance(&SpVoice, None, CLSCTX_ALL) {
            Ok(voice) => voice,
            Err(error) => {
                tracing::warn!(%error, "SAPI 语音不可用，朗读关闭");
                return;
            }
        };
        // 同步朗读，不解析 XML（Speak 默认把文本当 SSML）
        let flags = SPF_IS_NOT_XML.0 as u32;
        while let Ok(word) = receiver.recv() {
            let wide: Vec<u16> = word.encode_utf16().chain(std::iter::once(0)).collect();
            match voice.Speak(PCWSTR(wide.as_ptr()), flags, None) {
                Ok(()) => tracing::info!(word, "朗读完成"),
                Err(error) => tracing::warn!(%error, word, "朗读失败"),
            }
        }
    }
}
