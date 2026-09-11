//! 编辑会话：把一次按键算出的「上屏文本 + 组句拼音行」经 TSF 写进当前文档。
//!
//! TSF 不允许直接改文档，必须先经 [`ITfContext::RequestEditSession`] 申请一个编辑上下文（edit cookie），
//! 在回调 [`ITfEditSession_Impl::DoEditSession`] 里拿着这个 cookie 才能写。真正的写逻辑（起 / 改 / 收组句、
//! 落定上屏）在 [`super::composition::apply`]。
//!
//! **用异步读写会话**（`TF_ES_ASYNCDONTCARE | TF_ES_READWRITE`，其值就是 [`TF_ES_READWRITE`]）：
//! 现代沉浸式应用（Win11 新记事本等，文本存区在应用进程里、隔着进程边界）**不支持同步读写会话**，
//! 早先用 `TF_ES_SYNC` 会让 `textinputframework.dll` 在写入时访问违例、把宿主整个搞崩（事件日志坐实：
//! Notepad 故障模块 textinputframework.dll，0xc0000005→0xc000041d「用户回调中未处理异常」）。异步会话
//! 两类应用都稳，是生产输入法的标准做法；回调可能在 `OnKeyDown` 返回之后才跑，届时会话对象仍被框架持有，
//! 连带它存的上下文、共享组句状态、文本一起存活。
//!
//! 回调边界用 [`catch_unwind`](std::panic::catch_unwind) 兜住：Rust panic 一旦越过 FFI 边界钻进 C++ 调用栈
//! 就是未定义行为（多半又一次崩溃），所以回调里任何 panic 都在这里转成 `E_FAIL`，绝不外泄。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READWRITE,
};
use windows::core::{Error, Result, implement};

use super::composition::{Shared, apply};

/// 一次性的编辑会话：在 [`DoEditSession`](ITfEditSession_Impl::DoEditSession) 回调里把本次按键的组句
/// 更新写进 `context`。
#[implement(ITfEditSession)]
pub(crate) struct UpdateSession {
    /// 要写入的目标文档上下文。
    context: ITfContext,

    /// 跨按键存活的组句状态（活动组句句柄、组句标志），与 `TextService` 共享。
    shared: Rc<Shared>,

    /// 本次要立即落定上屏的文本；`None` 表示这次不上屏。
    commit: Option<String>,

    /// 本次组句拼音行（内联显示）；空串表示收起组句。
    preedit: String,
}

impl UpdateSession {
    fn run(&self, ec: u32) -> Result<()> {
        apply(
            &self.shared,
            &self.context,
            ec,
            self.commit.as_deref(),
            &self.preedit,
        )
    }
}

impl ITfEditSession_Impl for UpdateSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        // 回调在应用线程上、由框架 C++ 代码调进来：panic 绝不能外泄，兜成 E_FAIL。
        let result = catch_unwind(AssertUnwindSafe(|| self.run(ec)));
        match result {
            Ok(Ok(())) => {
                super::log::log("组句更新已写入（编辑会话）");
                Ok(())
            }
            Ok(Err(error)) => {
                super::log::log(&format!("组句更新失败: {error}"));
                Err(error)
            }
            Err(_) => {
                super::log::log("组句更新回调 panic（已兜住）");
                Err(Error::from(E_FAIL))
            }
        }
    }
}

/// 请求一个**异步**读写编辑会话，把本次按键的 `commit` / `preedit` 更新到 `context`。`client_id` 是 TSF 在
/// `Activate` 时分配给本 TIP 的 id。返回 `Ok` 只说明会话已被受理（异步下多为 `TF_S_ASYNC`）；实际写入
/// 结果在 [`UpdateSession_Impl::DoEditSession`] 里记日志。
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
    // SAFETY: context 有效；session 是本模块造的编辑会话对象，异步会话由框架 AddRef 持有直到回调跑完。
    let hr = unsafe { context.RequestEditSession(client_id, &session, TF_ES_READWRITE)? };
    hr.ok()
}
