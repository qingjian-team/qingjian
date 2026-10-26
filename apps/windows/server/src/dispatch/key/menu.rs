//! 候选窗口右键：问这一格能做哪几件事、叫 UI 线程弹菜单；点中的项交给平台层的候选菜单去做，
//! 那句提示随下一帧下发（画在拼音行旁，下一次按键清）。

use qingjian_core::Candidate;
use qingjian_platform::Config;
use qingjian_platform::candidate_menu::{self, CandidateAction};

use crate::dispatch::{CandidateMenu, CandidateMouse, Router};

impl Router {
    /// 候选窗口上的鼠标操作（UI 线程经工人通道送来）。
    pub fn handle_candidate_mouse(&mut self, mouse: CandidateMouse) {
        match mouse {
            CandidateMouse::Click(target) => self.handle_click(target),
            CandidateMouse::Menu { offset, point } => self.handle_candidate_menu(offset, point),
            CandidateMouse::MenuPick { offset, id } => self.handle_candidate_menu_pick(offset, id),
        }
    }

    /// 候选窗口上右键了这一页第 `offset` 格（UI 线程经工人通道送来），`point` 是光标的屏幕坐标。
    pub fn handle_candidate_menu(&mut self, offset: usize, point: (i32, i32)) {
        if self.click_session().is_none() || self.translation.is_some() || !self.composing() {
            return;
        }
        let Some(candidate) = self.candidate_in_slot(offset) else {
            return;
        };
        let items: Vec<_> =
            candidate_menu::actions(&self.engine, &candidate, &self.configured_phrases())
                .into_iter()
                .map(|action| (action.id(), action.label()))
                .collect();
        if items.is_empty() {
            return;
        }
        tracing::debug!(offset, ?items, "候选右键菜单");
        self.candidates.show_menu(CandidateMenu {
            offset,
            point,
            items,
        });
    }

    /// 右键菜单里点中了编号 `id` 的项。
    pub fn handle_candidate_menu_pick(&mut self, offset: usize, id: u32) {
        let Some(action) = CandidateAction::from_id(id) else {
            return;
        };
        if self.translation.is_some() || !self.composing() {
            return;
        }
        let Some(candidate) = self.candidate_in_slot(offset) else {
            return;
        };
        let path = self.config_path().map(|path| path.to_path_buf());
        let phrases = self.configured_phrases();
        let outcome = candidate_menu::perform(
            action,
            &mut self.engine,
            &candidate,
            path.as_deref(),
            &phrases,
        );
        // 不等热加载那一秒：短语当场换上
        if outcome.phrases_changed
            && let Err(error) = self.engine.set_custom_phrases(self.configured_phrases())
        {
            tracing::warn!(%error, "置顶后的自定义短语未应用");
        }
        tracing::info!(?action, notice = %outcome.notice, "候选右键菜单");
        self.notice = Some(outcome.notice);
        self.recompose();
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
    }

    /// 这一页第 `offset` 格（从 0 起）的候选。
    fn candidate_in_slot(&self, offset: usize) -> Option<Candidate> {
        self.slot_index(offset + 1)
            .and_then(|index| self.layout_candidate(index))
    }

    /// 配置文件里的自定义短语；没有配置文件（测试）或读不了时为空。
    fn configured_phrases(&self) -> Vec<qingjian_core::CustomPhrase> {
        self.config_path()
            .and_then(|path| Config::load(path).ok())
            .map(|config| config.custom_phrases)
            .unwrap_or_default()
    }
}
