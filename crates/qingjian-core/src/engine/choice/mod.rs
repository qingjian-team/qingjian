//! 选词模型重排（试验）：候选窗前几条里盖住整段输入的，交给选词模型（知意）比一次，挑中的挪到它们的第一位。
//!
//! 规则照知意输入法引擎的 laya 重排：只看前 18 条，可比的至少两条才比；分是模型 log softmax、二元 LM 的 ln P
//! 与位置先验之和。和整句重打分走同一套停顿后请求的流程（[`Engine::request_rescoring`] / [`Engine::poll_rescoring`]）：
//! 查询时没分就记下这页，壳停顿后送去后台，分回来再查一次就排好了。

mod char_table;
mod compose;
mod finished;
mod job;
mod model;
mod ranker;
mod request;
mod score;
mod tier;
mod worker;

use std::sync::Arc;

use super::input_log::ChoiceNote;
use super::{Engine, take_last_chars};
use crate::sentence::{LanguageModel, segment_text};
use crate::{Candidate, CandidateKind};

pub use char_table::CharTable;
pub(crate) use compose::ComposeRequest;
use finished::Finished;
use job::Job;
pub use model::ChoiceModel;
pub(crate) use ranker::ChoiceRanker;
pub(crate) use request::ChoiceRequest;
pub use score::ChoiceScore;
pub use tier::ChoiceTier;

/// 最多看前这么多条候选（知意 `laya` 的 2 × 每页 9 条），各档在这以内取。
const WINDOW: usize = 18;

/// 至少这么多条可比的才重排。
const MIN_CANDIDATES: usize = 2;

/// 二元 LM 分的权重（知意 `laya.ngram_weight`，它配的是自己的三元模型）。
const LM_WEIGHT: f64 = 1.0;

/// 短语级语言模型分的权重（知意 `laya.ngram_weight`，配的就是这个三元模型）。
const PHRASE_WEIGHT: f64 = 1.0;

/// 中文位置先验的权重：新日志三档里 1.0 最好（词 89.6% → 89.9%）。
const RANK_WEIGHT: f64 = 1.0;

/// 整句候选的语言模型分与位置先验再乘这个数：整句的先后本来就是词图按语言模型排的，两项照算等于把同一个判断数了三遍，
/// 知意对、语言模型错的那些句子（维基 / 危机）被压回去。取值见 `docs/notes/crate-notes.md`。
const SENTENCE_TERMS_SCALE: f64 = 0.5;

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

/// 整句词图里字分布得分的权重。
const EMISSION_WEIGHT: f64 = 0.5;

/// 字分布里一个字的最低分：字表外的字、概率极小的字都按它算，免得一个字把整条路径拖死。
const EMISSION_FLOOR: f64 = -12.0;

/// 读音被敲错边、模糊音改过的字固定扣的分（乘权重前）。
const ALTERED_EMISSION: f64 = -4.0;

/// 语言模型不认识的字（同文本切词）。
const UNKNOWN_LOG_PROB: f64 = -18.5;

impl Engine {
    /// 接上或卸掉选词模型（后台线程随之起停）。
    pub fn set_choice_model(&mut self, model: Option<Arc<dyn ChoiceModel>>) {
        let wake = self.wake.clone();
        self.choice = model.map(|model| ChoiceRanker::new(model, wake));
    }

    /// 接上或卸掉短语级语言模型：选词重排里整段候选的语言模型分改由它给（`PHRASE_WEIGHT` 加权）。
    pub fn set_phrase_model(&mut self, model: Option<Arc<dyn crate::sentence::PhraseModel>>) {
        self.phrase_model = model;
    }

    /// 换档，下一次查询就按新档比。
    pub fn set_choice_tier(&mut self, tier: ChoiceTier) {
        self.choice_tier = tier;
    }

