//! 备用选区评审：只读 UIA / Scintilla 的结果只在用户确认时复制，永不作为文档 commit。

use qingjian_platform::protocol::{ScreenRect, ServerMessage, SessionId};

use super::job::Translation;
use super::reading::{ReadJob, valid_text};
use crate::dispatch::Router;

impl Router {
    pub(in crate::dispatch) fn handle_selection_fallback(
        &mut self,
        session: SessionId,
        request: u64,
        target: (u32, u64),
        text: String,
        rect: ScreenRect,
    ) -> ServerMessage {
        let valid = self.focused == Some(session)
            && self.pending_selection == Some(request)
            && self.engine.translation_enabled()
            && !self.is_private()
            && self.focused_dll_protocol() >= 10;
        if !valid {
            return ServerMessage::SelectionReview {
                session,
                active: false,
            };
        }
        self.pending_selection = None;
        self.engine.cancel_prediction();
        self.last_rect = Some(rect);
        let mut translation = Translation::new(true);
        if text.is_empty() {
            translation.reading = Some(ReadJob::start(target.0, target.1));
            translation.status = Some("正在读取选中文字…".into());
        } else if valid_text(&text) {
            tracing::info!(
                units = text.encode_utf16().count(),
                "备用选区读取成功（Scintilla）"
            );
            self.engine.request_translation(&text);
        } else {
            translation.status = Some("选区为空或超过 500 个字符单元".into());
        }
        self.translation = Some(translation);
        self.show_selection_review(session)
    }

    pub(in crate::dispatch) fn poll_selection_reading(&mut self) {
        let result = self
            .translation
            .as_ref()
            .and_then(|t| t.reading.as_ref())
            .and_then(ReadJob::poll);
        let Some(result) = result else { return };
        let Some(translation) = self.translation.as_mut() else {
            return;
        };
        translation.reading = None;
        match result {
            Ok(text) if valid_text(&text) => {
                translation.status = None;
                self.engine.request_translation(&text);
            }
            Ok(_) => translation.status = Some("选区为空或过长，请重新选择".into()),
            Err(reason) => {
                tracing::info!(reason, "备用选区读取未完成");
                translation.status = Some(reason.into());
            }
        }
    }

    pub(super) fn show_selection_review(&mut self, session: SessionId) -> ServerMessage {
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
        ServerMessage::SelectionReview {
            session,
            active: self.translation.is_some(),
        }
    }

    pub(super) fn accept_fallback_translation(&mut self, session: SessionId) -> ServerMessage {
        let Some(translation) = self.translation.as_ref() else {
            return ServerMessage::SelectionReview {
                session,
                active: false,
            };
        };
        if translation.copied {
            self.end_translation();
            return ServerMessage::SelectionReview {
                session,
                active: false,
            };
        }
        if let Some(text) = translation.result.clone() {
            let copied = (self.selection_copy)(&text);
            if let Some(translation) = self.translation.as_mut() {
                match copied {
                    Ok(()) => {
                        translation.copied = true;
                        translation.result = None;
                        translation.status = Some("译文已复制，请按 Ctrl+V 粘贴".into());
                    }
                    Err(reason) => translation.status = Some(reason.into()),
                }
            }
        }
        // 等待 / 失败时都保留评审态，确认键不泄漏成聊天发送键。
        self.show_selection_review(session)
    }
}
