//! 选词模型重排（试验）：候选窗前几条里盖住整段输入的，交给选词模型（知意）比一次，挑中的挪到它们的第一位。
//!
//! 规则照知意输入法引擎的 laya 重排：只看前 18 条，可比的至少两条才比；分是模型 log softmax、二元 LM 的 ln P
//! 与位置先验之和。和整句重打分走同一套停顿后请求的流程（[`Engine::request_rescoring`] / [`Engine::poll_rescoring`]）：
//! 查询时没分就记下这页，壳停顿后送去后台，分回来再查一次就排好了。

mod model;
mod ranker;
mod request;
mod tier;
mod worker;

use std::sync::Arc;

use super::input_log::ChoiceNote;
use super::{Engine, take_last_chars};
use crate::sentence::{LanguageModel, segment_text};
use crate::{Candidate, CandidateKind};

pub use model::ChoiceModel;
pub(crate) use ranker::ChoiceRanker;
pub(crate) use request::ChoiceRequest;
pub use tier::ChoiceTier;

/// 最多看前这么多条候选（知意 `laya` 的 2 × 每页 9 条），各档在这以内取。
const WINDOW: usize = 18;

/// 至少这么多条可比的才重排。
const MIN_CANDIDATES: usize = 2;

/// 二元 LM 分的权重（知意 `laya.ngram_weight`，它配的是自己的三元模型）。
const LM_WEIGHT: f64 = 1.0;

/// 中文位置先验的权重：新日志三档里 1.0 最好（词 89.6% → 89.9%）。
const RANK_WEIGHT: f64 = 1.0;

/// 英文位置先验的权重（`laya.english_rank_prior_weight`）。
const ENGLISH_RANK_WEIGHT: f64 = 0.2;

/// 中文候选里要的词在第几位的 ln P：冻结日志（2026-09-12）1.26 万次词与整句上屏选中位置的分布，加一平滑。
/// 我们的顺序里带着个人学习，模型看不到，靠它把原来的名次带进来。
const CHINESE_RANK_LOG_PROB: [f64; WINDOW] = [
    -0.071, -3.443, -4.197, -4.944, -5.683, -6.077, -6.554, -6.611, -6.736, -6.736, -7.652, -7.498,
    -7.365, -7.835, -8.346, -7.835, -8.346, -8.346,
];

/// 英文候选同上：冻结日志 269 次英文上屏。
const ENGLISH_RANK_LOG_PROB: [f64; 5] = [-0.171, -3.02, -3.02, -4.561, -5.659];

/// 语言模型不认识的字（同文本切词）。
const UNKNOWN_LOG_PROB: f64 = -18.5;

impl Engine {
    /// 接上或卸掉选词模型（后台线程随之起停）。
    pub fn set_choice_model(&mut self, model: Option<Arc<dyn ChoiceModel>>) {
        self.choice = model.map(ChoiceRanker::new);
    }

    /// 换档，下一次查询就按新档比。
    pub fn set_choice_tier(&mut self, tier: ChoiceTier) {
        self.choice_tier = tier;
    }

    pub fn has_choice_model(&self) -> bool {
        self.choice.is_some()
    }

