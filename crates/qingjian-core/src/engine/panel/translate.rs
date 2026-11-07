//! 面板翻译组：框里打字、选词进原文，停键后发翻译，回车把译文交给应用。
//! 原文与进度在 [`crate::panel::SymbolPanel`] 里，这里管请求、缓存与上屏；壳每拍调 [`Engine::tick_panel_translation`]。

use std::time::Instant;

use crate::Language;
use crate::candidate::Candidate;
use crate::engine::Engine;
use crate::panel::{PanelGroup, TranslateStatus};

impl Engine {
    /// 翻译组能不能用：接了翻译提供方、没在私密输入。
    pub(super) fn panel_translate_available(&self) -> bool {
        self.translation_enabled() && !self.private
    }

    /// 翻译组缺省的目标语言：上次选的，没选过跟学习语言走（学习语言是中文或关掉时用英语）。
    pub(super) fn panel_language(&self) -> Language {
        self.panel_language
            .unwrap_or(match self.translator.language() {
                Language::Chinese => Language::English,
                language => language,
            })
    }

    /// 快捷键：打开面板并停在翻译组。翻译不可用时不开，返回 `false`。
    pub fn open_translate_panel(&mut self) -> bool {
        if !self.panel_translate_available() {
            return false;
        }
        self.open_symbol_panel();
        self.panel
            .as_mut()
            .is_some_and(|panel| panel.open_on(PanelGroup::Translate))
    }

    /// 面板开着而且停在翻译组：这时敲的字进框、选的词进原文，不进应用。
    pub fn panel_translating(&self) -> bool {
        self.panel
            .as_ref()
            .is_some_and(|panel| panel.group() == PanelGroup::Translate)
    }

    /// 直接往框里加字（英文模式下敲的字母、空格）。不在翻译组返回 `false`。
    pub fn push_panel_text(&mut self, text: &str) -> bool {
        if !self.panel_translating() {
            return false;
        }
        if let Some(panel) = self.panel.as_mut() {
            panel.draft_mut().push(text, Instant::now());
        }
        true
    }

    /// 框里敲的标点：中文标点开着时按组句外的规则换成全角。不在翻译组返回 `false`。
    pub fn push_panel_char(&mut self, c: char) -> bool {
        match self.full_width_punctuation_of(c) {
            Some(text) => self.push_panel_text(text),
            None => self.push_panel_text(c.encode_utf8(&mut [0; 4])),
        }
    }

    /// 删选中的字或光标前一个字；框里没字时返回 `false`（壳照别的组收起面板），光标在最前面时什么都不做、返回 `true`。
    pub fn panel_backspace(&mut self) -> bool {
        if !self.panel_translating() {
            return false;
        }
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        if panel.translate_source().is_empty() {
            return false;
        }
        panel.draft_mut().pop(Instant::now());
        true
    }

    /// Delete：删选中的字或光标后一个字。返回删没删。
    pub fn panel_delete(&mut self) -> bool {
        self.panel_translating()
            && self
                .panel
                .as_mut()
                .is_some_and(|panel| panel.draft_mut().delete(Instant::now()))
    }

    /// 光标左右挪 `delta` 个字（←→）；`extend` 时扩选区（Shift）。返回光标或选区变没变。
    pub fn move_panel_cursor(&mut self, delta: isize, extend: bool) -> bool {
        self.panel_translating()
            && self
                .panel
                .as_mut()
                .is_some_and(|panel| panel.draft_mut().move_cursor(delta, extend))
    }

    /// 光标放到第 `position` 个字之后（点框里的字、Home / End）；超出就到最后。
    /// `extend` 时选区从原来的光标拉到这里（拖动、Shift 点击、Shift+Home / End）。返回光标或选区变没变。
    pub fn place_panel_cursor(&mut self, position: usize, extend: bool) -> bool {
        self.panel_translating()
            && self
                .panel
                .as_mut()
                .is_some_and(|panel| panel.draft_mut().place_cursor(position, extend))
    }

    /// 全选框里的原文（Cmd / Ctrl+A）。返回变没变。
    pub fn select_all_panel_text(&mut self) -> bool {
        self.panel_translating()
            && self
                .panel
                .as_mut()
                .is_some_and(|panel| panel.draft_mut().select_all())
    }

