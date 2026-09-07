//! InputMethodKit 这一侧：输入控制器、文本客户端封装、修饰键与 Secure Input 查询。
//!
//! 只做两件事：把系统输入事件翻译成 Engine 的调用，把 Engine 的结果交给候选窗口。
//! 排序、词库、翻译逻辑一概不许出现在这里。

mod client;
mod controller;
pub mod modifiers;
pub mod secure_input;

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub use client::TextClient;
pub use controller::QingjianInputController;

/// 在 ObjC 运行时回调的边界拦住 panic。`define_class!` 生成的方法是系统直接调的，panic 穿过去整个进程就没了，
/// 用户正在打的字也跟着没了。拦住后记一条错误日志（位置与 backtrace 由 `main.rs` 装的 panic hook 记），
/// 返回 `None` 让调用方善后（见 [`recover_from_panic`]）。
pub fn catch_panic<R>(what: &'static str, f: impl FnOnce() -> R) -> Option<R> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Some(value),
        Err(payload) => {
            tracing::error!(
                what,
                message = panic_message(&*payload),
                "回调里 panic，已拦下"
            );
            None
        }
    }
}

/// panic 之后的善后：把缓冲区里的字母原样交给应用（用户敲过的键不能消失），再清掉引擎状态、收起窗口。
/// 善后本身再 panic 就只能放弃，至少进程还活着。
pub fn recover_from_panic(client: Option<TextClient<'_>>) {
    let recovered = catch_unwind(AssertUnwindSafe(|| {
        let pending = crate::host::with(|h| {
            let text = h.engine.composition().text().to_owned();
            h.engine.clear();
            h.cancel_prediction();
            h.window.hide();
            text
        });
        if let (Some(client), Some(text)) = (client, pending)
            && !text.is_empty()
        {
            tracing::warn!(len = text.chars().count(), "panic 善后：缓冲区字母原样上屏");
            client.insert_text(&text);
        }
    }));
    if recovered.is_err() {
        tracing::error!("panic 善后又 panic，放弃本次会话状态");
    }
}

/// panic payload 里的文字：`panic!("...")` 是 `&str` 或 `String`，别的类型给个占位。
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<非文字 payload>")
}
