use foldhash::{HashMap, HashMapExt};

use super::{
    CONFIDENCE_K, Context, MAX_CONFIDENCE, MAX_USER_TRANSITIONS, SENTENCE_START, TRIGRAM_DISCOUNT,
    USER_LAMBDA,
};

/// 个人 n-gram：用户上屏过的词序列计数（二元 + 三元），随上屏在线更新，进整句转换与词级排序的打分。
///
/// 二元只存 (前词, 后词) 计数：每个上屏的词都带着前词（句首用 [`SENTENCE_START`]），
/// 所以一个词的出现次数就是以它为后词的计数之和，无须另存一元表。
/// 三元存 (前二词, 前词, 后词) 计数，句首词不记三元（与二元重复）。
/// 持久化格式是 TSV：三列 `前词\t后词\t次数` 是二元，四列 `前二词\t前词\t后词\t次数` 是三元；文件读写在 learning crate，这里只做解析与序列化。
#[derive(Debug, Default, Clone)]
pub struct UserNgram {
    /// 前词 → (后词 → 次数)。
    pairs: HashMap<String, HashMap<String, u32>>,

    /// 前词 → 以它为前词的总次数（`pairs[v]` 的和），算二元条件概率的分母。
    context_totals: HashMap<String, u32>,

    /// 词 → 出现次数（以它为后词的计数之和）。
    word_counts: HashMap<String, u32>,

    /// 所有二元计数之和（不含句首标记作为后词的情况，它不会出现在后词位置）。
    total: u64,

    /// 前二词 → (前词 → (后词 → 次数))。
    triples: HashMap<String, HashMap<String, HashMap<String, u32>>>,

    /// 前二词 → (前词 → 这对上文的总次数)，算三元条件概率的分母。
    triple_totals: HashMap<String, HashMap<String, u32>>,
}

impl UserNgram {
    /// 记一次转移。超过上限时把所有计数减半，忘掉久远的偏好。
    pub fn record(&mut self, context: Context<'_>, word: &str) {
        self.record_times(context, word, 1);
    }

    /// 一次记 `times` 份：用户自己点选的转移比整句路径里顺带的更可信，Engine 给它记双份。
    pub fn record_times(&mut self, context: Context<'_>, word: &str, times: u32) {
        if word.is_empty() || times == 0 {
            return;
        }
        let previous = context.previous.unwrap_or(SENTENCE_START);
        *self
            .pairs
            .entry(previous.to_owned())
            .or_default()
            .entry(word.to_owned())
            .or_default() += times;
        *self.context_totals.entry(previous.to_owned()).or_default() += times;
        *self.word_counts.entry(word.to_owned()).or_default() += times;
        self.total += u64::from(times);
        if let Some(previous) = context.previous {
            let earlier = context.earlier.unwrap_or(SENTENCE_START);
            *self
                .triples
                .entry(earlier.to_owned())
                .or_default()
                .entry(previous.to_owned())
                .or_default()
                .entry(word.to_owned())
                .or_default() += times;
            *self
                .triple_totals
                .entry(earlier.to_owned())
                .or_default()
                .entry(previous.to_owned())
                .or_default() += times;
        }
        if self.transition_count() > MAX_USER_TRANSITIONS {
            self.decay();
        }
    }

    /// 退回 `times` 份转移（用户撤销了刚才的上屏）：减到零就删掉，没记过的不动。
    pub fn unrecord(&mut self, context: Context<'_>, word: &str, times: u32) {
        let previous = context.previous.unwrap_or(SENTENCE_START);
        if let Some(next) = self.pairs.get_mut(previous) {
            let removed = remove_times(next, word, times);
            if next.is_empty() {
                self.pairs.remove(previous);
            }
            decrement(&mut self.context_totals, previous, removed);
            decrement(&mut self.word_counts, word, removed);
            self.total = self.total.saturating_sub(u64::from(removed));
        }
        let Some(previous) = context.previous else {
            return;
        };
        let earlier = context.earlier.unwrap_or(SENTENCE_START);
        let Some(by_previous) = self.triples.get_mut(earlier) else {
            return;
        };
        let Some(next) = by_previous.get_mut(previous) else {
            return;
        };
        let removed = remove_times(next, word, times);
        if next.is_empty() {
            by_previous.remove(previous);
        }
        if by_previous.is_empty() {
            self.triples.remove(earlier);
        }
        if let Some(totals) = self.triple_totals.get_mut(earlier) {
            decrement(totals, previous, removed);
            if totals.is_empty() {
                self.triple_totals.remove(earlier);
            }
        }
    }

