//! 「翻译选中文字」：快捷键 → 请 DLL 读选区 → 翻译提供方 → 候选窗显示译文，回车 / 空格替换、Esc 保留。
//! 引擎调用在 Core。TSF 来源回填前复核原范围；备用来源只在用户确认时复制译文。
//! 进行态在 [`Translation`]。

mod clipboard;
mod fallback;
mod job;
mod reading;

pub(super) use clipboard::copy_text;

#[cfg(test)]
mod tests;

use qingjian_core::{Candidate, CandidateKind, CandidateList};
use qingjian_platform::protocol::{
    Frame, KeyEvent, KeyModifiers, KeyOutcome, ScreenRect, ServerMessage, SessionId,
};

pub(super) use self::job::Translation;
use super::Router;
use super::key::{ESCAPE, RETURN};

impl Router {
    /// 修饰键比物理组合（去掉 Caps / 中英模式两个状态位）。
    pub(super) fn matches_translate_combo(&self, event: &KeyEvent) -> bool {
        let combo = self.config.translate_selection;
        event.character == Some(combo.key)
            && event.modifiers.chord() == KeyModifiers::from(combo.modifiers)
    }

    /// DLL 回来的选区：非空且翻译提供方可用就进入评审；否则回空帧让 DLL 清掉本地翻译态。
    /// 回给 DLL 的帧恒空（候选窗在 Server 自绘）。
    pub(super) fn handle_selection(
        &mut self,
        session: SessionId,
        request: u64,
        text: String,
        rect: ScreenRect,
    ) -> ServerMessage {
        let empty = ServerMessage::KeyResult {
            session,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame: Frame::default(),
            english: None,
        };
        if self.focused != Some(session) || self.pending_selection != Some(request) {
            return empty;
        }
        self.pending_selection = None;
        if text.trim().is_empty() || !self.engine.translation_enabled() {
            tracing::info!("翻译选中文字：没有可读的选区（或翻译未启用）");
            return empty;
        }
        self.last_rect = Some(rect);
        self.translation = Some(Translation::new(false));
        self.engine.request_translation(&text);
        tracing::debug!(chars = text.chars().count(), "翻译选中文字：已发翻译请求");
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        empty
    }

    /// 评审态收到按键：回车 / 空格 / 1 接受（译文没到就只吃键并保留请求），Esc 放弃，其余放弃并交回应用。
    pub(super) fn handle_translation_review(
        &mut self,
        session: SessionId,
        event: &KeyEvent,
    ) -> ServerMessage {
        let accept =
            event.virtual_key == RETURN || matches!(event.character, Some(' ') | Some('1'));
        let escape = event.virtual_key == ESCAPE;
        if accept && self.translation.as_ref().is_some_and(|t| t.copy_only) {
            return self.accept_fallback_translation(session);
        }
        let result = self
            .translation
            .as_ref()
            .and_then(|translation| translation.result.clone());
        if accept && result.is_none() {
            // 新 DLL 需保留选区锚点和轮询；老 DLL 不能解析 SelectionReview，仍回旧消息。
            return if self.focused_dll_protocol() >= 10 {
                self.show_selection_review(session)
            } else {
                ServerMessage::KeyResult {
                    session,
                    outcome: KeyOutcome::Consumed,
                    commit: None,
                    frame: Frame::default(),
                    english: None,
                }
            };
        }
        self.end_translation();
        let (commit, outcome) = if accept {
            (result, KeyOutcome::Consumed)
        } else if escape {
            (None, KeyOutcome::Consumed)
        } else {
            (None, KeyOutcome::Passthrough)
        };
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame: Frame::default(),
            english: None,
        }
    }

    /// 结束评审：丢进行态、取消在飞的请求、收起候选窗。没在评审时是空操作。
    pub(super) fn end_translation(&mut self) {
        if self.translation.take().is_some() {
            self.cancel_prediction();
            self.hide_candidate_window();
        }
    }

    /// 评审帧：单条候选（译文或「翻译中…」），无 preedit、无分页。
    pub(super) fn translation_frame(&self, translation: &Translation) -> Frame {
        let text = translation.result.clone().unwrap_or_else(|| {
            translation
                .status
                .clone()
                .unwrap_or_else(|| "翻译中…".to_owned())
        });
        Frame {
            preedit: Vec::new(),
            preedit_mode: self.config.preedit,
            cursor: 0,
            window_preedit: None,
            candidates: CandidateList {
                items: vec![translate_candidate(text)],
            },
            highlight: 0,
            page: 0,
            page_count: 1,
            layout: self.config.layout,
            columns: 0,
            column_ems: Vec::new(),
            appearance: self.config.appearance,
            aux_code_show: self.config.aux_code_show,
            // 这条候选是「翻译选中文字」的译文本身，没有词性注记；值不影响渲染
            translation_pos: self.config.translation_pos,
            sentence: None,
            notice: translation.copy_only.then(|| {
                match (&translation.status, &translation.result) {
                    (Some(reason), Some(_)) => format!("{reason}；Enter 重试复制；Esc 取消"),
                    _ if translation.copied => "Esc 关闭；Ctrl+V 自行粘贴".to_owned(),
                    _ => "Enter / 空格 / 1 复制译文；Esc 取消；不自动替换原文".to_owned(),
                }
            }),
            symbol_panel: false,
        }
    }
}

/// 译文包成一条云端样式的候选（带云朵标记）。
fn translate_candidate(text: String) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::Cloud,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}
