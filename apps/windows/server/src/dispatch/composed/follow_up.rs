//! 纯五笔的选词键和上屏后联想。联想候选占一页，不持有待提交的编码。

use std::time::{Duration, Instant};

use qingjian_core::{Candidate, CandidateKind, CandidateList};
use qingjian_platform::protocol::{Frame, KeyEvent};
use qingjian_platform::{CommitKey, NumpadDigit};

use super::Composed;
use crate::dispatch::Router;
use crate::dispatch::key::{self, ESCAPE, Effect};

const FOLLOW_UP_TIMEOUT: Duration = Duration::from_secs(2);

impl Router {
    fn pure_wubi(&self) -> bool {
        !self.engine.is_phonetic()
            && self
                .engine
                .code_scheme()
                .is_some_and(|scheme| scheme.key == "wubi")
    }

    pub(in crate::dispatch) fn wubi_candidate_digit(&self, event: &KeyEvent) -> Option<usize> {
        if !self.config.wubi_candidate_keys
            || !self.pure_wubi()
            || event.modifiers.chord() != Default::default()
        {
            return None;
        }
        match event.character? {
            ';' => Some(2),
            '\'' => Some(3),
            _ => None,
        }
    }

    pub(super) fn offer_wubi_follow_ups(&mut self, committed: &str) {
        if !self.config.wubi_follow_ups || !self.pure_wubi() || self.panel_open() {
            return;
        }
        let words = self
            .engine
            .follow_ups_after_commit(committed, self.config.page_size.min(9));
        if !words.is_empty() {
            self.composed = Some(Composed::FollowUps {
                words,
                expires_at: Instant::now() + FOLLOW_UP_TIMEOUT,
            });
        }
    }

    pub(in crate::dispatch) fn has_follow_ups(&self) -> bool {
        matches!(self.composed, Some(Composed::FollowUps { .. }))
    }

    pub(in crate::dispatch) fn follow_up_remaining(&self) -> Option<Duration> {
        let Some(Composed::FollowUps { expires_at, .. }) = &self.composed else {
            return None;
        };
        Some(expires_at.saturating_duration_since(Instant::now()))
    }

    pub(in crate::dispatch) fn expire_follow_ups(&mut self) {
        if self
            .follow_up_remaining()
            .is_some_and(|remaining| remaining.is_zero())
        {
            self.hide_candidate_window();
        }
    }

    pub(in crate::dispatch) fn commit_follow_up_at(&mut self, index: usize) -> Option<String> {
        let Some(Composed::FollowUps { words, .. }) = &self.composed else {
            return None;
        };
        let word = words.get(index)?;
        Some(self.engine.commit_follow_up(word))
    }

    /// 只有明确选词的键写入联想词。其他键收起候选后仍按组句外的规则处理。
    pub(in crate::dispatch) fn apply_follow_up_key(&mut self, event: &KeyEvent) -> Option<Effect> {
        self.expire_follow_ups();
        if !self.has_follow_ups() {
            return None;
        }
        if event.modifiers == Default::default() {
            if self.config.commit_keys.candidate == CommitKey::SPACE
                && CommitKey::SPACE.matches(event)
            {
                // 上屏上一词的空格可能仍按着，只有重新按下才确认联想词。
                return Some(if event.repeat {
                    Effect::Navigated
                } else {
                    Effect::Changed(self.commit_follow_up_at(0))
                });
            }
            if let Some(digit) = self.wubi_candidate_digit(event).or_else(|| {
                if self.config.numpad_digit == NumpadDigit::Direct
                    && key::is_keypad(event.virtual_key)
                {
                    None
                } else {
                    key::candidate_digit(event)
                }
            }) && let Some(text) = self.commit_follow_up_at(digit - 1)
            {
                return Some(Effect::Changed(Some(text)));
            }
        }
        self.hide_candidate_window();
        (event.virtual_key == ESCAPE).then_some(Effect::Navigated)
    }

    pub(super) fn follow_up_frame(&self, words: &[String]) -> Frame {
        Frame {
            candidates: CandidateList {
                items: words
                    .iter()
                    .map(|word| Candidate {
                        text: word.clone(),
                        kind: CandidateKind::Code,
                        syllables: Vec::new(),
                        reading: None,
                        translation: None,
                        aux_code: None,
                    })
                    .collect(),
            },
            highlight: 0,
            page_count: 1,
            layout: self.config.layout,
            appearance: self.config.appearance,
            ..Frame::default()
        }
    }
}
