//! 翻译选中文字、译词键与删候选键。

use super::*;

impl QingjianInputController {
    /// Option+数字：上屏当前页第几个候选的译文（学习和拼音消耗与选那个候选一样）。
    /// 不在组句中时不管；候选没有译文就吞掉按键不动，免得 ¡™£ 进应用。
    /// 翻译应用里选中的文字：云服务关着、密码框、没有选区都不动（键交回应用）。
    pub(super) fn translate_selection(&self, client: TextClient<'_>) -> bool {
        if !host::with(|h| h.engine.prediction_enabled()).unwrap_or(false) {
            tracing::info!("云服务没开，翻译快捷键不生效");
            return false;
        }
        if secure_input::enabled() {
            tracing::debug!("Secure Input 中，不翻译");
            return false;
        }
        let Some((text, range)) = client.selected_text(MAX_TRANSLATE_CHARS) else {
            // 分不清是没选还是应用不给读（不少 Electron 应用不支持），两种情况都提示一下，键吞掉
            tracing::debug!("没有选中的文字，或应用不支持读选区");
            let anchor = client.caret_rect();
            host::with(|h| {
                h.show_notice(
                    "没有选中的文字，或这个应用不支持读取选区（最多 500 字）",
                    anchor,
                )
            });
            return true;
        };
        // 光标位置先在借用之外取好：取的过程会等应用回话，期间别的 IMK 回调可能重入
        let anchor = client.caret_rect();
        let sent = host::with(|h| {
            h.anchor = anchor;
            h.engine.request_translation(&text).is_some()
        })
        .unwrap_or(false);
        if !sent {
            return false;
        }
        tracing::debug!(chars = text.chars().count(), "翻译选中文字");
        host::with(|h| h.begin_translation(range));
        true
    }

    /// 翻译窗口开着时的按键：回车 / 空格 / 1 用译文替换选区，Esc 放弃；其他键放弃并交回应用。
    pub(super) fn handle_translation_review(&self, key: u16, client: TextClient<'_>) -> bool {
        let job = host::with(|h| h.translation.clone()).flatten();
        let Some(job) = job else {
            return false;
        };
        match key {
            // 回车 / 小键盘回车 / 空格 / 1：接受（译文还没到时先等）
            36 | 76 | 49 | 18 => {
                if let Some(result) = job.result {
                    tracing::debug!("接受译文");
                    client.replace_range(&result, job.range);
                    host::with(|h| h.end_translation());
                }
                true
            }
            // Esc：放弃
            53 => {
                host::with(|h| h.end_translation());
                true
            }
            _ => {
                host::with(|h| h.end_translation());
                false
            }
        }
    }

    pub(super) fn handle_translation_key(
        &self,
        digit: usize,
        sense: usize,
        client: TextClient<'_>,
    ) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let candidate = host::with(|h| {
            h.session
                .index_on_page(digit - 1)
                .and_then(|index| h.session.candidate(index))
        })
        .flatten();
        let text = candidate
            .and_then(|c| host::with(|h| h.engine.commit_translation(&c, sense)).flatten());
        match text {
            Some(text) => {
                tracing::debug!(%text, "commit translation");
                client.insert_text(&text);
                self.refresh(client);
            }
            None => tracing::debug!(digit, sense, "这个候选没有这条译文"),
        }
        true
    }

    /// 修饰键 + 数字（缺省 ⇧）：删掉当前页第几个候选。不在组句中不管；那格没有候选就吞掉按键不动。
    /// 删完重新查一遍（排序会变），结果那句话显示在拼音行右侧，敲下一键就没了。
    pub(super) fn handle_delete_key(&self, digit: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let Some(message) = host::with(|h| h.forget_candidate(digit - 1)).flatten() else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return true;
        };
        tracing::info!(%message);
        self.refresh(client);
        host::with(|h| h.status = Some(message));
        self.render(client);
        true
    }
}
