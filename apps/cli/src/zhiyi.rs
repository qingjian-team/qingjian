//! 试验：用知意的选词模型重排候选（`--zhiyi`），回放与整句评测共用，与现在的排序对比。
//! 规则照知意输入法引擎（`laya_rerank.h` 与 `config.h` 的 laya 一节）：只看前 2 × 每页候选数个，
//! 其中盖住整段输入的至少两个才比；分 = 模型 log 概率 + n-gram 权重 × ln P(候选 | 上文)，
//! 选中的挪到这些候选的第一个位置，其余（含只盖住一部分的）不动。n-gram 换成我们自己的二元语言模型。

use std::time::{Duration, Instant};

use qingjian_core::sentence::{LanguageModel, segment_text};
use qingjian_core::{CandidateKind, Engine, Query};
use qingjian_neural::NeuralError;
use qingjian_neural::zhiyi::ChoiceScorer;

/// 上文最多留这么多字（知意 `laya.context_chars`）。
const CONTEXT_CHARS: usize = 128;

/// 英文上文多留一些（`laya.english_context_chars`）。
const ENGLISH_CONTEXT_CHARS: usize = 192;

/// 至少这么多个可比的候选才重排（`laya.min_candidates`）。
const MIN_CANDIDATES: usize = 2;

/// 英文位置先验的权重（`laya.english_rank_prior_weight`）：分 += 权重 × ln P(要的词在这个位置)。
const ENGLISH_RANK_PRIOR_WEIGHT: f64 = 0.2;

/// 英文候选里要的词在第几位的 ln P：冻结日志（2026-09-12）269 次英文上屏选中位置的分布（第 1 位 241 次、
/// 第 2、3 位各 13 次、第 4 位 2 次），加一平滑；评测用的是之后的日志。
const ENGLISH_RANK_LOG_PROB: [f64; 5] = [-0.171, -3.02, -3.02, -4.561, -5.659];

/// 中文候选的位置先验：冻结日志（2026-09-12）1.26 万次词与整句上屏选中位置的 ln P（第 1 位 93%），加一平滑。
/// 我们的顺序里带着个人学习，模型看不到；知意的 `laya.rank_prior_weight` 是 0，这里的权重由 `--zhiyi-rank-weight` 给。
const CHINESE_RANK_LOG_PROB: [f64; 18] = [
    -0.071, -3.443, -4.197, -4.944, -5.683, -6.077, -6.554, -6.611, -6.736, -6.736, -7.652, -7.498,
    -7.365, -7.835, -8.346, -7.835, -8.346, -8.346,
];

/// 语言模型不认识的单字（同 Core 的文本切词）。
const UNKNOWN_LOG_PROB: f64 = -18.5;

pub struct Reranker {
    scorer: ChoiceScorer,

    /// 看前这么多个候选。
    window: usize,

    /// 语言模型分的权重（知意 `laya.ngram_weight` 是 1.0，配的是它自己的三元模型）。
    lm_weight: f64,

    /// 中文位置先验的权重。
    rank_weight: f64,

    /// 光标前的文字：回放里是上屏与直通的字符接起来，上文断开时清空；整句评测里是这句的上文。
    context: String,

    pub spent: Duration,
    pub calls: usize,
    pub reordered: usize,
}

impl Reranker {
    pub fn load(
        dir: &std::path::Path,
        window: usize,
        (lm_weight, rank_weight): (f64, f64),
    ) -> Result<Self, NeuralError> {
        Ok(Self {
            scorer: ChoiceScorer::load(dir)?,
            window,
            lm_weight,
            rank_weight,
            context: String::new(),
            spent: Duration::ZERO,
            calls: 0,
            reordered: 0,
        })
    }

    pub fn note(&mut self, text: &str) {
        self.context.push_str(text);
        let extra = self
            .context
            .chars()
            .count()
            .saturating_sub(ENGLISH_CONTEXT_CHARS);
        if extra > 0 {
            let cut = self.context.char_indices().nth(extra).map_or(0, |(i, _)| i);
            self.context.drain(..cut);
        }
    }

