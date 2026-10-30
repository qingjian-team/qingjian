//! 候选窗口右键：问这一格能做哪几件事、叫 UI 线程弹菜单；点中的项交给平台层的候选菜单去做，
//! 那句提示随下一帧下发（画在拼音行旁，下一次按键清）。

use qingjian_core::{Candidate, Translator};
use qingjian_platform::Config;
use qingjian_platform::candidate_menu::{self, CandidateAction, IndependentInputContext};

use crate::dispatch::{CandidateMenu, CandidateMouse, Router};

impl Router {
    /// 候选窗口上的鼠标操作（UI 线程经工人通道送来）。
    pub fn handle_candidate_mouse(&mut self, mouse: CandidateMouse) {
        match mouse {
            CandidateMouse::Click(target) => self.handle_click(target),
            CandidateMouse::Menu { offset, point } => self.handle_candidate_menu(offset, point),
            CandidateMouse::MenuPick { offset, id } => self.handle_candidate_menu_pick(offset, id),
            CandidateMouse::TranslationEdited { offset, text } => {
                self.handle_translation_edited(offset, &text)
            }
            CandidateMouse::Page(step) => self.handle_page_turn(step),
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
        let actions = if !self.vietnamese.is_empty() {
            self.vietnamese_candidate_actions(&candidate)
        } else {
            candidate_menu::actions(&self.engine, &candidate, &self.configured_phrases())
        };
        let items: Vec<_> = actions
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

    fn handle_translation_edited(&mut self, offset: usize, text: &str) {
        if self.translation.is_some() || !self.composing() {
            return;
        }
        let Some(candidate) = self.candidate_in_slot(offset) else {
            return;
        };
        let updated = self.engine.set_candidate_translation(&candidate, text);
        self.notice = Some(if updated {
            format!("已更新「{}」的译文", candidate.text)
        } else {
            "译文不能为空，或当前未启用学习语言".to_owned()
        });
        if updated {
            self.refresh_candidate_translation(&candidate.text);
        }
        self.reconcile_candidates(&self.self_drawn_frame());
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
        if !self.vietnamese.is_empty() {
            self.handle_vietnamese_candidate_menu_pick(action, candidate);
            return;
        }
        if action == CandidateAction::EditTranslation {
            let current = candidate
                .translation
                .as_ref()
                .and_then(|translation| translation.senses().first())
                .map_or(String::new(), |sense| sense.text.clone());
            self.candidates.edit_translation(offset, current);
            return;
        }
        if action == CandidateAction::RegenerateTranslation {
            self.candidate_translation = self
                .engine
                .request_candidate_translation(&candidate.text)
                .map(|sequence| (sequence, candidate, None));
            self.notice = Some(if self.candidate_translation.is_some() {
                "模型正在重新生成译文…".to_owned()
            } else {
                "模型生成不可用".to_owned()
            });
            self.reconcile_candidates(&self.self_drawn_frame());
            return;
        }
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

    fn vietnamese_candidate_actions(&self, candidate: &Candidate) -> Vec<CandidateAction> {
        candidate_menu::independent_actions(
            &self.engine,
            candidate,
            self.vietnamese_candidate_context(),
        )
    }

    fn handle_vietnamese_candidate_menu_pick(
        &mut self,
        action: CandidateAction,
        candidate: Candidate,
    ) {
        let input = self.vietnamese.raw().to_owned();
        let context = IndependentInputContext {
            namespace: "vi",
            input: &input,
            translation_language: qingjian_core::Language::Chinese,
        };
        let Some(outcome) =
            candidate_menu::perform_independent(action, &mut self.engine, &candidate, context)
        else {
            return;
        };
        self.notice = Some(outcome.notice);
        if let Some(language) = outcome.translation_request {
            self.candidate_translation = self
                .engine
                .request_candidate_translation_to(&candidate.text, language)
                .map(|sequence| (sequence, candidate, Some(language)));
            if self.candidate_translation.is_none() {
                self.notice = Some("模型生成不可用".to_owned());
            }
        }
        self.sync_vietnamese_preedit();
        self.reconcile_candidates(&self.self_drawn_frame());
    }

    fn vietnamese_candidate_context(&self) -> IndependentInputContext<'_> {
        IndependentInputContext {
            namespace: "vi",
            input: self.vietnamese.raw(),
            translation_language: qingjian_core::Language::Chinese,
        }
    }

    /// 配置文件里的自定义短语；没有配置文件（测试）或读不了时为空。
    pub(super) fn configured_phrases(&self) -> Vec<qingjian_core::CustomPhrase> {
        self.config_path()
            .and_then(|path| Config::load(path).ok())
            .map(|config| config.custom_phrases)
            .unwrap_or_default()
    }

    pub(in crate::dispatch) fn refresh_candidate_translation(&mut self, text: &str) {
        let translation = self.engine.candidate_translation(text);
        if let Some(crate::dispatch::Composed::Candidates { layout, .. }) = self.composed.as_mut() {
            for candidate in layout.local_mut() {
                if candidate.text == text {
                    candidate.translation = translation.clone();
                }
            }
            for candidate in layout.cloud_mut() {
                if candidate.text == text {
                    candidate.translation = translation.clone();
                }
            }
        }
        if let Some(crate::dispatch::Composed::Vietnamese { candidates, .. }) =
            self.composed.as_mut()
        {
            for candidate in candidates {
                if candidate.text == text {
                    candidate.translation = translation.clone();
                }
            }
        }
    }

    pub(in crate::dispatch) fn set_independent_candidate_translation(
        &mut self,
        text: &str,
        language: qingjian_core::Language,
        translation: &str,
    ) {
        let (part_of_speech, translation) = match translation.trim().split_once('\t') {
            Some((part, text)) => (part.trim().parse().ok(), text.trim()),
            None => (None, translation.trim()),
        };
        if translation.is_empty() {
            return;
        }
        let translated = qingjian_core::Translation::new(
            language,
            vec![qingjian_core::Sense {
                part_of_speech,
                text: translation.to_owned(),
                reading: None,
                fresh: false,
            }],
        );
        if let Some(glossary) = self.vietnamese_glossary.as_mut() {
            glossary.learn(text, translated.clone());
            glossary.flush();
        }
        let persisted = self
            .vietnamese_glossary
            .as_ref()
            .and_then(|glossary| glossary.translate(text))
            .unwrap_or(translated);
        if let Some(crate::dispatch::Composed::Vietnamese { candidates, .. }) =
            self.composed.as_mut()
        {
            for candidate in candidates {
                if candidate.text == text {
                    candidate.translation = Some(persisted.clone());
                }
            }
        }
    }
}
