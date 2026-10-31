//! 上屏：高亮候选、指定候选、拼音原样。

use super::*;

impl QingjianInputController {
    pub(super) fn commit_highlighted(&self, client: TextClient<'_>) -> bool {
        // 形码空码：编码查不到词，清掉编码，不把字母原样打进正文
        if host::with(|h| h.engine.composing_code() && h.session.layout.is_empty()).unwrap_or(false)
        {
            host::with(|h| h.engine.clear());
            self.refresh(client);
            return true;
        }
        let index = host::with(|h| h.session.highlighted).unwrap_or(0);
        self.commit_index(index, client)
    }

    /// 上屏第 `index` 个候选；没有候选时上屏拼音本身。上屏后剩余拼音继续组句。
    pub(super) fn commit_index(&self, index: usize, client: TextClient<'_>) -> bool {
        if host::with(|h| h.vietnamese_mode()).unwrap_or(false) {
            return self.commit_vietnamese_candidate(index, client);
        }
        let candidate = host::with(|h| h.session.candidate(index)).flatten();
        let Some(candidate) = candidate else {
            if host::with(|h| index < h.session.layout.len()).unwrap_or(false) {
                return true;
            }
            return self.commit_raw(client);
        };
        let Some(text) = host::with(|h| h.engine.commit(&candidate)) else {
            return false;
        };
        // 没选完一段拼音时词留在组句里（空串），随拼音一起显示
        if !text.is_empty() {
            tracing::debug!(%text, "commit");
            client.insert_text(&text);
        }
        self.refresh(client);
        true
    }

    /// 上屏高亮候选，后面还要接着插入别的（标点、粘贴的文字）：没选完拼音时留在组句里的词先交给应用，
    /// 免得插到它们前面。
    pub(super) fn commit_highlighted_then(&self, client: TextClient<'_>) {
        self.commit_highlighted(client);
        let held = host::with(|h| h.engine.take_held()).unwrap_or_default();
        if !held.is_empty() {
            client.insert_text(&held);
            self.refresh(client);
        }
    }

    /// 把拼音原样上屏并清空。缓冲区为空时返回 false。
    pub(super) fn commit_raw(&self, client: TextClient<'_>) -> bool {
        let Some(raw) = host::with(|h| h.engine.take_raw()) else {
            return false;
        };
        if raw.is_empty() {
            return false;
        }
        tracing::debug!(%raw, "commit raw");
        client.insert_text(&raw);
        self.refresh(client);
        true
    }
}
