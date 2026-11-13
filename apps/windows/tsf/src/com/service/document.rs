//! 往文档写字：按键结果经异步编辑会话写上屏文本 + 组句拼音行；失焦 / 停用 / 切模式时让 Server 交出缓冲区原样落定；
//! 翻译选中文字时起只读会话读选区。

use qingjian_platform::protocol::CaretEdit;
use windows::Win32::UI::TextServices::ITfContext;
use windows::core::Ref;

use super::TextService_Impl;
use crate::com::edit::{request_selection, request_update};
use crate::com::log::log;

impl TextService_Impl {
    pub(super) fn read_selection(&self, pic: Ref<ITfContext>, request: u64) {
        let Ok(context) = pic.ok() else {
            log("翻译选中文字：无上下文，取消读选区");
            return;
        };
        if let Err(error) = request_selection(
            context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            request,
        ) {
            log(&format!("请求读选区会话失败: {error}"));
        }
    }

    /// 失焦 / 停用 / 切模式：让 Server 交出缓冲区，原样落进最近收键的文档并收掉组句。
    /// 组句已被应用终止的（拼音已是普通文本）只清 Server 不再插。
    pub(super) fn commit_pending(&self) {
        if self.shared.translating() {
            self.shared.set_translating(false);
            self.shared.hide_candidates();
        }
        let stale = self.shared.take_server_stale();
        if !self.shared.composing() && !stale {
            return;
        }
        let text = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                self.shared.reset();
                return;
            };
            match client.commit() {
                Ok(text) => text,
                Err(error) => {
                    log(&format!(
                        "commit on focus loss failed, reconnecting on the next key: {error}"
                    ));
                    drop(guard);
                    self.disconnect();
                    return;
                }
            }
        };
        if stale {
            return;
        }
        self.shared.end_composing();
        let Some(context) = self.shared.last_context() else {
            log(&format!(
                "commit on focus loss has no context, dropped {} chars",
                text.as_deref().map_or(0, |text| text.chars().count())
            ));
            self.shared.reset();
            return;
        };
        log(&format!(
            "commit on focus loss: {} chars",
            text.as_deref().map_or(0, |text| text.chars().count())
        ));
        let requested = request_update(
            &context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            text.filter(|t| !t.is_empty()),
            String::new(),
            None,
        );
        if let Err(error) = requested {
            log(&format!(
                "edit session for the focus-loss commit was rejected: {error}"
            ));
            self.shared.reset();
        }
    }

    /// 经异步编辑会话把上屏文本 + 组句拼音行写进文档；`caret` 是成对符号要做的光标编辑。
    pub(super) fn update_document(
        &self,
        pic: Ref<ITfContext>,
        commit: Option<String>,
        preedit: String,
        caret: Option<CaretEdit>,
    ) {
        // 退到最后一个字母时帧已空但组句句柄还在，得跑一次把它收掉。
        if commit.is_none()
            && caret.is_none()
            && preedit.is_empty()
            && !self.shared.composing()
            && !self.shared.has_composition()
        {
            return;
        }
        let Ok(context) = pic.ok() else {
            log(&format!(
                "no context, update dropped: commit_len={} preedit_len={}",
                commit.as_deref().map_or(0, |text| text.chars().count()),
                preedit.chars().count()
            ));
            return;
        };
        if let Err(error) = request_update(
            context,
            self.client_id.get(),
            self.engine.clone(),
            self.shared.clone(),
            commit,
            preedit,
            caret,
        ) {
            log(&format!(
                "requesting the composition update failed: {error}"
            ));
        }
    }
}