    /// 忘掉一个词：删掉它在任何位置出现的全部转移，重算各项合计。返回删掉的二元计数总和。
    pub fn forget_word(&mut self, word: &str) -> u32 {
        let mut removed = 0u32;
        if let Some(next) = self.pairs.remove(word) {
            for (following, count) in next {
                removed += count;
                decrement(&mut self.word_counts, &following, count);
            }
            self.context_totals.remove(word);
        }
        let mut emptied = Vec::new();
        for (previous, next) in &mut self.pairs {
            if let Some(count) = next.remove(word) {
                removed += count;
                decrement(&mut self.context_totals, previous, count);
                if next.is_empty() {
                    emptied.push(previous.clone());
                }
            }
        }
        for previous in emptied {
            self.pairs.remove(&previous);
        }
        self.word_counts.remove(word);
        self.total = self.total.saturating_sub(u64::from(removed));
        self.forget_word_in_triples(word);
        removed
    }

    /// 三元表里删掉 `word` 作为前二词、前词、后词的全部条目。
    fn forget_word_in_triples(&mut self, word: &str) {
        self.triples.remove(word);
        self.triple_totals.remove(word);
        let mut emptied = Vec::new();
        for (earlier, by_previous) in &mut self.triples {
            by_previous.remove(word);
            let totals = self.triple_totals.entry(earlier.clone()).or_default();
            totals.remove(word);
            let mut cleared = Vec::new();
            for (previous, next) in by_previous.iter_mut() {
                if let Some(count) = next.remove(word) {
                    decrement(totals, previous, count);
                    if next.is_empty() {
                        cleared.push(previous.clone());
                    }
                }
            }
            for previous in cleared {
                by_previous.remove(&previous);
            }
            if by_previous.is_empty() {
                emptied.push(earlier.clone());
            }
        }
        for earlier in emptied {
            self.triples.remove(&earlier);
            self.triple_totals.remove(&earlier);
        }
    }

    /// 词的出现次数。
    pub fn count(&self, word: &str) -> u32 {
        self.word_counts.get(word).copied().unwrap_or(0)
    }

    /// `word` 紧跟在 `previous` 之后的次数。
    pub fn pair(&self, previous: Option<&str>, word: &str) -> u32 {
        self.pairs
            .get(previous.unwrap_or(SENTENCE_START))
            .and_then(|next| next.get(word))
            .copied()
            .unwrap_or(0)
    }

    /// `word` 跟在 `earlier previous` 之后的次数（`earlier` 为 `None` 是句首标记）。
    pub fn triple(&self, earlier: Option<&str>, previous: &str, word: &str) -> u32 {
        self.triples
            .get(earlier.unwrap_or(SENTENCE_START))
            .and_then(|by_previous| by_previous.get(previous))
            .and_then(|next| next.get(word))
            .copied()
            .unwrap_or(0)
    }

    /// 不同的 (前词, 后词) 对数。
    pub fn pair_count(&self) -> usize {
        self.pairs.values().map(HashMap::len).sum()
    }

    /// 不同的 (前二词, 前词, 后词) 条数。
    pub fn triple_count(&self) -> usize {
        self.triples
            .values()
            .flat_map(HashMap::values)
            .map(HashMap::len)
            .sum()
    }