    /// 删掉框里选中的字（开始打拼音时）。
    pub(in crate::engine) fn remove_panel_selection(&mut self) {
        if self.panel_translating()
            && let Some(panel) = self.panel.as_mut()
        {
            panel.draft_mut().remove_selection(Instant::now());
        }
    }

    /// 在翻译组里选词：学习照常记，选出的字进框里的原文而不是交给应用。不在翻译组返回 `false`。
    pub fn commit_to_panel(&mut self, candidate: &Candidate) -> bool {
        if !self.panel_translating() {
            return false;
        }
        let mut panel = self.panel.take().expect("刚看过面板开着");
        // 框里的字不在应用里：自动空格与退格撤销都不能拿它当上文
        self.recent_commits.clear();
        let text = self.commit(candidate);
        self.recent_commits.clear();
        self.chain.reset();
        panel.draft_mut().push(&text, Instant::now());
        self.panel = Some(panel);
        true
    }

    /// 回车时译文还没好：不等停键，下一拍就发。
    pub fn hurry_panel_translation(&mut self) {
        if let Some(panel) = self.panel.as_mut() {
            panel.draft_mut().hurry();
        }
    }

    /// 回车上屏：译文对得上框里的原文与语言就收起面板、清掉拼音，返回译文；还没好返回 `None`。
    pub fn take_panel_translation(&mut self) -> Option<String> {
        let panel = self
            .panel
            .as_ref()
            .filter(|p| p.group() == PanelGroup::Translate)?;
        let text = panel.draft().ready(panel.target_language()?)?.to_owned();
        self.clear();
        self.recent_commits.clear();
        self.chain.reset();
        Some(text)
    }

    /// 应用的 preedit 里该显示什么：翻译组里最近一条译文。不在翻译组或还没有译文时为 `None`。
    pub fn panel_preedit(&self) -> Option<&str> {
        let panel = self.panel.as_ref()?;
        (panel.group() == PanelGroup::Translate)
            .then(|| panel.translation())
            .flatten()
    }

    /// 翻译组还在等停键或等译文：壳要接着轮询（[`Self::tick_panel_translation`]）。
    pub fn panel_translation_pending(&self) -> bool {
        self.panel_translating()
            && self.panel.as_ref().is_some_and(|panel| {
                panel.translate_status() == TranslateStatus::Waiting
                    || panel.draft().retry_pending()
            })
    }

    /// 每拍调一次（先 [`Self::poll_prediction`]）：停键够久就查缓存或发请求。返回翻译框有没有变化要重画。
    pub fn tick_panel_translation(&mut self, now: Instant) -> bool {
        if !self.panel_translating() {
            return false;
        }
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        let Some(chosen) = panel.target_language() else {
            return false;
        };
        // 上层清过全部联想（拼音删光时会）：在飞的请求没了，重发
        if let Some(sequence) = panel.draft().in_flight()
            && !self.pending_predictions.contains_key(&sequence)
        {
            panel.draft_mut().forget_request();
        }
        if let Some(job) = panel.draft().due(now, chosen) {
            if let Some(text) = self.panel_translations.get(&job) {
                panel.draft_mut().resolve(job, text.to_owned());
            } else {
                let sent = self.submit_panel_translation(&job.source, job.language);
                let draft = self.panel.as_mut().expect("发请求不动面板").draft_mut();
                match sent {
                    Some(sequence) => draft.sent(sequence, job),
                    None => draft.fail(job, now),
                }
            }
        }
        self.panel
            .as_mut()
            .is_some_and(|panel| panel.draft_mut().take_unseen())
    }

    /// [`Self::poll_prediction`] 收到的结果是翻译框发的就记下（进缓存），返回 `true`，不再交给壳。
    pub(in crate::engine) fn answer_panel_translation(
        &mut self,
        sequence: u64,
        sentence: &Option<String>,
    ) -> bool {
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        let Some((job, text)) =
            panel
                .draft_mut()
                .answer(sequence, sentence.clone(), Instant::now())
        else {
            return false;
        };
        match text {
            Some(text) => self.panel_translations.insert(job, text),
            None => tracing::info!("panel translation: no result from provider"),
        }
        true
    }
}
