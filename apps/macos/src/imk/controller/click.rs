//! 鼠标点候选窗口：点候选与按它的数字键一样上屏，点译词与按译词键一样上屏它的那条译词。
//!
//! 候选窗口不知道是哪个会话，所以在 `activateServer:` 时记下当前控制器，点击时用它的客户端上屏。

use std::cell::RefCell;

use objc2::Message;
use qingjian_render::HitTarget;

use super::*;

thread_local! {
    /// 最近激活的会话；`deactivateServer:` 时清掉。
    static ACTIVE: RefCell<Option<Retained<QingjianInputController>>> = const { RefCell::new(None) };
}

/// 候选窗口的点击回调（host 建窗口时注册）。
pub fn click(target: HitTarget) {
    let Some(controller) = ACTIVE.with(|active| active.borrow().clone()) else {
        tracing::debug!("没有激活的会话，点击不处理");
        return;
    };
    if catch_panic("click", || controller.click(target)).is_none() {
        recover_from_panic(None);
    }
}

impl QingjianInputController {
    pub(super) fn set_active(&self) {
        ACTIVE.with(|active| *active.borrow_mut() = Some(self.retain()));
    }

    pub(super) fn clear_active(&self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .as_deref()
                .is_some_and(|current| std::ptr::eq(current, self))
            {
                *active = None;
            }
        });
    }

    fn click(&self, target: HitTarget) {
        // SAFETY: IMKInputController 的 `client` 返回当前会话的文本客户端（实现 IMKTextInput 的代理），可能为 nil
        let client: Option<Retained<AnyObject>> = unsafe { msg_send![self, client] };
        let Some(client) = client else {
            return;
        };
        let client = TextClient::new(&client);
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return;
        }
        match target {
            HitTarget::Candidate(offset) => {
                if let Some(index) = host::with(|h| h.session.index_in_view(offset)).flatten() {
                    self.commit_index(index, client);
                }
            }
            HitTarget::Translation { candidate, sense } => {
                let index = host::with(|h| h.session.index_in_view(candidate)).flatten();
                self.commit_translation_at(index, sense, client);
            }
        }
    }
}
