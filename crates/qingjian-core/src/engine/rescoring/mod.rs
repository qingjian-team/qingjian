//! 神经重打分：整句转换的前几条路径交给字级模型（[`SentenceScorer`]）再排一次。
//!
//! 打分有两种接法：同步的（[`Engine::with_sentence_scorer`]，查询里当场打，CLI 评测用）和异步的
//! （[`Engine::with_async_sentence_scorer`]，后台线程；壳里用）。两种都经过一张「前文 + 文本 → 神经分」的缓存
//! （[`NeuralCache`]）：同步时缺的分当场补进去，异步时缺的先记下来，壳在用户停顿后调 [`Engine::request_rescoring`]
//! 一次送去后台，[`Engine::poll_rescoring`] 收到结果后再查一次，这时全部路径的分都在缓存里，排序自然换成重排后的。
//! 按键回调永远不等模型：先按词级模型出候选，模型的意见晚几十毫秒到。

mod cache;
mod worker;

#[cfg(test)]
mod tests;

use super::*;

pub(crate) use cache::NeuralCache;
pub(crate) use worker::RescoreWorker;

/// 直接生成整句时的 beam 宽度。5 是 2026-09-24 在冻结集上量的：beam 5 首选 51.9%、前五 73.5%，
/// 再宽只换来零点几个点，延迟却线性涨。
const GENERATE_BEAM: usize = 5;

/// 生成长度上限（字）。超过这个长度的输入用户早就分段上屏了，放开只会让最坏情况的延迟没有上界。
const GENERATE_MAX_CHARS: usize = 32;

/// 生成出来的整句最多取几条当候选。
const GENERATED_CANDIDATES: usize = 2;

impl Engine {
    /// 接了重打分器（同步或异步）。
    pub fn has_sentence_scorer(&self) -> bool {
        self.sentence_scorer.is_some()
            || self.rescorer.as_ref().is_some_and(RescoreWorker::is_alive)
    }

    /// 给模型看的前文：壳给了应用里的光标前文就用它（[`Self::set_rescoring_context`]），
    /// 否则用本会话最近上屏的字符；长度按 `neural_context` 截。
    pub(super) fn rescoring_context(&self) -> String {
        if self.neural_context == 0 {
            return String::new();
        }
        match &self.rescoring_before {
            Some(before) => take_last_chars(before, self.neural_context),
            None => self.history.recent(self.neural_context).to_owned(),
        }
    }

    /// 壳告知应用里光标前的文本（每次查询前给；应用给不出就 `None`，退回本会话历史）。
    pub fn set_rescoring_context(&mut self, before: Option<String>) {
        self.rescoring_before = before;
    }

