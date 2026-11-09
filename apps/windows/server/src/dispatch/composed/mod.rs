//! 组句的展示状态：缓冲变化时重查候选并重建 [`Composed`]；云联想在 [`prediction`]，高亮 / 翻页在 [`navigation`]，
//! 给 DLL 与自绘窗的帧在 [`frame`]。

mod frame;
mod navigation;
mod prediction;
mod state;

use qingjian_core::candidate::arrows::Focus;
use qingjian_core::{Candidate, CandidateLayout, Query};
use qingjian_platform::protocol::PreeditSegment;

pub(super) use self::state::{Composed, TypedKeys};
use super::Router;

impl Router {
    /// 缓冲变化后：按 Engine 状态重建 [`Composed`]，发一次云联想请求，归零高亮与整句补全。
    pub(super) fn recompose(&mut self) {
        self.composed = None;
        self.highlight = 0;
        self.navigated = false;
        self.grid = None;
        self.sentence = None;
        if self.engine.composition().is_empty() {
            self.focus = Focus::default();
            // 清空时也要撤掉 worker 中的待发请求，避免无用请求占掉最短间隔。
            self.engine.cancel_prediction();
            self.stop_rescoring();
            return;
        }
        self.engine.cancel_composition_prediction();
        self.attach_loaded_model();
        let built = self.engine.query().ok().map(|mut query| {
            let (preedit, cursor, typed_keys) = marked_parts(&query);
            let items = std::mem::take(&mut query.candidates.items);
            (items, preedit, cursor, typed_keys, Box::new(query))
        });
        self.composed = Some(match built {
            Some((items, preedit, cursor, typed_keys, spelling)) => {
                let layout =
                    CandidateLayout::new(items, self.config.page_size, self.config.cloud_slots);
                if self.config.cloud_enabled && self.engine.prediction_enabled() {
                    let surrounding = self.surrounding_for_prediction();
                    self.engine.request_prediction(surrounding, layout.local());
                }
                Composed::Candidates {
                    preedit,
                    cursor,
                    typed_keys,
                    layout,
                    spelling,
                    manual_sequence: None,
                }
            }
            None => {
                self.cancel_prediction();
                let composition = self.engine.composition();
                let text = composition.text().to_owned();
                let cursor = text[..composition.cursor()].chars().count();
                Composed::Raw { text, cursor }
            }
        });
        self.schedule_rescoring();
    }

    pub(super) fn candidate_count(&self) -> usize {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.len(),
            Some(Composed::Vietnamese { candidates, .. }) => candidates.len(),
            _ => 0,
        }
    }

    /// 候选布局里第 `index` 个（跨页下标）。
    pub(super) fn layout_candidate(&self, index: usize) -> Option<Candidate> {
        match &self.composed {
            Some(Composed::Candidates { layout, .. }) => layout.candidate(index).cloned(),
            Some(Composed::Vietnamese { candidates, .. }) => candidates.get(index).cloned(),
            _ => None,
        }
    }

    pub(super) fn commit_index(&mut self, index: usize) -> Option<String> {
        if let Some(Composed::Vietnamese { candidates, .. }) = &self.composed {
            let text = candidates.get(index)?.text.clone();
            let phrase_prefix = self.vietnamese.phrase_prefix();
            let phrase =
                phrase_prefix != self.vietnamese.current() && text.starts_with(phrase_prefix);
            if phrase {
                self.vietnamese.choose_phrase(&text, ' ');
            } else {
                self.vietnamese.commit_word(Some(&text), ' ');
            }
            self.sync_vietnamese_preedit();
            return Some(String::new());
        }
        let candidate = self.layout_candidate(index)?;
        Some(self.engine.commit(&candidate))
    }
}

/// 一次查询的拼音行分段与光标，以及输入框另显示原始按键时的那一串。
/// 光标用 Core 的映射：自动补的 `'` 会让显示串比敲的长。
pub(super) fn marked_parts(query: &Query) -> (Vec<PreeditSegment>, usize, Option<TypedKeys>) {
    let preedit = query.marked_segments().iter().map(Into::into).collect();
    let typed_keys = query.shuangpin_raw_preedit.then(|| TypedKeys {
        text: query.marked_text(),
        cursor: query.marked_cursor(),
    });
    (preedit, query.segments_cursor(), typed_keys)
}

#[cfg(test)]
mod tests;
