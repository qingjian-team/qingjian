//! 上屏：高亮候选、指定候选、拼音原样。

use super::QingjianInputController;
use crate::host;
use crate::imk::TextClient;

impl QingjianInputController {
    pub(super) fn commit_highlighted(&self, client: TextClient<'_>) -> bool {
        let index = host::with(|h| h.session.highlighted).unwrap_or(0);
        self.commit_index(index, client)
    }

    /// 上屏第 `index` 个候选；没有候选时上屏拼音本身。上屏后剩余拼音继续组句。
    pub(super) fn commit_index(&self, index: usize, client: TextClient<'_>) -> bool {
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
        tracing::debug!(%text, "commit");
        host::with(|h| h.commit_live_text(&text));
        client.insert_text(&text);
        self.refresh(client);
        true
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
        host::with(|h| h.commit_live_text(&raw));
        client.insert_text(&raw);
        self.refresh(client);
        true
    }
}
