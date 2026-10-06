//! 整句候选：把词图里重排过的前几条路径变成候选，以及整句转换本身。

use super::leading_english;
use crate::candidate::Candidate;
use crate::candidate::CandidateKind;
use crate::engine::ALTERNATE_MIN_SYLLABLES;
use crate::engine::Engine;
use crate::engine::RESCORE_PATHS;
use crate::engine::SENTENCE_CANDIDATES;
use crate::engine::learning::Learner;
use crate::engine::query::english_tail::EnglishTail;
use crate::parser;
use crate::parser::Segmentation;
use crate::sentence;
use crate::sentence::Conversion;

impl Engine {
    /// 整句候选。没有英文尾段时是整段拼音的转换（[`Self::plain_sentence`]），排在开头的英文候选之后。
    /// 有英文尾段且英文读法胜出（`head_wins`）时，头段的转换加上那个词排第一（`woxiangxuehaorust` → 我想学好rust），
    /// 整段也能读成拼音的再把拼音读法的整句放在第二；英文读法输了就不出（`diaoyong` 不出 掉Yong），
    /// 免得把真正要的候选往后挤。
    /// `typos` 为假时词图里不加敲错边（整段一处编辑的纠错已经生效，不在纠正后的拼音上再猜第二处）。
    pub(in crate::engine) fn insert_sentence(
        &self,
        items: &mut Vec<Candidate>,
        segmentations: &[Segmentation],
        typos: bool,
        english_tail: Option<&EnglishTail>,
        head_wins: bool,
    ) {
        let Some(best) = segmentations.first() else {
            return;
        };
        let keys = self.composition.scope();
        let first_segmentation = |text: &str| parser::segment(text).ok()?.into_iter().next();
        match english_tail {
            Some(tail) if head_wins => {
                if let Some(mixed) = self.mixed_sentence(best, tail, typos) {
                    items.insert(0, mixed);
                }
                // 整段读成拼音的那条本来就是「另一种读法」，只给一条
                if tail.competes
                    && let Some(full) = first_segmentation(keys)
                    && let Some(plain) = self.plain_sentence(items, &full, typos, false).pop()
                {
                    let position = items.len().min(1);
                    items.insert(position, plain);
                }
            }
            _ => {
                // 整段本身就是英文词（`database`）：备选整句会把那条英文候选再往后挤，只给一条
                let alternates = !self.scope_is_english_word();
                let sentences = self.plain_sentence(items, best, typos, alternates);
                let position = leading_english(items);
                // 词图读不通整段时模型直接生成的整句排在词图那几条前面：这时词图给的是把英文段
                // 硬读成拼音的结果（`yongdockerbushuhenfangbian` → 用的哦乘客仍不熟很方便），排它前面没有可惜的
                // `typos` 为假就是拼写纠错已经生效（见调用处），那时 `best` 是纠正后的切分
                let generated: Vec<Candidate> = self
                    .generated_sentence_candidates(best, keys, !typos)
                    .into_iter()
                    .filter(|g| {
                        !items
                            .iter()
                            .chain(sentences.iter())
                            .any(|c| c.text == g.text)
                    })
                    .collect();
                let generated_count = generated.len();
                for (offset, candidate) in generated.into_iter().enumerate() {
                    items.insert((position + offset).min(items.len()), candidate);
                }
                for (offset, plain) in sentences.into_iter().enumerate() {
                    items.insert(
                        (position + generated_count + offset).min(items.len()),
                        plain,
                    );
                }
            }
        }
    }

