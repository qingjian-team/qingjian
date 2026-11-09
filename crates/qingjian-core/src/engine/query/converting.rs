//! 整句候选：把词图里重排过的前几条路径变成候选，以及整句转换本身。

use crate::engine::choice::char_emission;
use crate::engine::query::{EnglishTail, leading_english};
use crate::engine::{
    ALTERNATE_MIN_SYLLABLES, CHOICE_SENTENCE_CANDIDATES, Learner, RESCORE_PATHS,
    SENTENCE_CANDIDATES, TIED_SEGMENTATIONS, choice_key, reads_as_typed,
};
use crate::sentence::Conversion;
use crate::{Candidate, CandidateKind, Engine, Segmentation, parser, sentence};

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
        let Some(best) = self.sentence_segmentation(segmentations, typos) else {
            return;
        };
        let keys = self.composition.scope();
        match english_tail {
            Some(tail) if head_wins => {
                if let Some(mixed) = self.mixed_sentence(best, tail, typos) {
                    items.insert(0, mixed);
                }
                // 整段读成拼音的那条本来就是「另一种读法」，只给一条：与比分时同一种切分，取原样读的那条（第二条是纠正）
                let segmentations = parser::segment(keys).ok();
                if tail.competes
                    && let Some(full) = segmentations
                        .as_deref()
                        .and_then(|s| self.sentence_segmentation(s, typos))
                    && let Some(plain) = self
                        .plain_sentence(items, full, typos, false)
                        .into_iter()
                        .next()
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
                // 接了生成器时拼音干净的输入也是生成的在前，没打完、纠正过的词图在前（见 `generated_sentence_candidates`）
                let (generated, lead) =
                    self.generated_sentence_candidates(best, keys, !typos, &sentences);
                let mut sentences = sentences;
                let mut ordered: Vec<Candidate> = Vec::new();
                if !lead {
                    // 词图没出整句时生成的跟在原来排第一的那个候选后面
                    if !sentences.is_empty() {
                        ordered.push(sentences.remove(0));
                    } else if position < items.len() {
                        ordered.push(items.remove(position));
                    }
                }
                let mixed = !generated.is_empty();
                // 生成的与词图的整句相同时用词图那条（带读音，上屏能学），但排在生成给的名次上
                for g in generated {
                    if items.iter().chain(ordered.iter()).any(|c| c.text == g.text) {
                        continue;
                    }
                    match sentences.iter().position(|c| c.text == g.text) {
                        Some(i) => ordered.push(sentences.remove(i)),
                        None => ordered.push(g),
                    }
                }
                ordered.extend(sentences);
                if mixed {
                    // 生成的加词图的整句一起最多 SENTENCE_CANDIDATES 条，不把第一页的词挤走（#769）
                    let mut kept = 0;
                    ordered.retain(|c| {
                        if !matches!(c.kind, CandidateKind::Sentence | CandidateKind::Generated) {
                            return true;
                        }
                        kept += 1;
                        kept <= SENTENCE_CANDIDATES
                    });
                }
                // 词级排第一、盖住整段的用户词（逐字选出的自造词、整句记忆）排在整句与生成的整句前面，
                // 除非排第一的整句在这串键下选得更多：简拼 `xmqlxx` 的 晓铭去了学校 没在这串键下选过，也不该排在 下面请来学习 后面。
                // 词库里的词不这样，同一串键这次要词还是要整句由上下文定（`shiyong` 选过 使用，这次要 是用）
                let letters = choice_key(keys, keys.len());
                let lead_choice = ordered
                    .first()
                    .map_or(0, |s| self.learner.choice_weight(&letters, &s.text));
                let user_word_first = items.get(position).is_some_and(|c| {
                    c.kind == CandidateKind::Chinese
                        && c.syllables.len() == best.syllables.len()
                        && self.is_user_word(c)
                        && self.learner.choice_weight(&letters, &c.text) >= lead_choice
                });
                let position = position + usize::from(user_word_first);
                for (offset, candidate) in ordered.into_iter().enumerate() {
                    items.insert((position + offset).min(items.len()), candidate);
                }
            }
        }
    }

    /// 整句按哪种切分转：切分器只按结构排序（音节少、完整的在前），打平时靠贪心，`shangou` 排在前面的是 `shang ou`。
    /// 打平的几种（音节数与不完整音节数都与第一种相同）各转一遍，取路径分最高的，分数相同时留排在前面的。
    pub(in crate::engine) fn sentence_segmentation<'s>(
        &self,
        segmentations: &'s [Segmentation],
        typos: bool,
    ) -> Option<&'s Segmentation> {
        let first = segmentations.first()?;
        let tied = |s: &&Segmentation| {
            s.syllables.len() == first.syllables.len()
                && s.incomplete_count() == first.incomplete_count()
        };
        let mut best = (f64::NEG_INFINITY, first);
        for segmentation in segmentations
            .iter()
            .take_while(tied)
            .take(TIED_SEGMENTATIONS)
        {
            let score = self
                .convert_sentence(&segmentation.patterns(), typos)
                .filter(|conversion| !conversion.has_placeholder())
                .map_or(f64::NEG_INFINITY, |conversion| conversion.score);
            if score > best.0 {
                best = (score, segmentation);
            }
        }
        Some(best.1)
    }

    /// 整段拼音的整句候选：最优切分至少两个音节、且最优路径不止一个词时才有（空格上屏的就是它）。
    /// 整段本身就是词库里的词时不重复；有音节没转成字的不算句子。
    /// 词级候选里已有同文本同读音的候选时不出（那条留在词级排序给它的位置），同文本不同读音的从 `items` 里去掉。
    ///
    /// `alternates` 为真且输入够长时把重排后的第 2、3 条路径也给出来（[`SENTENCE_CANDIDATES`] 条封顶）：
    /// 长句错一个字就得整句拆开重打，一条候选不够用；短输入不给，那几格留给词级候选更值。
    /// 备选只收真整句（路径上不止一个词），「整段读成一个词」那种噪声信道的判断只认第一条。
    ///
    /// 敲的拼音每个音节都完整（[`reads_as_typed`]）、原样读出的最优路径又全是认识的词（[`Self::credible`]）时只按原样读：
    /// 敲错边读出的最优路径与原样不同就当第二条给（`shabi` 原样 沙比 / 傻逼，杀不 排第二；`meiganxi` 原样 美感洗，没关系 排第二），
    /// 要不要把它提到第一交给带前文的选词模型去比。原样读不通（有残缺音节，或只读得出 咋嫩 这种碎片）时敲错边照旧可以赢（`zanen` → 咱们）。
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
        // 末尾只敲了一个声母（viterbi 放行的落单声母，见 #619）时，词级已有一个整词逐音节对上
        // 全部输入（`houxuank` → 候选框、`zaiw` → 在外），整句把同一个声母猜成另一句话
        // （后悬空、在我）就不该插到那个词前面：与「整段本就是词库词时不出整句」同一道理。
        let trailing_single_initial =
            best.last_is_partial() && best.syllables.last().is_some_and(|s| s.text.len() == 1);
        if trailing_single_initial {
            let patterns = best.patterns();
            if items.iter().any(|c| {
                c.kind == CandidateKind::Chinese
                    && c.syllables.len() == patterns.len()
                    && c.syllables
                        .iter()
                        .zip(&patterns)
                        .all(|(syllable, pattern)| pattern.accepts(syllable))
            }) {
                return Vec::new();
            }
        }
        let wanted = if alternates && best.syllables.len() >= ALTERNATE_MIN_SYLLABLES {
            if self.has_choice_model() {
                CHOICE_SENTENCE_CANDIDATES
            } else {
                SENTENCE_CANDIDATES
            }
        } else {
            1
        };
        let (as_typed, corrected) = if typos && reads_as_typed(best) {
            let corrected = self
                .sentence_paths(&best.patterns(), true, false, 1)
                .into_iter()
                .next()
                .filter(|path| path.altered() && !path.has_placeholder());
            let as_typed = self
                .sentence_paths(&best.patterns(), false, false, 1)
                .first()
                .is_some_and(|path| self.credible(path, corrected.as_ref()));
            (as_typed, corrected.filter(|_| as_typed))
        } else {
            (false, None)
        };
        let mut out = self.sentence_candidates(items, best, typos && !as_typed, wanted);
        let Some(corrected) = corrected else {
            return out;
        };
        if items
            .iter()
            .chain(out.iter())
            .any(|c| c.text == corrected.text)
        {
            return out;
        }
        self.corrected_alternates
            .borrow_mut()
            .push(corrected.text.clone());
        let candidate = Candidate {
            kind: if corrected.word_count() >= 2 {
                CandidateKind::Sentence
            } else {
                CandidateKind::Chinese
            },
            text: corrected.text,
            syllables: corrected.syllables,
            reading: None,
            translation: None,
            aux_code: None,
        };
        // 原样那条撞上词级候选、整句一条都不出时，备选跟在词级首选后面
        if out.is_empty() {
            let position = (leading_english(items) + 1).min(items.len());
            items.insert(position, candidate);
        } else {
            out.insert(1, candidate);
            out.truncate(wanted.max(2));
        }
        out
    }

    /// 原样读出的路径上，纠正那条没有的词语言模型都认识（没接模型时退回主词库里的词都算），且至少有一个多字词：这样才算「读得通」。
    /// 只看有没有词等于从不纠：青简词库里几乎每种读音都有个词（赞恩、咋嫩），它们模型不认识；
    /// 每个音节也总有个常用字，全靠单字拼出来的（赞 + 恩）不算。
    /// 不要求主词库：补充词库里模型认识的词（傻逼）也是用户敲的原话。
    /// 两条都有的词不算：长句里有个模型不认识的专名（菩提伽耶），纠正那条也照样带着它，不能因此判原样读不通（`xintaan…` → 新法案）。
    fn credible(&self, path: &Conversion, corrected: Option<&Conversion>) -> bool {
        let shared = |text: &str| corrected.is_some_and(|c| c.words.iter().any(|w| w.text == text));
        !path.has_placeholder()
            && path.words.iter().any(|word| word.text.chars().count() >= 2)
            && path
                .words
                .iter()
                .filter(|word| !shared(&word.text))
                .all(|word| {
                    if self.language_model.is_empty() {
                        self.in_primary_dictionary(&word.text, &word.syllables)
                    } else {
                        self.language_model.log_prob(None, &word.text).is_some()
                    }
                })
    }

    /// [`Self::plain_sentence`] 里按一种读法（`typos` 决定加不加敲错边）出的整句候选。
    fn sentence_candidates(
        &self,
        items: &mut Vec<Candidate>,
        best: &Segmentation,
        typos: bool,
        wanted: usize,
    ) -> Vec<Candidate> {
        let mut paths = self.sentence_paths(&best.patterns(), typos, false, wanted);
        if paths.is_empty() {
            return Vec::new();
        }
        // 不按原样读的路径（敲错边 / 模糊音）不许压过「敲的拼音本身就是一个词」：`jineng` 按 `jin eng` 切时
        // 词图里没有 技能，敲错边读出 近藤；`ceshi` 读出 的是。词级候选里有音节正好拼成整段输入的词时退回原样的路径。
        // 只认语言模型认识的主词库词（没接模型时认全部主词库词）：青简词库里几乎每种读音都有个词
        //（`zanen` 的 赞恩 / 咋嫩、`jiusi` 的 就死），算上它们敲错就再也纠不回 咱们 / 就是；模型没见过的词交给词图按兜底分去比
        if paths[0].altered() {
            let letters = best.joined("");
            let spelled_exactly = items.iter().any(|c| {
                c.kind == CandidateKind::Chinese
                    && c.syllables.concat() == letters
                    && self.in_primary_dictionary(&c.text, &c.syllables)
                    && (self.language_model.is_empty()
                        || self.language_model.log_prob(None, &c.text).is_some())
            });
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
            // 补充词库的整词（只用、可到）在词级排序里低一档，排在 智永 这种主词库生僻词后面；
            // 词图拿语言模型比过它和拆开的读法，它是最优就照样插到最前
            let whole_secondary = first
                && conversion.word_count() == 1
                && conversion.syllables.len() == best.syllables.len()
                && !self.in_primary_dictionary(&conversion.text, &conversion.syllables);
            let kind = if conversion.word_count() >= 2 {
                CandidateKind::Sentence
            } else if first
                && (conversion.altered() || whole_secondary)
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
            // 撞上的是补充词库的词就不算：它在词级排序里低一档，留着它等于把词图选的首选压到后面去
            if let Some(index) = items.iter().position(|c| c.text == conversion.text) {
                if items[index].syllables == conversion.syllables
                    && self.in_primary_dictionary(&items[index].text, &items[index].syllables)
                {
                    // 首选与词级候选撞上：整句这边一条都不出（备选插在它前面会绕到那条词前面去）
                    if first {
                        return Vec::new();
                    }
                    continue;
                }
                items.remove(index);
            }
            out.push(Candidate {
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

    /// 注音每个音节敲的声调：`patterns` 就是当前输入解出来的那串音节（或其开头一段）时才给，
    /// 纠错、英文尾段等换过切分的给空，免得声调错位到别的音节上。
    fn sentence_tones(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
    ) -> Vec<Option<u8>> {
        let Some(decoded) = self.decode(self.composition.scope()) else {
            return Vec::new();
        };
        let Some(segmentation) = decoded.segmentation() else {
            return Vec::new();
        };
        let aligned = patterns.len() <= segmentation.syllables.len()
            && patterns
                .iter()
                .zip(&segmentation.syllables)
                .all(|(pattern, syllable)| pattern.text == syllable.text);
        if aligned { decoded.tones() } else { Vec::new() }
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
        let k = if self.reranks_paths() {
            RESCORE_PATHS.max(want)
        } else {
            want
        };
        let table = self.char_table(patterns);
        let emission = |start: usize, text: &str, syllables: &[String]| {
            table.as_deref().map_or(0.0, |table| {
                char_emission(table, patterns, start, text, syllables)
            })
        };
        let tones = self.sentence_tones(patterns);
        let input = sentence::SearchInput::new(
            &dictionaries,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |index, syllable| expanded.cost(index, syllable),
        )
        .with_tones(&tones)
        .with_emission(&emission)
        .with_limits(self.search_limits);
        let mut paths = sentence::convert_paths(
            &input,
            &expanded.positions(),
            whole,
            k,
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

/// 选词模型重排完只留前 [`SENTENCE_CANDIDATES`] 条整句：词图交给模型的 [`CHOICE_SENTENCE_CANDIDATES`] 条多半只差一两个字，
/// 全显示会把第一页的词挤走。没被模型排到前面的那些本来就只是给它比的。
pub(in crate::engine) fn trim_sentence_alternates(items: &mut Vec<Candidate>) {
    let mut kept = 0;
    items.retain(|c| {
        if c.kind != CandidateKind::Sentence {
            return true;
        }
        kept += 1;
        kept <= SENTENCE_CANDIDATES
    });
}
