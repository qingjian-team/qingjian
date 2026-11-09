//! 云联想与整句补全：组句时按停键发请求，结果插在本地首选之后（先整句补全，再云端词），选中整句走 `accept_prediction`。
//! 光标前后文由壳在每段组句开始时交一次（[`Session::set_surrounding`]）；工具框（快捷翻译、常用语编辑）开着时不发。

use qingjian_core::{CandidateList, CloudWord, SurroundingText};

use super::Session;
use crate::frame::Frame;

impl Session {
    /// 这段组句的光标前后文（应用输入框里的），发联想时由 Engine 按观察窗口裁剪；组句结束后作废。
    /// 壳在组句第一帧之后才交（第一键的请求已经发了），所以交了就按带上下文的再要一次，worker 只发最新的。
    pub fn set_surrounding(&mut self, before: String, after: String) {
        self.surrounding = Some(SurroundingText { before, after });
        if !self.engine.composition().is_empty() {
            self.request_prediction();
        }
    }

    /// 组句刷新完：开着自动联想就按新拼音再要一次（停键防抖在 worker 里），旧的云端结果随新查询作废。
    /// 没开自动联想时什么都不做：`request_prediction` 会顺手取消在飞的请求，快捷翻译的译文也在里面。
    pub(super) fn request_prediction(&mut self) {
        self.sentence_at = None;
        if self.tools_open
            || !self.engine.prediction_enabled()
            || !self.engine.prediction_policy().automatic
        {
            return;
        }
        if self.engine.composition().is_empty() {
            self.engine.cancel_composition_prediction();
            self.surrounding = None;
            return;
        }
        self.engine
            .request_prediction(self.surrounding.clone(), &self.candidates.items);
    }

    /// 取回云联想的结果插进候选：整句补全在本地首选之后，云端词再其后，与本地重复的不要。
    /// 没有新结果、已不在组句或结果为空返回 `None`，壳不用重画。
    pub fn poll_prediction(&mut self) -> Option<Frame> {
        if self.tools_open || !self.engine.prediction_enabled() {
            return None;
        }
        let prediction = self.engine.poll_prediction()?;
        if self.engine.composition().is_empty() || self.kana.is_some() {
            return None;
        }
        let local = &self.candidates.items;
        let fresh = |text: &str| !local.iter().any(|candidate| candidate.text == text);
        let sentence = prediction
            .sentence
            .filter(|text| !text.is_empty() && fresh(text) && self.sentence_at.is_none());
        let mut words = CandidateList {
            items: prediction
                .words
                .into_iter()
                .filter(|word| fresh(&word.text) && sentence.as_deref() != Some(&word.text))
                .map(CloudWord::into_candidate)
                .collect(),
        };
        if sentence.is_none() && words.items.is_empty() {
            return None;
        }
        self.engine.annotate(&mut words);
        let mut at = usize::from(!self.candidates.items.is_empty());
        if let Some(text) = sentence {
            let candidate = CloudWord {
                text,
                ..CloudWord::default()
            }
            .into_candidate();
            self.candidates.items.insert(at, candidate);
            self.sentence_at = Some(at);
            at += 1;
        }
        for word in words.items {
            self.candidates.items.insert(at, word);
            at += 1;
        }
        Some(self.frame(String::new()))
    }

    /// 选中的是插进来的整句补全：整段拼音换成这句上屏，返回上屏的文字；不是返回 `None`。
    pub(super) fn select_sentence(&mut self, index: usize) -> Option<String> {
        if self.sentence_at != Some(index) {
            return None;
        }
        let text = self.candidates.items.get(index)?.text.clone();
        self.sentence_at = None;
        Some(self.engine.accept_prediction(&text))
    }
}
