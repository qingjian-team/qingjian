//! 只用形码时的按键规则：满码唯一自动上屏、顶字，以及标点与空码（见 [`Engine::composing_code`]）。
//!
//! 判断在 Core，动作分两种：顶字上屏的是壳里的高亮候选（用户可能挪过高亮），所以只告诉壳该不该顶；
//! 自动上屏发生在刚敲完一键、还没有可挪的高亮时，Core 直接按首选上屏。

use super::Engine;
use crate::candidate::CandidateKind;
use crate::code_scheme::CodeScheme;

impl Engine {
    /// 敲 `c` 之前要不要先顶字：方案开着顶字、编码已满码长且有候选、`c` 是字母。
    /// 是的话壳先把高亮候选上屏，再把 `c` 收进缓冲区，它就是下一段编码的第一个字母。
    pub fn code_tops(&self, c: char) -> bool {
        let Some(scheme) = self.plain_code() else {
            return false;
        };
        let scope = self.composition.scope();
        scheme.top
            && c.is_ascii_lowercase()
            && scope.len() >= scheme.code_length
            && !scheme.table.lookup(scope, 1).is_empty()
    }

    /// 方案开着自动上屏、编码刚好满码长且这个全码只有一个词时把它上屏，返回上屏的文本（与 [`Self::commit`] 一样）。
    /// 壳在每次把字母收进缓冲区之后调。同码的自定义短语排在它前面时不自动上屏，留给用户选。
    pub fn commit_unique_code(&mut self) -> Option<String> {
        let scheme = self.plain_code()?;
        let scope = self.composition.scope();
        if !scheme.auto_commit || scope.len() != scheme.code_length {
            return None;
        }
        let hits = scheme.table.lookup(scope, 2);
        if !matches!(hits.as_slice(), [only] if only.exact) {
            return None;
        }
        // 走一遍完整查询：上屏要的是出 Core 的那份候选（繁体输出开着时是繁体）
        let first = self
            .query()
            .ok()?
            .candidates
            .items
            .into_iter()
            .next()
            .filter(|candidate| candidate.kind == CandidateKind::Code)?;
        Some(self.commit(&first))
    }

    /// 只用形码、缓冲区里是一段编码。壳据此按五笔的习惯处理两件事：
    /// 会转全角的标点总是先把高亮候选上屏再补标点（不看 `[general] punct_commits`），
    /// 编码查不到词（空码）时空格与标点把编码清掉，不把字母原样打进正文。
    pub fn composing_code(&self) -> bool {
        self.plain_code().is_some()
    }

    /// 这些规则生效的前提：只用形码（混输下满码之后的字母还要给拼音）、不在英文 / 辅码态、
    /// 光标在末尾、缓冲区全是字母（问字的 `?`、直输段的 `-` 都不算编码）。
    fn plain_code(&self) -> Option<&CodeScheme> {
        let scheme = self.code.as_ref().filter(|_| !self.phonetic)?;
        let text = self.composition.text();
        let plain = !self.english_mode
            && self.aux_code.is_none()
            && self.composition.cursor() == text.len()
            && !text.is_empty()
            && text.chars().all(|c| c.is_ascii_lowercase());
        plain.then_some(scheme)
    }
}