    /// 二元与三元条目总数，决定什么时候减半。
    pub fn transition_count(&self) -> usize {
        self.pair_count() + self.triple_count()
    }

    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// 把静态模型给出的 `log P(word | previous)` 与个人概率插值后返回。
    ///
    /// 个人二元 P₂ = λ·c(v,w)/c(v) + (1-λ)·c(w)/N；这对上文 (u,v) 见过时再套一层绝对折扣的三元：
    /// P₃ = max(c(u,v,w) − D, 0)/c(u,v) + D·N₁₊(u,v,·)/c(u,v)·P₂，没见过的接续只拿回退的份额。
    /// 插值权重 μ = c(v)/(c(v)+K)，封顶 [`MAX_CONFIDENCE`]：见过这个前词越多越信个人数据，但永远压不死静态模型。
    /// 前词从没见过时（句首用句首标记）原样返回。
    pub fn blend(&self, context: Context<'_>, word: &str, base_log_prob: f64) -> f64 {
        let previous = context.previous.unwrap_or(SENTENCE_START);
        let Some(&context_total) = self.context_totals.get(previous) else {
            return base_log_prob;
        };
        if context_total == 0 || self.total == 0 {
            return base_log_prob;
        }
        let pair = f64::from(self.pair(context.previous, word));
        let unigram = f64::from(self.count(word)) / self.total as f64;
        let bigram = USER_LAMBDA * pair / f64::from(context_total) + (1.0 - USER_LAMBDA) * unigram;
        let personal = match context
            .previous
            .and_then(|p| self.triple_row(context.earlier, p))
        {
            Some((next, triple_total)) if triple_total > 0 => {
                let triple_total = f64::from(triple_total);
                let seen = next.get(word).copied().unwrap_or(0);
                let discounted = (f64::from(seen) - TRIGRAM_DISCOUNT).max(0.0) / triple_total;
                let backoff = TRIGRAM_DISCOUNT * next.len() as f64 / triple_total;
                discounted + backoff * bigram
            }
            _ => bigram,
        };
        let confidence = (f64::from(context_total) / (f64::from(context_total) + CONFIDENCE_K))
            .min(MAX_CONFIDENCE);
        let blended = (1.0 - confidence) * base_log_prob.exp() + confidence * personal;
        blended.max(f64::MIN_POSITIVE).ln()
    }

    /// 上文 (earlier, previous) 下的后词表与总次数；没见过这对上文返回 `None`。
    fn triple_row(
        &self,
        earlier: Option<&str>,
        previous: &str,
    ) -> Option<(&HashMap<String, u32>, u32)> {
        let earlier = earlier.unwrap_or(SENTENCE_START);
        let next = self.triples.get(earlier)?.get(previous)?;
        let total = self.triple_totals.get(earlier)?.get(previous).copied()?;
        Some((next, total))
    }

    /// 所有计数减半，去掉减到零的。
    fn decay(&mut self) {
        let mut pairs: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for (previous, next) in &self.pairs {
            let halved = halve(next);
            if !halved.is_empty() {
                pairs.insert(previous.clone(), halved);
            }
        }
        let mut triples: HashMap<String, HashMap<String, HashMap<String, u32>>> = HashMap::new();
        for (earlier, by_previous) in &self.triples {
            for (previous, next) in by_previous {
                let halved = halve(next);
                if !halved.is_empty() {
                    triples
                        .entry(earlier.clone())
                        .or_default()
                        .insert(previous.clone(), halved);
                }
            }
        }
        *self = Self::from_counts(pairs, triples);
    }

    fn from_counts(
        pairs: HashMap<String, HashMap<String, u32>>,
        triples: HashMap<String, HashMap<String, HashMap<String, u32>>>,
    ) -> Self {
        let mut model = Self {
            pairs,
            triples,
            ..Self::default()
        };
        for (previous, next) in &model.pairs {
            for (word, count) in next {
                *model.context_totals.entry(previous.clone()).or_default() += count;
                *model.word_counts.entry(word.clone()).or_default() += count;
                model.total += u64::from(*count);
            }
        }
        for (earlier, by_previous) in &model.triples {
            let totals = model.triple_totals.entry(earlier.clone()).or_default();
            for (previous, next) in by_previous {
                *totals.entry(previous.clone()).or_default() += next.values().sum::<u32>();
            }
        }
        model
    }