    /// 把几条整句路径按「路径分 + λ·(神经分 − 静态分)」重排。缓存里缺分的：同步打分器当场补，异步的先记下等壳来取；
    /// 有任何一条没分就不动顺序（半截重排比不重排还糟）。
    /// `keys` 是这批路径共同解释的那段用户按键（P2C 的条件）。
    /// 只改顺序，每条路径的 `score` 原样不动（理由见下面的注释）。
    pub(super) fn rescore_paths(&self, paths: &mut [Conversion], keys: &str) {
        if paths.len() < 2 || !self.has_sentence_scorer() {
            return;
        }
        let context = self.rescoring_context();
        let mut cache = self.neural_cache.borrow_mut();
        cache.ensure_condition(&context, keys);
        let mut missing: Vec<String> = Vec::new();
        for path in paths.iter() {
            if cache.get(&path.text).is_none() && !missing.contains(&path.text) {
                missing.push(path.text.clone());
            }
        }
        if !missing.is_empty() {
            match &self.sentence_scorer {
                Some(scorer) => {
                    let texts: Vec<&str> = missing.iter().map(String::as_str).collect();
                    let scores = scorer.score(&context, keys, &texts);
                    if scores.len() != texts.len() {
                        return;
                    }
                    for (text, score) in texts.iter().zip(scores) {
                        cache.insert(text, score);
                    }
                }
                None => {
                    for text in &missing {
                        cache.want(text);
                    }
                    return;
                }
            }
        }
        // 神经分只决定名次，不写回 `score`：P2C 打的是 log P(汉字 | 拼音)，被拼音条件住，
        // 比静态分高十几二十 nat，抬升还随句子长短与静态模型的偏好变化。写回去会污染跨读法的比较
        // （拼写纠错的原样 vs 纠正、混输的头段 vs 整段），那两处的门槛都是按静态尺度定的：
        // `woxiangxuexirust` 就是这么被「纠正」成 我想学习如斯 的。
        let lambda = self.neural_weight;
        let rescored = |path: &Conversion| {
            let neural = cache.get(&path.text).expect("filled above");
            path.score + lambda * (neural - path.static_score)
        };
        paths.sort_by(|a, b| {
            rescored(b)
                .partial_cmp(&rescored(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.last_rescored.set(true);
    }

    /// 不经词图，让模型直接按整段按键生成整句，最好的在前。
    ///
    /// 词图只会把按键读成拼音，中英混输（`yongdockerbushuhenfangbian`）与生词在它那里没有路径，
    /// 出来的只能是把英文段硬读成拼音的结果（用的哦乘客仍不熟很方便）。这条路不受读法限制。
    /// 同步打分器当场生成，异步的先记下、等壳在用户停顿后取（与重打分同一次请求）。
    pub(super) fn generated_sentences(&self, keys: &str) -> Vec<String> {
        if keys.is_empty() || !self.has_sentence_scorer() {
            return Vec::new();
        }
        let mut cache = self.neural_cache.borrow_mut();
        if let Some(texts) = cache.generated(keys) {
            return texts.iter().take(GENERATED_CANDIDATES).cloned().collect();
        }
        let Some(scorer) = &self.sentence_scorer else {
            cache.want_generation(keys);
            return Vec::new();
        };
        let texts = scorer.generate(keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
        let out = texts.iter().take(GENERATED_CANDIDATES).cloned().collect();
        cache.insert_generated(keys, texts);
        out
    }

    /// 最近一次查询里有整句路径还没拿到神经分、或有整段还没生成：壳该在用户停顿后调 [`Self::request_rescoring`]。
    pub fn rescoring_pending(&self) -> bool {
        self.rescorer.is_some() && {
            let cache = self.neural_cache.borrow();
            cache.has_wanted() || cache.wanted_generation().is_some()
        }
    }

    /// 把攒着的文本送去后台打分。没接异步打分器或没什么要打的返回 `false`。
    pub fn request_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut cache = self.neural_cache.borrow_mut();
        let wanted = cache.take_wanted();
        let generate = cache.take_wanted_generation();
        if wanted.is_empty() && generate.is_none() {
            return false;
        }
        tracing::debug!(
            texts = wanted.len(),
            generate = generate.is_some(),
            "神经请求"
        );
        worker.submit(
            cache.context().to_owned(),
            cache.keys().to_owned(),
            wanted,
            generate,
        );
        true
    }

    /// 收后台打好的分。有新分进了缓存返回 `true`，壳该重新 [`Self::query`] 一次；前文已经变了的结果丢掉。
    pub fn poll_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut updated = false;
        while let Some(scored) = worker.poll() {
            let mut cache = self.neural_cache.borrow_mut();
            // 生成的结果自带按键、与前文无关，不跟着打分那边的条件一起作废
            if let Some((keys, texts)) = scored.generated {
                cache.insert_generated(&keys, texts);
                updated = true;
            }
            if scored.context != cache.context()
                || scored.keys != cache.keys()
                || scored.scores.len() != scored.texts.len()
            {
                continue;
            }
            for (text, score) in scored.texts.iter().zip(scored.scores) {
                cache.insert(text, score);
            }
            updated = true;
        }
        updated
    }
}