    pub fn set_context(&mut self, text: &str) {
        self.context.clear();
        self.note(text);
    }

    pub fn reset(&mut self) {
        self.context.clear();
    }

    pub fn summary(&self) -> String {
        format!(
            "知意重排：比了 {} 次，改了首位 {} 次，平均 {:.1} ms",
            self.calls,
            self.reordered,
            self.spent.as_secs_f64() * 1000.0 / self.calls.max(1) as f64
        )
    }

    /// 按知意的规则重排；打分失败就不动。
    pub fn rerank(&mut self, engine: &Engine, query: &mut Query, scope: &str) {
        let items = &mut query.candidates.items;
        let keys: String = scope.chars().filter(|c| *c != '\'').collect();
        // 中英分开排：英文模式只比英文补全，和敲的一样的那个词留在原位（知意：敲的词在英文模式里总在第一）；
        // 中文模式只比中文，夹在里面的英文词、emoji 原地不动
        let english = engine.english_mode();
        let mut whole: Vec<(usize, Vec<u32>)> = Vec::new();
        for (index, candidate) in items.iter().enumerate().take(self.window) {
            let units = match candidate.kind {
                CandidateKind::English
                    if english && !candidate.text.eq_ignore_ascii_case(&keys) =>
                {
                    Some(self.scorer.letter_units(&keys))
                }
                CandidateKind::Chinese | CandidateKind::Sentence if !english => {
                    self.scorer.align(&keys, &candidate.syllables)
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
        let limit = if english {
            ENGLISH_CONTEXT_CHARS
        } else {
            CONTEXT_CHARS
        };
        let skip = self.context.chars().count().saturating_sub(limit);
        let context: String = self.context.chars().skip(skip).collect();
        let texts: Vec<&str> = whole.iter().map(|(i, _)| items[*i].text.as_str()).collect();
        let started = Instant::now();
        let logits = match self.scorer.score(&context, &whole[0].1, &texts) {
            Ok(logits) => logits,
            Err(error) => {
                tracing::warn!(%error, "知意模型打分失败");
                return;
            }
        };
        self.spent += started.elapsed();
        self.calls += 1;
        let log_probs = log_softmax(&logits);
        let previous = (!english)
            .then(|| last_word(&context, engine.language_model()))
            .flatten();
        let best = (0..whole.len())
            .map(|k| {
                let rank = whole[k].0;
                let prior = if english {
                    ENGLISH_RANK_PRIOR_WEIGHT * ENGLISH_RANK_LOG_PROB[rank.min(4)]
                } else {
                    self.lm_weight
                        * text_log_prob(previous.as_deref(), texts[k], engine.language_model())
                        + self.rank_weight * CHINESE_RANK_LOG_PROB[rank.min(17)]
                };
                (k, log_probs[k] + prior)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
            .unwrap_or(0);
        if best != 0 {
            self.reordered += 1;
            let candidate = items.remove(whole[best].0);
            items.insert(whole[0].0, candidate);
        }
    }
}

fn log_softmax(logits: &[f32]) -> Vec<f64> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    let sum: f64 = logits.iter().map(|&l| (l as f64 - max).exp()).sum();
    logits.iter().map(|&l| l as f64 - max - sum.ln()).collect()
}

/// 上文最后一个词（按语言模型切词，标点处断句：上文以标点结尾就当句首）。
fn last_word(context: &str, model: &dyn LanguageModel) -> Option<String> {
    let last = context.chars().last()?;
    if !is_han(last) {
        return None;
    }
    segment_text(context, model)?.pop()?.pop()
}

/// ln P(文本 | 上一个词)：按语言模型切词后逐词累加，模型不认识的按罕见单字算。
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

fn is_han(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c)
}
