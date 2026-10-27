//! 鼠标点候选窗口：点候选与按它的数字键一样上屏，点译词与按译词键一样上屏那一条。
//! Server 不能直接写文档：上屏的文本挂到焦点会话上，DLL 下一拍轮询时随 `Update.commit` 取走写进去，
//! 赶在轮询前又敲了键就接在那次按键结果前面。只对 v8 起的 DLL 收点击，更老的读不到 `commit`。
//! 翻页（箭头、滚轮）不上屏，不挑 DLL 版本。

use qingjian_platform::protocol::SessionId;
use qingjian_render::HitTarget;

use crate::dispatch::Router;

/// 从这一版协议起 DLL 会把 `Update.commit` 写进文档。
const CLICK_SINCE: u32 = 8;

impl Router {
    /// 候选窗口上点中了候选或译词（UI 线程经工人通道送来）。
    pub fn handle_click(&mut self, target: HitTarget) {
        if let HitTarget::Page(step) = target {
            return self.handle_page_turn(step);
        }
        let Some(session) = self.click_session() else {
            return;
        };
        if self.translation.is_some() || !self.composing() {
            return;
        }
        let text = match target {
            HitTarget::Candidate(offset) => self
                .slot_index(offset + 1)
                .and_then(|index| self.commit_index(index)),
            HitTarget::Translation { candidate, sense } => self
                .annotated_candidate_on_page(candidate + 1)
                .and_then(|candidate| self.engine.commit_translation(&candidate, sense)),
            // 菜单项在菜单自己的窗口里点，候选窗不会给出；翻页上面已经处理
            HitTarget::MenuItem(_) | HitTarget::Page(_) => None,
        };
        let Some(text) = text else {
            tracing::debug!(?target, "点中的格子没有可上屏的");
            return;
        };
        tracing::debug!(?target, "点击上屏");
        self.deliver_clicked(session, text);
    }

    /// 点翻页箭头或滚轮翻页：和翻页键一样走 `page`，只重画候选窗，组句文字不变、不用告诉 DLL。
    pub fn handle_page_turn(&mut self, step: isize) {
        if self.focused.is_none() || self.translation.is_some() || !self.composing() {
            return;
        }
        self.page(step);
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
    }

    /// 收得了点击的聚焦会话：DLL 太老、读不到 `Update.commit` 时不收。
    pub(in crate::dispatch) fn click_session(&self) -> Option<SessionId> {
        let session = self.focused?;
        if self.focused_dll_protocol() < CLICK_SINCE {
            tracing::debug!("DLL 协议太老，不收鼠标上屏");
            return None;
        }
        Some(session)
    }

    /// 点击上屏的文本挂到会话上等 DLL 取走，再按新状态重画。
    pub(in crate::dispatch) fn deliver_clicked(&mut self, session: SessionId, text: String) {
        let pending = self.take_clicked(session).unwrap_or_default() + &text;
        // 选的词没选完一段拼音时留在组句里，没有要交给 DLL 的
        self.clicked = (!pending.is_empty()).then_some((session, pending));
        self.recompose();
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
    }

    /// 取走这个会话点击上屏、还没交出去的文本。
    pub(in crate::dispatch) fn take_clicked(&mut self, session: SessionId) -> Option<String> {
        if self
            .clicked
            .as_ref()
            .is_some_and(|(owner, _)| *owner == session)
        {
            self.clicked.take().map(|(_, text)| text)
        } else {
            None
        }
    }
}