    /// 从 TSV 解析；空行与 `#` 开头的行跳过，三列是二元、四列是三元，格式不对的行返回行号。
    pub fn parse(source: &str) -> Result<Self, usize> {
        let (model, skipped) = Self::parse_lenient(source);
        match skipped.first() {
            Some(&line) => Err(line),
            None => Ok(model),
        }
    }

    /// 同 [`Self::parse`]，但坏行跳过而不是整份报错，返回 (模型, 跳过的行号)。
    /// 学习数据文件可能被崩溃写坏一两行，不能因此丢掉整个个人模型。
    pub fn parse_lenient(source: &str) -> (Self, Vec<usize>) {
        let mut pairs: HashMap<String, HashMap<String, u32>> = HashMap::new();
        let mut triples: HashMap<String, HashMap<String, HashMap<String, u32>>> = HashMap::new();
        let mut skipped = Vec::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let (context, word, count) = match fields.as_slice() {
                [previous, word, count] => (vec![*previous], *word, *count),
                [earlier, previous, word, count] => (vec![*earlier, *previous], *word, *count),
                _ => {
                    skipped.push(index + 1);
                    continue;
                }
            };
            let Ok(count) = count.trim().parse::<u32>() else {
                skipped.push(index + 1);
                continue;
            };
            if count == 0 || word.is_empty() {
                continue;
            }
            let slot = match context.as_slice() {
                [previous] => pairs.entry((*previous).to_owned()).or_default(),
                [earlier, previous] => triples
                    .entry((*earlier).to_owned())
                    .or_default()
                    .entry((*previous).to_owned())
                    .or_default(),
                _ => unreachable!(),
            };
            *slot.entry(word.to_owned()).or_default() += count;
        }
        (Self::from_counts(pairs, triples), skipped)
    }

    /// 序列化成 TSV 行（不含表头），二元在前三元在后，各自排好序，便于人看和 diff。
    pub fn to_tsv(&self) -> String {
        let mut rows: Vec<(&str, &str, u32)> = self
            .pairs
            .iter()
            .flat_map(|(previous, next)| {
                next.iter()
                    .map(move |(word, count)| (previous.as_str(), word.as_str(), *count))
            })
            .collect();
        rows.sort_unstable();
        let mut out = String::new();
        for (previous, word, count) in rows {
            out.push_str(previous);
            out.push('\t');
            out.push_str(word);
            out.push('\t');
            out.push_str(&count.to_string());
            out.push('\n');
        }
        let mut rows: Vec<(&str, &str, &str, u32)> = self
            .triples
            .iter()
            .flat_map(|(earlier, by_previous)| {
                by_previous.iter().flat_map(move |(previous, next)| {
                    next.iter().map(move |(word, count)| {
                        (earlier.as_str(), previous.as_str(), word.as_str(), *count)
                    })
                })
            })
            .collect();
        rows.sort_unstable();
        for (earlier, previous, word, count) in rows {
            out.push_str(earlier);
            out.push('\t');
            out.push_str(previous);
            out.push('\t');
            out.push_str(word);
            out.push('\t');
            out.push_str(&count.to_string());
            out.push('\n');
        }
        out
    }
}

/// 计数表里某一项减 `by`，减到零就删掉。
fn decrement(table: &mut HashMap<String, u32>, key: &str, by: u32) {
    if let Some(count) = table.get_mut(key) {
        *count = count.saturating_sub(by);
        if *count == 0 {
            table.remove(key);
        }
    }
}

/// 从后词表里退回 `word` 最多 `times` 份，返回实际退回的份数；减到零就删掉。
fn remove_times(next: &mut HashMap<String, u32>, word: &str, times: u32) -> u32 {
    let Some(count) = next.get_mut(word) else {
        return 0;
    };
    let removed = times.min(*count);
    *count -= removed;
    if *count == 0 {
        next.remove(word);
    }
    removed
}

