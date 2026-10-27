//! 候选列表元素的起止：Server 每回一帧就交给 [`CandidateList::show`]，组句结束时 [`CandidateList::end`]。

use std::cell::RefCell;

use windows::Win32::Foundation::TRUE;
use windows::Win32::UI::TextServices::{ITfCandidateListUIElement, ITfContext, ITfUIElementMgr};
use windows::core::ComObject;

use qingjian_platform::protocol::Frame;

use super::CandidateElement;
use crate::com::log::log;

/// 按 TSF 约定，组句开始时 `BeginUIElement`，应用回 `bShow = FALSE` 就是它自己画候选：
/// 这时不向 Server 报光标位置，Server 没有位置就不画候选窗。
#[derive(Default)]
pub(crate) struct CandidateList {
    /// 激活时从线程管理器取，停用时放掉。
    manager: RefCell<Option<ITfUIElementMgr>>,

    /// 进行中的元素与它的 id。
    current: RefCell<Option<(ComObject<CandidateElement>, u32)>>,
}

impl CandidateList {
    pub(crate) fn set_manager(&self, manager: Option<ITfUIElementMgr>) {
        *self.manager.borrow_mut() = manager;
    }

    /// 应用说它自己画候选。
    pub(crate) fn app_draws(&self) -> bool {
        self.current
            .borrow()
            .as_ref()
            .is_some_and(|(element, _)| !element.shown())
    }

    /// 交来 Server 回的一帧：空帧收掉元素，否则头一帧起元素、之后有变化才让应用重读。
    /// 调 TSF 时不挂着借用：应用的回调里可能再进来（读候选、终止组句）。
    pub(crate) fn show(&self, frame: &Frame, context: Option<&ITfContext>) {
        if frame.is_empty() {
            self.end();
            return;
        }
        let Some(manager) = self.manager.borrow().clone() else {
            return;
        };
        let current = self.current.borrow().clone();
        let id = match current {
            Some((element, id)) => {
                if !element.set_frame(frame) {
                    return;
                }
                id
            }
            None => {
                let document = context.and_then(|context| unsafe { context.GetDocumentMgr() }.ok());
                let element = ComObject::new(CandidateElement::new(document));
                element.set_frame(frame);
                let (mut show, mut id) = (TRUE, 0);
                let interface = element.to_interface::<ITfCandidateListUIElement>();
                if let Err(error) =
                    unsafe { manager.BeginUIElement(&interface, &mut show, &mut id) }
                {
                    log(&format!(
                        "BeginUIElement 失败，候选照常由 Server 画: {error}"
                    ));
                    return;
                }
                element.set_shown(show.as_bool());
                if !show.as_bool() {
                    log("应用自己画候选（UI-less），Server 不画候选窗");
                }
                *self.current.borrow_mut() = Some((element, id));
                id
            }
        };
        let _ = unsafe { manager.UpdateUIElement(id) };
    }

    /// 收掉进行中的元素（组句结束 / 断线 / 停用）。
    pub(crate) fn end(&self) {
        let Some((_, id)) = self.current.borrow_mut().take() else {
            return;
        };
        if let Some(manager) = self.manager.borrow().clone() {
            let _ = unsafe { manager.EndUIElement(id) };
        }
    }
}
