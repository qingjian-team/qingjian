mod snapshot;

pub(super) use snapshot::QuerySnapshot;

use crate::candidate::CandidateList;
use crate::correction::Correction;
use crate::parser::Segmentation;

/// 最优切分的音节用 `'` 连接，再接未切分尾部。
pub(crate) fn join_marked(segmentations: &[Segmentation], tail: &str) -> String {
    let mut text = segmentations
        .first()
        .map(|s| s.joined("'"))
        .unwrap_or_default();
    if !tail.is_empty() {
        if !text.is_empty() {
            text.push('\'');
        }
        text.push_str(tail);
    }
    text
}

use super::timings::Timings;
use super::{MarkedKind, MarkedSegment};

/// 不带译文的候选查询结果。
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// 参与候选生成的所有切分，索引 0 为首选切分。
    pub segmentations: Vec<Segmentation>,

    /// 排好序的候选，`translation` 均为 `None`。
    pub candidates: CandidateList,

    /// 输入末尾无法切分的字母（如 `kaifv` 的 `v`），不参与本次候选，留给后续输入。
    pub tail: String,

    /// 查询时缓冲区里的原始文本。
    pub text: String,

    /// 查询时的光标位置（`text` 的字节下标）。
    pub cursor: usize,

    /// 光标停在中间时，作用域之后剩下的拼音的显示形式（已按音节用 `'` 连好）；候选不管它，只画出来。
    pub rest: String,

    /// 各阶段耗时。
    pub timings: Timings,

    /// 生效的拼写纠正：`segmentations` 与候选都来自纠正后的拼音，`text` 仍是用户敲的。
    pub correction: Option<Correction>,

    /// 双拼开着：`segmentations` 是解出来的全拼，`text` 是敲的键，两者长度对不上。
    pub shuangpin: bool,
}

impl Query {
    /// 给 marked text 用的显示形式：最优切分的音节用 `'` 连接，再接未切分尾部，
    /// 光标后的剩余拼音跟在最后。`kaifa` → `kai'fa`，`kf` → `k'f`，`ni|hao` → `ni'hao`。
    pub fn marked_text(&self) -> String {
        self.marked_segments()
            .iter()
            .map(|s| s.text.as_str())
            .collect()
    }

    /// [`Self::marked_text`] 的分段形式：敲的拼音一段（`Typed`），光标后剩下的拼音连同前面的 `'` 一段（`Rest`）。
    /// 壳按段画样式；[`Self::marked_cursor`] 的位置按各段拼接后的字符数算。
    pub fn marked_segments(&self) -> Vec<MarkedSegment> {
        let mut segments = match &self.correction {
            Some(correction) => correction.marked_segments(),
            None => Vec::with_capacity(2),
        };
        if self.correction.is_none() {
            let typed = join_marked(&self.segmentations, &self.tail);
            if !typed.is_empty() {
                segments.push(MarkedSegment::new(typed, MarkedKind::Typed));
            }
        }
        if !self.rest.is_empty() {
            let rest = if segments.is_empty() {
                self.rest.clone()
            } else {
                format!("'{}", self.rest)
            };
            segments.push(MarkedSegment::new(rest, MarkedKind::Rest));
        }
        segments
    }

    /// 光标在 [`Self::marked_text`] 里的字符位置。自动补的 `'` 让显示串比原文长，按「光标前有几个字母」对齐；
    /// 光标原本在用户自己敲的 `'` 之后时，显示上也放到 `'` 之后。
    pub fn marked_cursor(&self) -> usize {
        // 纠错生效、双拼解码时显示串与敲的不一样长，作用域又总在光标前：光标就在敲的部分末尾
        // （双拼光标在开头时作用域是整段，光标仍在开头）
        if self.shuangpin && self.cursor == 0 {
            return 0;
        }
        if self.correction.is_some() || self.shuangpin {
            return self
                .marked_segments()
                .iter()
                .filter(|s| s.kind != MarkedKind::Rest)
                .map(|s| s.text.chars().count())
                .sum();
        }
        let letters_before = self.text[..self.cursor.min(self.text.len())]
            .chars()
            .filter(|c| *c != '\'')
            .count();
        let after_apostrophe = self.text[..self.cursor.min(self.text.len())].ends_with('\'');
        let marked: Vec<char> = self.marked_text().chars().collect();
        let mut seen = 0;
        let mut position = 0;
        while position < marked.len() && seen < letters_before {
            if marked[position] != '\'' {
                seen += 1;
            }
            position += 1;
        }
        if after_apostrophe && marked.get(position) == Some(&'\'') {
            position += 1;
        }
        position
    }
}