    /// 整段拼音的整句候选：最优切分至少两个音节、且最优路径不止一个词时才有（空格上屏的就是它）。
    /// 整段本身就是词库里的词时不重复；有音节没转成字的不算句子。
    /// 词级候选里已有同文本同读音的候选时不出（那条留在词级排序给它的位置），同文本不同读音的从 `items` 里去掉。
    ///
    /// `alternates` 为真且输入够长时把重排后的第 2、3 条路径也给出来（[`SENTENCE_CANDIDATES`] 条封顶）：
    /// 长句错一个字就得整句拆开重打，一条候选不够用；短输入不给，那几格留给词级候选更值。
    /// 备选只收真整句（路径上不止一个词），「整段读成一个词」那种噪声信道的判断只认第一条。
    pub(in crate::engine) fn plain_sentence(
        &self,
        items: &mut Vec<Candidate>,
        best: &Segmentation,
        typos: bool,
        alternates: bool,
    ) -> Vec<Candidate> {
        if best.syllables.len() < 2 {
            return Vec::new();
        }
        let wanted = if alternates && best.syllables.len() >= ALTERNATE_MIN_SYLLABLES {
            SENTENCE_CANDIDATES
        } else {
            1
        };
        let mut paths = self.sentence_paths(&best.patterns(), typos, false, wanted);
        if paths.is_empty() {
            return Vec::new();
        }
        // 不按原样读的路径（敲错边 / 模糊音）不许压过「敲的拼音本身就是一个词」：`jineng` 按 `jin eng` 切时
        // 词图里没有 技能，敲错边读出 近藤；`ceshi` 读出 的是。词级候选里有音节正好拼成整段输入的词时退回原样的路径
        if paths[0].altered() {
            let letters = best.joined("");
            let spelled_exactly = items
                .iter()
                .any(|c| c.kind == CandidateKind::Chinese && c.syllables.concat() == letters);
            if spelled_exactly {
                paths = self.sentence_paths(&best.patterns(), false, false, wanted);
            }
        }
        let mut out: Vec<Candidate> = Vec::new();
        for (rank, conversion) in paths.into_iter().enumerate() {
            if out.len() >= wanted {
                break;
            }
            // 最优路径不合格就一条都不出：它不合格多半是「整段本来就是一个词」（`nihao` → 你好），
            // 这时越过它去拿第二条路径，等于把 你号 插到词库词 你好 前面
            let first = rank == 0;
            if conversion.has_placeholder() {
                if first {
                    return Vec::new();
                }
                continue;
            }
            // 整段本来就是一个词时不出整句；但路径靠敲错变体把整段读成的一个词（`meiganxi` → 没关系）是噪声信道的判断，
            // 词级查询按原样查不到它，作为普通词候选插到最前。只读了一部分（末尾没打完的音节没算进去）的不插。
            // 备选只收真整句，单个词的第 2、3 条路径是噪声
            let kind = if conversion.word_count() >= 2 {
                CandidateKind::Sentence
            } else if first
                && conversion.altered()
                && conversion.syllables.len() == best.syllables.len()
            {
                CandidateKind::Chinese
            } else if first {
                return Vec::new();
            } else {
                continue;
            };
            if out.iter().any(|c| c.text == conversion.text) {
                continue;
            }
            // 词级候选里已经有同样的文本：读音也相同就是同一个候选，不重复插、词留在词级排序给它的位置
            //（先是 / 有的 这种整句恰好拼成一个词的，词级排序更可信）；读音不同的是按别的读音对上的词
            //（云端学来的错读音用户词 `我的 wo di` 靠敲错变体对上 `wode`），那条不是这个候选，去掉它，整句以正确读音顶上
            if let Some(index) = items.iter().position(|c| c.text == conversion.text) {
                if items[index].syllables == conversion.syllables {
                    // 首选与词级候选撞上：整句这边一条都不出（备选插在它前面会绕到那条词前面去）
                    if first {
                        return Vec::new();
                    }
                    continue;
                }
                items.remove(index);
            }
            out.push(Candidate {
                rime: None,
                text: conversion.text,
                kind,
                syllables: conversion.syllables,
                reading: None,
                translation: None,
                aux_code: None,
            });
        }
        out
    }

    /// 跑一次整句转换：主词库 + 用户词（含模糊音与敲错写法，命中的按代价扣分），静态语言模型与个人 n-gram 插值，用户选择次数加分。
    /// `typos` 为假时不加敲错边。
    pub(in crate::engine) fn convert_sentence(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
    ) -> Option<Conversion> {
        self.convert_sentence_with(patterns, typos, false)
    }

    /// 同 [`Self::convert_sentence`]，`whole` 为真时末尾单字母也读（[`sentence::convert_whole`]），只给比分用。
    /// 接了神经重打分器时取前 [`RESCORE_PATHS`] 条路径，按「路径分 + λ·(神经分 − 静态分)」重排（[`Self::rescore_paths`]）：
    /// 神经分替换的是静态二元模型那部分判断，个人 n-gram 插值、用户加分、敲错代价原样保留。
    /// 返回重排后的第一条；`score` 始终是静态尺度的路径分，神经分不写进去，跨读法的比较才还成立。
    pub(in crate::engine) fn convert_sentence_with(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
        whole: bool,
    ) -> Option<Conversion> {
        self.sentence_paths(patterns, typos, whole, 1)
            .into_iter()
            .next()
    }

    /// 重排后的整句路径，最好的在前。`want` 是调用方要几条：只要一条时没接模型就只算一条
    /// （拼写纠错对每个候选都要转一次，多算几条纯是浪费）；要备选时至少算这么多条。
    pub(in crate::engine) fn sentence_paths(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
        whole: bool,
        want: usize,
    ) -> Vec<Conversion> {
        let dictionaries = self.all_dictionaries();
        let expanded = self.expand_positions(patterns, typos);
        let k = if self.has_sentence_scorer() {
            RESCORE_PATHS.max(want)
        } else {
            want
        };
        let mut paths = sentence::convert_paths(
            &dictionaries,
            &expanded.positions(),
            whole,
            k,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |index, syllable| expanded.cost(index, syllable),
            &mut self.span_cache.borrow_mut(),
        );
        // 与最优路径差得太远的不参与：那种差距多半是个人 n-gram 拉开的
        if paths.len() > 1 {
            let floor = paths[0].score - self.neural_margin;
            paths.retain(|p| p.score >= floor);
            // 条件是这批路径共同解释的那段按键，不是整个作用域：英文尾巴那条只转换了 head，
            // 拿整段当条件会让它凭空背上没覆盖的字母
            let keys: String = patterns.iter().map(|p| p.text).collect();
            self.rescore_paths(&mut paths, &keys);
        }
        paths
    }
}