/// 计数减半，去掉减到零的。
fn halve(next: &HashMap<String, u32>) -> HashMap<String, u32> {
    next.iter()
        .filter(|(_, count)| **count >= 2)
        .map(|(word, count)| (word.clone(), count / 2))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_transitions_and_round_trips_through_tsv() {
        let mut model = UserNgram::default();
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        model.record(Context::after_two("我", "想"), "去");
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        assert_eq!(model.count("我"), 2);
        assert_eq!(model.count("想"), 2);
        assert_eq!(model.pair(None, "我"), 2);
        assert_eq!(model.pair(Some("我"), "想"), 2);
        assert_eq!(model.pair(Some("我"), "去"), 0);
        assert_eq!(model.pair_count(), 3);
        // 句首词不记三元；第二个词的前二词是句首标记
        assert_eq!(model.triple(None, "我", "想"), 2);
        assert_eq!(model.triple(Some("我"), "想", "去"), 1);
        assert_eq!(model.triple_count(), 2);

        let tsv = model.to_tsv();
        assert!(tsv.contains("<s>\t我\t2\n"));
        assert!(tsv.contains("<s>\t我\t想\t2\n"));
        assert!(tsv.contains("我\t想\t去\t1\n"));
        let restored = UserNgram::parse(&tsv).unwrap();
        assert_eq!(restored.pair(Some("我"), "想"), 2);
        assert_eq!(restored.count("去"), 1);
        assert_eq!(restored.pair_count(), 3);
        assert_eq!(restored.triple(None, "我", "想"), 2);
        assert_eq!(restored.triple_count(), 2);
        assert_eq!(restored.to_tsv(), tsv);
        assert_eq!(UserNgram::parse("我\t想\n").unwrap_err(), 1);
        assert_eq!(UserNgram::parse("<s>\t我\t想\t去\t1\n").unwrap_err(), 1);
        // 宽松解析：坏行跳过，好行照读
        let (model, skipped) = UserNgram::parse_lenient("<s>\t我\t3\n坏行\n我\t想\tx\n我\t想\t2\n");
        assert_eq!(skipped, [2, 3]);
        assert_eq!(model.pair(None, "我"), 3);
        assert_eq!(model.pair(Some("我"), "想"), 2);
    }

    /// 旧版只有二元的文件照读，三元表为空。
    #[test]
    fn parses_bigram_only_files() {
        let model = UserNgram::parse("<s>\t我\t3\n我\t想\t2\n").unwrap();
        assert_eq!(model.pair(Some("我"), "想"), 2);
        assert_eq!(model.triple_count(), 0);
        assert_eq!(
            model.blend(Context::after_two("我", "想"), "去", -5.0),
            model.blend(Context::after("想"), "去", -5.0)
        );
    }

    #[test]
    fn blend_favors_seen_continuations_but_never_kills_unseen_ones() {
        let mut model = UserNgram::default();
        let base_ba = (-9.0_f64).exp().ln();
        // 前词没见过：原样返回
        assert_eq!(model.blend(Context::after("吃饭"), "把", base_ba), base_ba);
        // 选过两次 吃饭 → 把：个人证据抬上来
        model.record(Context::after("吃饭"), "把");
        model.record(Context::after("吃饭"), "把");
        let lifted = model.blend(Context::after("吃饭"), "把", base_ba);
        assert!(lifted > -2.5, "{lifted}");
        // 没跟在 吃饭 后面出现过的 吧 只是打折，不会被压死
        let base_ba_particle = -2.0;
        let discounted = model.blend(Context::after("吃饭"), "吧", base_ba_particle);
        assert!(discounted < base_ba_particle);
        assert!(discounted > base_ba_particle + (1.0 - MAX_CONFIDENCE).ln() - 1e-9);
    }

    /// 三元分辨二元分不开的接续：「想 → 去」在「我想」后面和「不想」后面偏好不同。
    #[test]
    fn trigram_separates_continuations_the_bigram_mixes() {
        let mut model = UserNgram::default();
        for _ in 0..3 {
            model.record(Context::after_two("我", "想"), "去");
            model.record(Context::after_two("不", "想"), "要");
        }
        let base = -6.0;
        let after_wo = model.blend(Context::after_two("我", "想"), "去", base);
        let after_bu = model.blend(Context::after_two("不", "想"), "去", base);
        let bigram_only = model.blend(Context::after("想"), "去", base);
        assert!(after_wo > bigram_only, "{after_wo} vs {bigram_only}");
        assert!(bigram_only > after_bu, "{bigram_only} vs {after_bu}");
        // 三元上文没见过的（「很想」）退回二元
        assert_eq!(
            model.blend(Context::after_two("很", "想"), "去", base),
            bigram_only
        );
        // 没见过的接续在见过的三元上文里仍拿到回退份额，不会被压死
        let unseen = model.blend(Context::after_two("我", "想"), "要", base);
        assert!(unseen > base + (1.0 - MAX_CONFIDENCE).ln() - 1e-9);
        assert!(unseen < after_wo);
    }

    #[test]
    fn forget_word_removes_it_from_every_position() {
        let mut model = UserNgram::default();
        model.record(Context::START, "我");
        model.record(Context::after("我"), "想");
        model.record(Context::after_two("我", "想"), "去");
        model.record(Context::after("我"), "去");
        assert_eq!(model.forget_word("想"), 2);
        assert_eq!(model.count("想"), 0);
        assert_eq!(model.pair(Some("我"), "想"), 0);
        assert_eq!(model.pair(Some("想"), "去"), 0);
        assert_eq!(model.pair(Some("我"), "去"), 1);
        assert_eq!(model.count("去"), 1);
        assert_eq!(model.pair_count(), 2);
        assert_eq!(model.triple(None, "我", "想"), 0);
        assert_eq!(model.triple(Some("我"), "想", "去"), 0);
        assert_eq!(model.triple(None, "我", "去"), 1);
        assert_eq!(model.triple_count(), 1);
        assert_eq!(model.forget_word("没有"), 0);
        // 作为前二词删掉
        assert_eq!(model.forget_word("我"), 2);
        assert_eq!(model.triple_count(), 0);
        assert!(model.is_empty());
    }

    #[test]
    fn decays_when_too_many_transitions() {
        let mut model = UserNgram::default();
        // 每条记录一个二元加一个三元
        for i in 0..MAX_USER_TRANSITIONS / 2 {
            model.record(Context::after("甲"), &format!("乙{i}"));
        }
        for _ in 0..3 {
            model.record(Context::after("甲"), "丙");
        }
        // 超上限后减半：单次的都被忘掉，选过三次的还留着
        model.record(Context::after("甲"), "丁");
        assert!(model.transition_count() <= 4);
        assert!(model.pair(Some("甲"), "丙") >= 1);
        assert!(model.triple(None, "甲", "丙") >= 1);
        assert_eq!(model.pair(Some("甲"), "乙0"), 0);
        assert_eq!(model.triple(None, "甲", "乙0"), 0);
    }

    #[test]
    fn unrecord_reverses_record_and_ignores_unknown_transitions() {
        let mut model = UserNgram::default();
        model.record_times(Context::START, "好的", 2);
        model.record(Context::after("好的"), "啊");
        model.unrecord(Context::START, "好的", 2);
        assert_eq!(model.pair(None, "好的"), 0);
        assert_eq!(model.count("好的"), 0);
        assert_eq!(model.pair(Some("好的"), "啊"), 1);
        assert_eq!(model.triple(None, "好的", "啊"), 1);
        assert_eq!(model.pair_count(), 1);
        model.unrecord(Context::after("没有"), "的", 5);
        model.unrecord(Context::after("好的"), "啊", 5);
        assert!(model.is_empty());
        assert_eq!(model.transition_count(), 0);
    }
}
