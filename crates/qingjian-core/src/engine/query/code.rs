//! 形码查询：编码按前缀查码表，没有切分、没有整句；以及与拼音的混输。

use std::collections::HashSet;

use super::*;

impl Engine {
    /// 形码方案（五笔）的候选：编码打全的词排在同前缀的更长编码词前面，其余按词频与上下文。
    ///
    /// 拼音那一套在这里都不成立：编码不需要切分，没有简拼与模糊音，一处编辑的拼写纠错更不适用，
    /// 整句转换、中英混输与神经重排也没有可以展开的东西。反过来，译词标注、生词记录、输入日志、
    /// 用户选择学习与个人 n-gram 都按上屏的词工作，与拼音方案共用同一条路。
    pub(super) fn query_code(&self, keys: &str, rest: String, start: Instant) -> Query {
        self.query_code_counted(keys, rest, start).0
    }

    /// 同 [`Self::query_code`]，另带回排在最前面的「编码打全」的候选有几条（混输按它分两段）。
    fn query_code_counted(&self, keys: &str, rest: String, start: Instant) -> (Query, usize) {
        let table = self.code.as_ref().expect("只在形码方案下调用");
        // 编码以外的字符（`no-way` 的 `-`）不是编码，交给原样上屏那条路
        if !keys.chars().all(|c| c.is_ascii_lowercase()) {
            return (self.query_raw(keys, rest, start), 0);
        }
        let parse = start.elapsed();

        let start = Instant::now();
        let log_total = (table.total_frequency() as f64).max(1.0).ln();
        let mut scored: Vec<Scored<'_>> = table
            .lookup(keys, MAX_CANDIDATES)
            .into_iter()
            .map(|hit| Scored {
                hit,
                // 编码是前缀匹配：没有「最后一个音节打完了」这回事，也不吃简拼、模糊音与敲错
                full_last: true,
                coverage: keys.len(),
                abbreviated: 0,
                weight: self.learner.weight(hit.text),
                penalty: 0.0,
            })
            .collect();
        let lookup = start.elapsed();

        // 同一段编码下选过的词优先，再按上下文得分（上一个上屏的词）；词频只在模型不认识时兜底
        let start = Instant::now();
        let letters = choice_key(keys, keys.len());
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let choice = letters
                .get(..item.coverage)
                .map_or(0, |input| self.learner.choice_weight(input, item.hit.text));
            let log_prob = sentence::transition_log_prob(
                &*self.language_model,
                self.personal(),
                self.chain.context(),
                item.hit.text,
                sentence::fallback_log_prob(item.hit.frequency, log_total),
            );
            (choice, log_prob)
        });
        let rank = start.elapsed();
        // `exact` 是排序的最高位，打全的都在最前面
        let exact = scored.iter().take_while(|s| s.hit.exact).count();
        let items: Vec<Candidate> = scored
            .into_iter()
            .map(|s| Candidate {
                text: s.hit.text.to_owned(),
                kind: CandidateKind::Code,
                // 编码不是拼音音节：候选窗按音节高亮的部分对形码没有意义，留空
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        let query = Query {
            // 形码没有切分：preedit 的显示串靠 `tail` 原样带出去（见 `Query::marked_text`）
            segmentations: Vec::new(),
            candidates: CandidateList { items },
            tail: keys.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: false,
            shuangpin_raw_preedit: false,
            typed_display: None,
            correction: None,
            aux: None,
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        };
        (query, exact)
    }

    /// 混输：形码与拼音两边都出候选。**编码打全的形码词在最前，其次拼音，只命中前缀的形码词垫后。**
    ///
    /// 打全的编码是精确的（`ga` 就是 开）；只敲了前缀的形码词一律放前面的话，`kai` 的首选会变成
    /// 编码 `kaik` 的 中共党员，拼音就没法用了。拼音那条路给不出解析时（`ggll` 切不成音节）不算失败，
    /// 整个查询按形码的结果走；五笔码最长 4 位，第 5 个字母起形码查不到东西，自然只剩拼音。
    pub(super) fn query_mixed(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        let (code, exact) = self.query_code_counted(keys, rest.clone(), start);
        let mut query = match self.query_phonetic(keys, rest, start) {
            Ok(query) => query,
            Err(error) => {
                // 拼音读不出来，但形码有东西：把形码那条留着，出错只在两边都空时才算
                if code.candidates.items.is_empty() {
                    return Err(error);
                }
                return Ok(code);
            }
        };
        let mut prefixed = code.candidates.items;
        let exact = exact.min(prefixed.len());
        let tail = prefixed.split_off(exact);
        // 同一个词可能两边都命中，按文本去重，靠前的那条留着
        let mut seen: HashSet<String> = HashSet::new();
        let combined: Vec<Candidate> = prefixed
            .into_iter()
            .chain(std::mem::take(&mut query.candidates.items))
            .chain(tail)
            .filter(|candidate| seen.insert(candidate.text.clone()))
            .take(MAX_CANDIDATES)
            .collect();
        query.candidates.items = combined;
        Ok(query)
    }
}
