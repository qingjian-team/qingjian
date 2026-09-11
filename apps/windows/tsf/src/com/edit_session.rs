//! 编辑会话：TSF 不允许直接改文档，要经 [`ITfContext::RequestEditSession`] 申请，在回调里拿着 edit cookie 写。
//!
//! 必须用异步读写会话（`TF_ES_READWRITE`，不带 `TF_ES_SYNC`）：沉浸式应用（Win11 新记事本等）的文本存区隔着
//! 进程边界，不支持同步读写，同步请求会让 `textinputframework.dll` 访问违例把宿主整个搞崩。异步下回调可能在
//! `OnKeyDown` 返回之后才跑，会话对象连同它存的东西由框架持有到回调结束。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READWRITE,
};
use windows::core::{Error, Result, implement};

use super::composition::{Shared, apply};
use super::log::log;

/// 一次性的编辑会话：在回调里把本次按键的组句更新写进 `context`。
#[implement(ITfEditSession)]
pub(crate) struct UpdateSession {
    /// 目标文档上下文。
    context: ITfContext,

    /// 组句状态。
    shared: Rc<Shared>,

    /// 本次要落定上屏的文本。
    commit: Option<String>,

    /// 本次组句拼音行；空串表示收起组句。
    preedit: String,
}

impl ITfEditSession_Impl for UpdateSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        // 从框架的 C++ 调进来：panic 不能越过 FFI，兜成 E_FAIL。
        let result = catch_unwind(AssertUnwindSafe(|| {
            apply(
                &self.shared,
                &self.context,
                ec,
                self.commit.as_deref(),
                &self.preedit,
            )
        }));
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                log(&format!("组句更新失败: {error}"));
                Err(error)
            }
            Err(_) => {
                log("组句更新回调 panic（已兜住）");
                Err(Error::from(E_FAIL))
            }
        }
    }
}

/// 请求一个异步读写编辑会话。返回 `Ok` 只说明会话已被受理，实际写入结果在回调里记日志。
pub(crate) fn request_update(
    context: &ITfContext,
    client_id: u32,
    shared: Rc<Shared>,
    commit: Option<String>,
    preedit: String,
) -> Result<()> {
    let session: ITfEditSession = UpdateSession {
        context: context.clone(),
        shared,
        commit,
        preedit,
    }
    .into();
    // SAFETY: context 有效；异步会话由框架 AddRef 持有直到回调跑完。
    let hr = unsafe { context.RequestEditSession(client_id, &session, TF_ES_READWRITE)? };
    hr.ok()
}