    /// 最近一次查询里选词模型给这页各候选的分（没排过为空）：评测归因用。
    pub fn last_choice_scores(&self) -> Vec<ChoiceScore> {
        self.last_choice_scores.borrow().clone()
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
        // 敲错边读出的备选不比：知意把 `suran` 的 虽然 当成 `su` 是 `sui` 的简拼对齐上，重排里的语言模型分与位置先验
        // 只看词常不常用，会把它顶到用户敲对的 肃然 前面（上屏 → 商品、这俩 → 这里 都是这么丢的）
        let corrected = self.corrected_alternates.borrow();
        for (index, candidate) in items.iter().enumerate().take(self.choice_tier.window()) {
            if corrected.contains(&candidate.text) {
                continue;
            }
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
        let phrase = (!english)
            .then_some(self.phrase_model.as_ref())
            .flatten()
            .map(|phrase| {
                let texts: Vec<&str> = request.texts.iter().map(String::as_str).collect();
                phrase.score(&request.context, &texts)
            })
            .filter(|scores| scores.len() == request.texts.len());
        let previous = (!english)
            .then(|| last_word(&request.context, model))
            .flatten();
        let scores: Vec<ChoiceScore> = (0..whole.len())
            .map(|k| {
                let rank = whole[k].0;
                let (language_model, prior) = if english {
                    (
                        0.0,
                        ENGLISH_RANK_WEIGHT * ENGLISH_RANK_LOG_PROB[rank.min(4)],
                    )
                } else {
                    let lm = match &phrase {
                        Some(scores) => PHRASE_WEIGHT * scores[k],
                        None => {
                            LM_WEIGHT * text_log_prob(previous.as_deref(), &request.texts[k], model)
                        }
                    };
                    let scale = if items[rank].kind == CandidateKind::Sentence {
                        SENTENCE_TERMS_SCALE
                    } else {
                        1.0
                    };
                    (
                        scale * lm,
                        scale * RANK_WEIGHT * CHINESE_RANK_LOG_PROB[rank.min(WINDOW - 1)],
                    )
                };
                ChoiceScore {
                    text: request.texts[k].clone(),
                    rank,
                    model: log_probs[k],
                    language_model,
                    prior,
                }
            })
            .collect();
        let total = |s: &ChoiceScore| s.model + s.language_model + s.prior;
        let best = (0..scores.len())
            .max_by(|&a, &b| total(&scores[a]).total_cmp(&total(&scores[b])))
            .unwrap_or(0);
        *self.last_choice_scores.borrow_mut() = scores;
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

    /// 这段音节的字分布（选词模型按音节出的），整句词图给路径上的字打分用。算好了就给；没算过就记下来，
    /// 等壳停顿后和选词打分一起送去后台（[`Self::request_choice`]），这次先不用。
    pub(in crate::engine) fn char_table(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
    ) -> Option<Arc<CharTable>> {
        let choice = self.choice.as_ref()?;
        let limit = self.choice_tier.context_chars(false);
        let context = match &self.rescoring_before {
            Some(before) => take_last_chars(before, limit),
            None => self.history.recent(limit).to_owned(),
        };
        let key = compose_key(&context, patterns);
        if let Some(table) = choice.table(&key) {
            return table;
        }
        let mut wanted = choice.compose_wanted.borrow_mut();
        if !wanted.iter().any(|request| request.key == key) {
            wanted.push(ComposeRequest {
                key,
                context,
                syllables: patterns
                    .iter()
                    .map(|p| (p.text.to_owned(), p.complete))
                    .collect(),
            });
        }
        None
    }

    /// 查询开头：上一次查询记下、还没送出的出字请求作废，只留这次要的。逐键打字时每个前缀都会记一条，
    /// 不清的话停键后几十个过时前缀排队先算，整句那张表要等好几百毫秒（空格早就上屏了）。
    pub(super) fn forget_stale_compose(&self) {
        if let Some(choice) = &self.choice {
            choice.compose_wanted.borrow_mut().clear();
        }
    }

    /// 有一页等着选词模型打分，或有输入等着出字。
    pub(super) fn choice_pending(&self) -> bool {
        self.choice.as_ref().is_some_and(|choice| {
            choice.wanted.borrow().is_some() || !choice.compose_wanted.borrow().is_empty()
        })
    }

    /// 把等着出字的输入、或等着的那页送去后台；都没有返回 `false`。有字要出时只送出字：它回来后词图会变、
    /// 这一页也跟着变，先给旧页打分是白算，还让出字晚几十毫秒；壳重查后新的一页再来一轮。
    pub(super) fn request_choice(&self) -> bool {
        let Some(choice) = &self.choice else {
            return false;
        };
        let compose: Vec<ComposeRequest> = choice.compose_wanted.borrow_mut().drain(..).collect();
        if !compose.is_empty() {
            choice.wanted.borrow_mut().take();
            choice.worker.submit(Job::Compose(compose));
            return true;
        }
        let Some(request) = choice.wanted.borrow_mut().take() else {
            return false;
        };
        choice.worker.submit(Job::Score(request));
        true
    }

    /// 收后台打好的分与字分布；最近一次请求的到了返回 `true`。
    pub(super) fn poll_choice(&self) -> bool {
        let Some(choice) = &self.choice else {
            return false;
        };
        let latest = choice.worker.latest();
        let mut updated = false;
        while let Some((serial, finished)) = choice.worker.poll() {
            match finished {
                Finished::Scored {
                    request,
                    logits,
                    ms,
                } => {
                    if logits.len() != request.texts.len() {
                        continue;
                    }
                    *choice.scored.borrow_mut() = Some((request, logits, ms));
                }
                Finished::Composed(tables) => {
                    for (key, table) in tables {
                        choice.insert_table(key, table);
                    }
                }
            }
            updated |= serial == latest;
        }
        updated
    }
}

/// 词 `text`（从第 `start` 个音节起，读音 `syllables`）在字分布里的得分，乘 [`EMISSION_WEIGHT`]：
/// 读音与敲的一致的字按「比这个位置最好的字差多少」算（下限 [`EMISSION_FLOOR`]）；敲错边、模糊音改过读音的字
/// 字分布是按敲的音节出的、说明不了它，固定扣 [`ALTERED_EMISSION`]，取舍交给原来的纠错代价。
pub(in crate::engine) fn char_emission(
    table: &CharTable,
    patterns: &[qingjian_dictionary::SyllablePattern<'_>],
    start: usize,
    text: &str,
    syllables: &[String],
) -> f64 {
    text.chars()
        .enumerate()
        .map(|(offset, ch)| {
            let slot = start + offset;
            let as_typed = match (patterns.get(slot), syllables.get(offset)) {
                (Some(typed), Some(reading)) => {
                    if typed.complete {
                        reading == typed.text
                    } else {
                        reading.starts_with(typed.text)
                    }
                }
                _ => false,
            };
            if !as_typed {
                return ALTERED_EMISSION;
            }
            table
                .relative(slot, ch)
                .map_or(EMISSION_FLOOR, f64::from)
                .max(EMISSION_FLOOR)
        })
        .sum::<f64>()
        * EMISSION_WEIGHT
}

/// 字分布的缓存键：前文与音节串（敲全的音节后跟 `|`，没敲全的跟 `~`）。
fn compose_key(context: &str, patterns: &[qingjian_dictionary::SyllablePattern<'_>]) -> String {
    let mut key = String::with_capacity(context.len() + 8 * patterns.len());
    key.push_str(context);
    key.push('\x01');
    for pattern in patterns {
        key.push_str(pattern.text);
        key.push(if pattern.complete { '|' } else { '~' });
    }
    key
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