    /// 查询的最后一步：这页有分就按模型重排，没分记下等壳来取。
    pub(super) fn rerank_choices(&self, items: &mut Vec<Candidate>) {
        let Some(choice) = &self.choice else {
            return;
        };
        let keys: String = self
            .composition
            .scope()
            .chars()
            .filter(|c| *c != '\'')
            .collect();
        let english = self.english_mode;
        // 英文模式只比英文补全，和敲的一样的那个词留在原位；中文模式只比中文，夹在里面的英文词、emoji 不动
        let mut whole: Vec<(usize, Vec<u32>)> = Vec::new();
        for (index, candidate) in items.iter().enumerate().take(self.choice_tier.window()) {
            let units = match candidate.kind {
                CandidateKind::English
                    if english && !candidate.text.eq_ignore_ascii_case(&keys) =>
                {
                    Some(choice.model.letter_units(&keys))
                }
                CandidateKind::Chinese | CandidateKind::Sentence if !english => {
                    choice.model.align(&keys, &candidate.syllables)
                }
                _ => None,
            };
            if let Some(units) = units {
                whole.push((index, units));
            }
        }
        if whole.len() < MIN_CANDIDATES {
            return;
        }
        let limit = self.choice_tier.context_chars(english);
        let context = match &self.rescoring_before {
            Some(before) => take_last_chars(before, limit),
            None => self.history.recent(limit).to_owned(),
        };
        let request = ChoiceRequest {
            context,
            from_app: self.rescoring_before.is_some(),
            typed: whole[0].1.clone(),
            texts: whole.iter().map(|(i, _)| items[*i].text.clone()).collect(),
        };
        let Some((logits, ms)) = choice.cached(&request) else {
            *choice.wanted.borrow_mut() = Some(request);
            return;
        };
        if logits.len() != whole.len() {
            return;
        }
        let log_probs = log_softmax(&logits);
        let model = self.language_model();
        let previous = (!english)
            .then(|| last_word(&request.context, model))
            .flatten();
        let best = (0..whole.len())
            .map(|k| {
                let rank = whole[k].0;
                let prior = if english {
                    ENGLISH_RANK_WEIGHT * ENGLISH_RANK_LOG_PROB[rank.min(4)]
                } else {
                    LM_WEIGHT * text_log_prob(previous.as_deref(), &request.texts[k], model)
                        + RANK_WEIGHT * CHINESE_RANK_LOG_PROB[rank.min(WINDOW - 1)]
                };
                (k, log_probs[k] + prior)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(k, _)| k);
        self.last_rescored.set(true);
        self.last_choice.set(Some(ChoiceNote {
            from: whole[best].0 - whole[0].0,
            ctx: request.context.chars().count(),
            app: request.from_app,
            ms,
        }));
        if best != 0 {
            let candidate = items.remove(whole[best].0);
            items.insert(whole[0].0, candidate);
        }
    }

    /// 有一页等着选词模型打分。
    pub(super) fn choice_pending(&self) -> bool {
        self.choice
            .as_ref()
            .is_some_and(|choice| choice.wanted.borrow().is_some())
    }

    /// 把等着的那页送去后台；没有返回 `false`。
    pub(super) fn request_choice(&self) -> bool {
        let Some(choice) = &self.choice else {
            return false;
        };
        let Some(request) = choice.wanted.borrow_mut().take() else {
            return false;
        };
        choice.worker.submit(request);
        true
    }

    /// 收后台打好的分；最近一次请求的到了返回 `true`。
    pub(super) fn poll_choice(&self) -> bool {
        let Some(choice) = &self.choice else {
            return false;
        };
        let latest = choice.worker.latest();
        let mut updated = false;
        while let Some(chosen) = choice.worker.poll() {
            if chosen.logits.len() != chosen.request.texts.len() {
                continue;
            }
            updated |= chosen.serial == latest;
            *choice.scored.borrow_mut() = Some((chosen.request, chosen.logits, chosen.ms));
        }
        updated
    }
}

fn log_softmax(logits: &[f32]) -> Vec<f64> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    let sum: f64 = logits.iter().map(|&l| (l as f64 - max).exp()).sum();
    logits.iter().map(|&l| l as f64 - max - sum.ln()).collect()
}

/// 上文最后一个词（按语言模型切词）；上文不是以汉字结尾就当句首。
fn last_word(context: &str, model: &dyn LanguageModel) -> Option<String> {
    let last = context.chars().last()?;
    if !('\u{4e00}'..='\u{9fff}').contains(&last) {
        return None;
    }
    segment_text(context, model)?.pop()?.pop()
}

/// ln P(文本 | 上一个词)：按语言模型切词后逐词累加，不认识的按罕见单字算。
fn text_log_prob(previous: Option<&str>, text: &str, model: &dyn LanguageModel) -> f64 {
    let Some(clauses) = segment_text(text, model) else {
        return UNKNOWN_LOG_PROB * text.chars().count() as f64;
    };
    let mut previous = previous.map(str::to_owned);
    let mut total = 0.0;
    for words in clauses {
        for word in words {
            total += model
                .log_prob(previous.as_deref(), &word)
                .unwrap_or(UNKNOWN_LOG_PROB);
            previous = Some(word);
        }
    }
    total
}

#[cfg(test)]
mod tests;
