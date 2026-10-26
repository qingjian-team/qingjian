//! 输入 profile 通知：本线程切成别的输入法（微软拼音等）或别的键盘布局（英语的 ENG）时告诉 Server 收状态条。
//! 用 `ITfInputProcessorProfileActivationSink`：`ITfActiveLanguageProfileNotifySink` 只报文字服务，
//! 跨语言切到纯键盘布局时不来，状态条就一直留着。
//! 应用退出时只有 `Deactivate`、没有别的 profile 被激活，所以状态条不跟会话开关走。
//! 通知在青简停用**之后**到来，那时会话连接已关，用一条临时连接发。
//! sink 挂在线程管理器上，激活一次、之后不撤（线程管理器销毁时自然释放），重复激活不再挂。

use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    ITfInputProcessorProfileActivationSink, ITfInputProcessorProfileActivationSink_Impl, ITfSource,
    ITfThreadMgr, TF_IPSINK_FLAG_ACTIVE, TF_PROFILETYPE_INPUTPROCESSOR,
};
use windows::core::{GUID, Interface, Result, implement};

use qingjian_platform::protocol::SessionId;

use super::log::log;
use crate::client::EngineClient;
use crate::client::pipe::connect_default;

#[implement(ITfInputProcessorProfileActivationSink)]
struct ProfileSink {
    /// 通知里带的会话标识（Server 只当信号，不查会话）。
    session: SessionId,
}

impl ProfileSink {
    fn new(session: SessionId) -> Self {
        super::lock_module();
        Self { session }
    }
}

impl Drop for ProfileSink {
    fn drop(&mut self) {
        super::unlock_module();
    }
}

impl ITfInputProcessorProfileActivationSink_Impl for ProfileSink_Impl {
    fn OnActivated(
        &self,
        dwprofiletype: u32,
        _langid: u16,
        clsid: *const GUID,
        _catid: *const GUID,
        _guidprofile: *const GUID,
        _hkl: HKL,
        dwflags: u32,
    ) -> Result<()> {
        // 青简自己被停用（切走、应用退出）也会来一次，不带 ACTIVE，不管它；只认「别的 profile 被激活」
        let ours = dwprofiletype == TF_PROFILETYPE_INPUTPROCESSOR
            && unsafe { *clsid } == super::CLSID_QINGJIAN;
        if dwflags & TF_IPSINK_FLAG_ACTIVE != 0 && !ours {
            let sent = connect_default()
                .map_err(|e| e.to_string())
                .and_then(|stream| {
                    EngineClient::notify_ime_switched(stream, self.session)
                        .map_err(|e| e.to_string())
                });
            match sent {
                Ok(()) => log("切到别的输入法或键盘布局，已通知 Server 收状态条"),
                Err(error) => log(&format!(
                    "切到别的输入法或键盘布局，通知 Server 失败: {error}"
                )),
            }
        }
        Ok(())
    }
}

/// 挂上通知，返回 cookie（只用来判断挂过没有）。
pub(super) fn advise(thread_mgr: &ITfThreadMgr, session: SessionId) -> Result<u32> {
    let source: ITfSource = thread_mgr.cast()?;
    let sink: ITfInputProcessorProfileActivationSink = ProfileSink::new(session).into();
    unsafe { source.AdviseSink(&ITfInputProcessorProfileActivationSink::IID, &sink) }
}
