//! 编辑会话：TSF 不允许直接改文档，要经 `RequestEditSession` 申请，在回调里拿着 edit cookie 写。
//!
//! 必须用异步会话（不带 `TF_ES_SYNC`）：沉浸式应用（Win11 新记事本等）的文本存区隔着进程边界，
//! 同步读写会让 `textinputframework.dll` 访问违例把宿主整个搞崩。回调可能在 `OnKeyDown` 返回之后才跑。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_CONTEXT_EDIT_CONTEXT_FLAGS, TF_ES_READWRITE,
};
use windows::core::{Error, Result, implement};

use crate::com::composition::{Shared, apply};
use crate::com::log::log;
use crate::com::service::SharedClient;

/// 一次性的读写会话：把本次按键的组句更新写进 `context`。
#[implement(ITfEditSession)]
pub(crate) struct UpdateSession {
    /// 目标文档上下文。
    context: ITfContext,

    /// 引擎层：量到光标矩形后报给 Server 摆候选窗口。
    engine: SharedClient,

    /// 组句状态。
    shared: Rc<Shared>,

    /// 本次要落定上屏的文本。
    commit: Option<String>,

    /// 本次组句拼音行；空串表示收起组句。
    preedit: String,
}

impl ITfEditSession_Impl for UpdateSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        // 从框架的 C++ 调进来：panic 不能越过 FFI。
        let result = catch_unwind(AssertUnwindSafe(|| {
            apply(
                &self.shared,
                &self.engine,
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

/// 请求一个异步读写会话。`Ok` 只说明已受理，写入结果在回调里记日志。
pub(crate) fn request_update(
    context: &ITfContext,
    client_id: u32,
    engine: SharedClient,
    shared: Rc<Shared>,
    commit: Option<String>,
    preedit: String,
) -> Result<()> {
    let session = UpdateSession {
        context: context.clone(),
        engine,
        shared,
        commit,
        preedit,
    };
    request(context, client_id, session.into(), TF_ES_READWRITE)
}

/// 异步提交一个编辑会话；会话对象由框架持有到回调跑完。
pub(super) fn request(
    context: &ITfContext,
    client_id: u32,
    session: ITfEditSession,
    flags: TF_CONTEXT_EDIT_CONTEXT_FLAGS,
) -> Result<()> {
    unsafe { context.RequestEditSession(client_id, &session, flags)? }.ok()
}
