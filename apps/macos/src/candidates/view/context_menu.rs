//! 右键（或 ⌃ 点按）候选：问 host 这一格能做哪几件事，在鼠标处弹自绘菜单（[`MenuPanel`]），点中的项连同那一格交回 host。

use std::rc::Rc;

use objc2::{DefinedClass, MainThreadMarker};
use objc2_app_kit::NSEvent;
use qingjian_render::HitTarget;

use super::CandidateView;
use crate::candidates::menu::MenuPanel;

/// 右键菜单的两头：视图里第几格（从 0 起）给哪些项（编号, 标题），以及点中了哪一项。
pub struct MenuHandlers {
    pub items: Box<dyn Fn(usize) -> Vec<(u32, &'static str)>>,
    pub pick: Rc<dyn Fn(usize, u32)>,
}

impl CandidateView {
    pub fn set_menu_handlers(&self, handlers: MenuHandlers) {
        *self.ivars().on_menu.borrow_mut() = Some(handlers);
    }

    /// 收起右键菜单（候选窗重画、收起，或在候选窗上点了别处）。
    pub fn close_menu(&self) {
        if let Some(menu) = &*self.ivars().menu.borrow() {
            menu.close();
        }
    }

    pub fn menu_open(&self) -> bool {
        self.ivars()
            .menu
            .borrow()
            .as_ref()
            .is_some_and(MenuPanel::is_open)
    }

    pub(super) fn right_clicked(&self, event: &NSEvent) {
        tracing::debug!(point = ?event.locationInWindow(), "候选窗口收到右键");
        self.close_menu();
        if self.ivars().frame.borrow().is_panel() {
            return;
        }
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let target = self
            .ivars()
            .bitmap
            .borrow()
            .as_ref()
            .and_then(|bitmap| bitmap.hit(point))
            .or_else(|| self.system_candidate_at(point).map(HitTarget::Candidate));
        let offset = match target {
            Some(
                HitTarget::Candidate(offset)
                | HitTarget::Translation {
                    candidate: offset, ..
                },
            ) => offset,
            Some(HitTarget::MenuItem(_)) | None => return,
        };
        tracing::debug!(offset, "候选右键命中");
        let (items, pick) = match &*self.ivars().on_menu.borrow() {
            Some(handlers) => ((handlers.items)(offset), handlers.pick.clone()),
            None => return,
        };
        if items.is_empty() {
            return;
        }
        let Some(screen) = self.window().and_then(|window| window.screen()) else {
            return;
        };
        tracing::debug!(offset, "候选右键菜单");
        let mtm = MainThreadMarker::from(self);
        let dark = self.is_dark();
        let mut menu = self.ivars().menu.borrow_mut();
        let menu = menu.get_or_insert_with(|| MenuPanel::new(mtm));
        menu.open(
            items,
            NSEvent::mouseLocation(),
            screen.visibleFrame(),
            dark,
            Rc::new(move |id| pick(offset, id)),
        );
    }
}
