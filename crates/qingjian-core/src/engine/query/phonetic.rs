//! 拼音侧的候选生成：切分、拼写纠错、词级查找与排序，再补上整句、英文、快捷与 emoji 候选。

use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use qingjian_dictionary::Match;

use crate::correction::edit_distance;
use crate::engine::query::chinese_candidate;
use crate::engine::{
    ABBREVIATION_CHOICE_PREFIX, Learner, MAX_CANDIDATES, abbreviated_count, choice_key,
    pattern_key, segment_longest_prefix,
};
use crate::ranking::{FUZZY_PENALTY, Scored};
use crate::{
    Candidate, CandidateKind, CandidateList, Engine, ParseError, Query, Segmentation, Timings,
    correction, parser, ranking,
};

/// 排序用的选择次数里，真在这串键下选过的一次顶这么多次全拼按简拼记的（见 [`ABBREVIATION_CHOICE_PREFIX`]）。
const CHOICE_TIERS: u32 = 1024;

impl Engine {
    /// 拼音侧（全拼 / 双拼 / 注音）的候选生成：整段作用域是一串读音。
    pub(super) fn query_phonetic(
        &self,
        keys: &str,
        rest: String,
        start: Instant,
    ) -> Result<Query, ParseError> {
        // 双拼先解成全拼（音节间已用 `'` 连好，切分没有歧义），之后与全拼同路；解不动的键当尾巴。
        // 开了混输时一段键有多条读法，主读法管显示与尾巴，其余读法只多给切分
        let readings = self.decode_readings(keys);
        let decoded = readings.first();
        let scope: &str = decoded.map_or(keys, |d| d.pinyin());
        // 末尾是英文词（`woxiangxuehaorust`）：拼音候选与整句只按头段算，尾段整个跟在整句后面。
        // 整段也能读成拼音时（`database`、`…rust` 当简拼）两种读法比分，英文赢了才按头段算，
        // 输了整段按拼音读、英文读法排在拼音整句后面
        let english_tail = if decoded.is_none() {
            self.split_english_tail(keys)
        } else {
            None
        };
        let head_wins = english_tail
            .as_ref()
            .is_some_and(|t| !t.competes || self.mixed_beats_plain(keys, t));
        let parsed = match (decoded, &english_tail) {
            (Some(_), _) => {
                // 混输：每条读法各给一条切分（解不动的尾巴留在主读法上）
                let mut segs: Vec<Segmentation> = Vec::new();
                let mut sources: Vec<usize> = Vec::new();
                for (index, reading) in readings.iter().enumerate() {
                    if let Some(seg) = reading.segmentation()
                        && !segs
                            .iter()
                            .any(|existing| existing.syllables == seg.syllables)
                    {
                        segs.push(seg);
                        sources.push(index);
                    }
                }
                if segs.is_empty() {
                    Err(ParseError::NoSegmentation)
                } else {
                    // 尾巴：有一条读法把整段解完就没有解不动的键；都解不完时取第一条解出切分的读法
                    //（双拼方案下打全拼 `beijing`，主读法解不动、全拼读法解完）
                    let complete = sources.iter().any(|index| {
                        readings
                            .get(*index)
                            .is_some_and(|reading| reading.tail().is_empty())
                    });
                    let tail = if complete {
                        ""
                    } else {
                        sources
                            .first()
                            .and_then(|index| readings.get(*index))
                            .map_or("", |reading| reading.tail())
                    };
                    Ok((segs, tail, sources))
                }
            }
            (None, Some(tail)) if head_wins => {
                parser::segment(&keys[..tail.head_len]).map(|s| (s, "", vec![]))
            }
            _ => segment_longest_prefix(keys).map(|(s, tail)| {
                let sources = vec![0; s.len()];
                (s, tail, sources)
            }),
        };
        // 连第一个字母都切不动（`impor`）：拼音这边没戏，但英文词 / 补全、快捷候选还可以有
        let (segmentations, tail, sources) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                let mut items = Vec::new();
                self.insert_english(&mut items, true);
                self.insert_shortcuts(&mut items, keys);
                if items.is_empty() {
                    return Err(error);
                }
                return Ok(Query {
                    segmentations: Vec::new(),
                    spelled: 0,
                    candidates: CandidateList { items },
                    tail: keys.to_owned(),
                    text: self.composition.text().to_owned(),
                    cursor: self.composition.cursor(),
                    rest,
                    decoded_keys: self.shuangpin.is_some() || self.zhuyin,
                    shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
                    shuangpin_raw_window: self.shuangpin.is_some() && self.shuangpin_raw_window,
                    typed_display: decoded.as_ref().map(|d| d.marked()),
                    typed_case: None,
                    pinyin_choices: Vec::new(),
                    correction: None,
                    aux: None,
                    held: String::new(),
                    timings: Timings {
                        parse: start.elapsed(),
                        lookup: Duration::ZERO,
                        rank: Duration::ZERO,
                    },
                });
            }
        };
        // 拼音「不像话」时试拼写纠错；纠正生效则按纠正后的切分查词，原串只用来记学习与显示。
        // 混输时按最完整的那条切分判断，别被双拼读法留下的半截音节带偏（`women` 双拼读成
        // wo'me'n…，会让英文词 women 排到 我们 前面）
        let judged = segmentations
            .iter()
            .find(|segmentation| segmentation.syllables.iter().all(|x| x.complete))
            .or_else(|| segmentations.first());
        let unlikely =
            correction::unlikely_pinyin(judged, tail) || correction::trailing_single_letter(judged);
        let correction = if unlikely {
            self.active_correction(scope)
        } else {
            None
        };
        let (mut segmentations, tail, mut sources): (Vec<Segmentation>, &str, Vec<usize>) =
            match &correction {
                Some(c) => (vec![c.segmentation.clone()], "", vec![0]),
                None => (segmentations, tail, sources),
            };
        // 混输时同一段键有多条读法，切分的全拼长度在不同读法之间不可比（双拼展开与全拼不一定同长），
        // 覆盖量一律换算成敲了多少键来比；非混输仍是字母数
        let mixed_readings = readings.len() > 1;
        // 第 `index` 条切分的前 `syllables` 个音节吃掉多少覆盖量（混输按键，非混输按字母）。
        // `keys_for` 收的是全拼串的字节偏移，音节之间有 `'`，所以字母数要加上分隔符数量
        let span =
            |segmentations: &[Segmentation], sources: &[usize], index: usize, syllables: usize| {
                let segmentation = &segmentations[index];
                let letters: usize = segmentation
                    .syllables
                    .iter()
                    .take(syllables)
                    .map(|syllable| syllable.text.len())
                    .sum();
                match sources
                    .get(index)
                    .and_then(|source| readings.get(*source))
                    .filter(|_| mixed_readings)
                {
                    Some(reading) => reading.keys_for(letters + syllables.saturating_sub(1)),
                    None => letters,
                }
            };
        // 混输：主切分先看谁盖住的键多（只解出开头一个音节的读法不能当主切分，否则
        // `doubudui` 会由 `duo` 那半条读法当家、出 多 / 躲 / 朵），键数相同再比整句分数
        if mixed_readings && segmentations.len() > 1 {
            let mut best = 0;
            let mut best_keys = 0;
            let mut best_score = f64::NEG_INFINITY;
            for index in 0..segmentations.len() {
                let keys = span(
                    &segmentations,
                    &sources,
                    index,
                    segmentations[index].syllables.len(),
                );
                let score = self
                    .convert_sentence(&segmentations[index].patterns(), correction.is_none())
                    .map_or(f64::NEG_INFINITY, |conversion| conversion.score);
                if keys > best_keys || (keys == best_keys && score > best_score) {
                    best = index;
                    best_keys = keys;
                    best_score = score;
                }
            }
            segmentations.swap(0, best);
            // 切分与它来自哪条读法是一对，一起换，否则按索引取读法会错位（覆盖率换算错）
            sources.swap(0, best);
        }
        let parse = start.elapsed();
        // 注音每个音节敲的声调：词库不带调，查词照旧，只在排序里把对不上的字词往后放。纠错换了切分时不比
        let tones = match (&decoded, &correction) {
            (Some(d), None) => d.tones(),
            _ => Vec::new(),
        };

        let start = Instant::now();
        let mut scored = Vec::new();
        // 不同切分共享很多前缀（`zh g d o…` 的各种切法前几段一样），同一次查询里同一个模式只查一遍
        let mut memo: HashMap<String, Vec<Match<'_>>> = HashMap::new();
        // 有一种切法全是完整音节时，别的切法里拆开的音节（`shan` 拆成 `s h an` / `sha n`）多半不是用户的意思
        let clean = segmentations
            .iter()
            .any(|s| s.syllables.iter().all(|x| x.complete));
        // 盖住整段、不靠拆开音节读出的用户词（自造词、接受过的云端词）：(词, 拼音)
        let mut user_words: HashSet<(&str, &str)> = HashSet::new();
        for (index, segmentation) in segmentations.iter().enumerate() {
            let reading = sources
                .get(index)
                .and_then(|source| readings.get(*source))
                .filter(|_| mixed_readings);
            let covered_keys = |syllables: usize, letters: usize| match reading {
                Some(reading) => reading.keys_for(letters + syllables.saturating_sub(1)),
                None => letters,
            };
            // 混输：主切分之外的读法只留盖住整段的，解不完的（`doubudui` 的 duo 半条读法）整条跳过
            if mixed_readings
                && index != 0
                && reading.is_some_and(|reading| !reading.tail().is_empty())
            {
                continue;
            }
            let mut patterns = segmentation.patterns();
            let count = patterns.len();
            let last = &segmentation.syllables[count - 1];
            // 最后一个音节即使打完了也可能还没打完（`xia` 可能是 `xiang` 的前缀），按前缀查；双拼两键就是定局
            if last.complete && decoded.is_none() && parser::is_syllable_prefix(&last.text) {
                patterns[count - 1].complete = false;
            }
            // 词级候选只按敲的原样与模糊音查，敲错变体只进整句词图（它的候选从那边插进来）：
            // 词级排序把音节数对得上的排最前，敲错命中的词（`kaif` → 咖啡）会把更长的原样词挤到后面
            let expanded = self.fuzzy.expand(&patterns);
            let positions = expanded.positions();
            let abbreviated = abbreviated_count(&patterns);
            // 没有替代写法时每条命中都是敲的原音节，`penalty` 直接给 0（单字母简拼能命中几万条）
            let hits = self.lookup_all(&positions);
            let whole_split = clean && segmentation.syllables.iter().any(|x| !x.complete);
            if !whole_split && let Some(words) = self.learner.user_words() {
                user_words.extend(
                    words
                        .lookup_pattern_alt(&positions)
                        .into_iter()
                        .filter(|hit| hit.exact)
                        .map(|hit| (hit.text, hit.pinyin)),
                );
            }
            // 末音节读成它的模糊音写法（`zi` 开 z-zh 读成 zhi）也算打完的末音节：否则模糊音命中的常用字
            // 全排在原样读音的几百个生僻字之后，预选还会把它们砍掉（`li` 开 n-l 的 你 进不了前 500）
            let last_readings = if last.complete {
                self.fuzzy.readings(&last.text)
            } else {
                Vec::new()
            };
            scored.reserve(hits.len());
            for hit in hits {
                let last_syllable = hit.syllables().nth(count - 1);
                let typed_last = last.complete && last_syllable == Some(patterns[count - 1].text);
                let fuzzy_last = last.complete
                    && !typed_last
                    && last_syllable
                        .is_some_and(|syllable| last_readings.iter().any(|r| r == syllable));
                let mut penalty = expanded.penalty(hit.syllables());
                // `fan` 是 `fang` 的前缀，按前缀查到的 放 不带模糊音的扣分；当作 an-ang 的读法就该扣
                if fuzzy_last
                    && let Some(syllable) = last_syllable
                    && expanded.cost(count - 1, syllable) == 0.0
                {
                    penalty += FUZZY_PENALTY;
                }
                // 用户词至少算选过一次：简拼命中几万条时预选按词频会把它砍掉
                let user_word =
                    hit.exact && !whole_split && user_words.contains(&(hit.text, hit.pinyin));
                scored.push(Scored {
                    hit,
                    full_last: typed_last || fuzzy_last,
                    coverage: covered_keys(count, segmentation.letters()),
                    abbreviated,
                    weight: self.learner.weight(hit.text).max(u32::from(user_word)),
                    penalty,
                    tone_cost: crate::zhuyin::tone::cost(hit.text, hit.syllables(), &tones),
                    split: whole_split,
                });
            }
            // 输入的前缀也出候选（`kaifazhe` → 开发、开），否则长句没法逐词上屏。
            // 只收音节数正好等于前缀长度的词，更长的词会与输入后面的音节冲突。
            // 前缀不含最后一个位置，因此可复用上面的扩展结果。
            // 混输只从主切分产前缀：别的读法的短前缀（`banben` 的 ba）会把学习记录套到整段输入上，
            // 只读开头一个音节的词就这样越过盖住整段的词（#374）
            if !mixed_readings || index == 0 {
                for prefix_len in (1..count).rev() {
                    let prefix = &patterns[..prefix_len];
                    let prefix_letters: usize = prefix.iter().map(|p| p.text.len()).sum();
                    let hits = memo
                        .entry(pattern_key(prefix))
                        .or_insert_with(|| self.lookup_exact_all(&positions[..prefix_len]));
                    let abbreviated = abbreviated_count(prefix);
                    for hit in hits.iter().copied() {
                        scored.push(Scored {
                            // 对整个输入来说它不是精确命中，只是覆盖了前面一部分
                            hit: Match {
                                exact: false,
                                ..hit
                            },
                            full_last: true,
                            coverage: covered_keys(prefix_len, prefix_letters),
                            abbreviated,
                            weight: self.learner.weight(hit.text),
                            penalty: expanded.penalty(hit.syllables()),
                            tone_cost: crate::zhuyin::tone::cost(hit.text, hit.syllables(), &tones),
                            // 拆开的读法的前缀也算拆开的（`shi sha n` 的 是啥）；同一个词从全是完整音节的切法也命中时那条排在前面
                            split: whole_split,
                        });
                    }
                }
            }
        }
        let lookup = start.elapsed();

        // 盖住整段的用户词当作在这串键下选过一次（自造词选一次之后用简拼 `ss` 的 屎山 也排在前面），
        // 只取用得最多的几个：简拼能对上十几个自造词，全提上来又会把单字挤到几页之后
        let mut lifted: Vec<(&str, &str)> = user_words.into_iter().collect();
        lifted.sort_by_key(|&(text, pinyin)| (Reverse(self.learner.weight(text)), text, pinyin));
        lifted.truncate(ranking::USER_WORDS_BEFORE_SINGLES);

        let start = Instant::now();
        // 再往后翻也翻不到的候选不必再造：单字母简拼能命中两万个词，排完序只留前面这些。
        // 同输入串（候选覆盖的那段字母）下选过的优先；上下文是上一个上屏的词（句首为 None）：
        // `ba` 在「做了」后面出 吧、句首出 把
        let log_total = (self.total_frequency() as f64).max(1.0).ln();
        let letters = if mixed_readings {
            choice_key(keys, keys.len())
        } else {
            choice_key(scope, scope.len())
        };
        // 混输的 coverage 是敲的键数，选择记录按去 `'` 的字母串存：先把键偏移换算成字母偏移
        let letters_before_key: Vec<usize> = if mixed_readings {
            let mut counts = Vec::with_capacity(keys.len() + 1);
            let mut count = 0;
            counts.push(0);
            for c in keys.chars() {
                if c != '\'' {
                    count += 1;
                }
                counts.push(count);
            }
            counts
        } else {
            Vec::new()
        };
        // 纠错生效时覆盖的是纠正后的字母，换算回原串再查「这个输入串下选过什么」
        // 全拼选过、按简拼另记的选择（`^ss`），按覆盖到的长度先拼好键：几万条命中逐条拼键太慢
        let abbreviation_keys: Vec<String> = (0..=letters.len())
            .map(|end| format!("{ABBREVIATION_CHOICE_PREFIX}{}", &letters[..end]))
            .collect();
        // 真在这串键下的选择排在前面；全拼选过按简拼记的、当作选过的用户词排在没选过的词前面、真选过的后面
        let choice = |item: &Scored<'_>| {
            let covered = correction
                .as_ref()
                .map_or(item.coverage, |c| c.edit.to_original(item.coverage));
            let covered = if mixed_readings {
                letters_before_key
                    .get(covered)
                    .copied()
                    .unwrap_or(letters.len())
            } else {
                covered
            };
            let user_word =
                item.hit.exact && !item.split && lifted.contains(&(item.hit.text, item.hit.pinyin));
            let chosen = letters
                .get(..covered)
                .map_or(0, |input| self.learner.choice_weight(input, item.hit.text));
            let by_full_pinyin = abbreviation_keys
                .get(covered)
                .map_or(0, |key| self.learner.choice_weight(key, item.hit.text))
                .max(u32::from(user_word));
            chosen.saturating_mul(CHOICE_TIERS) + by_full_pinyin.min(CHOICE_TIERS - 1)
        };
        // 覆盖量的单位与 coverage 一致：混输按键算，非混输按字母算
        let whole = (0..segmentations.len())
            .map(|index| {
                span(
                    &segmentations,
                    &sources,
                    index,
                    segmentations[index].syllables.len(),
                )
            })
            .max()
            .unwrap_or(0);
        let first = segmentations
            .first()
            .map_or(0, |_| span(&segmentations, &sources, 0, 1));
        // 多音节输入时首音节的单字单独排、单独留名额：简拼 `ss` 读得出几万个两字词，合在一起预选会把单字全砍掉
        let mut singles = Vec::new();
        if whole > first {
            let (chars, rest): (Vec<_>, Vec<_>) = scored
                .into_iter()
                .partition(|item| item.coverage == first && item.hit.text.chars().count() == 1);
            singles = chars;
            scored = rest;
        }
        let context = |item: &Scored<'_>| (choice(item), self.word_log_prob(&item.hit, log_total));
        // 这串键下选过的词不进预选，并按主词库的词排：预选只看结构与词频，排序先比是不是主词库，
        // `ss` 选过几次的 屎山（知意词表里的词，低主词库一档）会排在几千个主词库两字词后面被截掉，轮不到按选择次数排
        let rank_keeping_chosen = |items: &mut Vec<Scored<'_>>, limit: usize| {
            let (mut chosen, rest): (Vec<_>, Vec<_>) = std::mem::take(items)
                .into_iter()
                .partition(|item| choice(item) > 0);
            for item in &mut chosen {
                item.hit.secondary = false;
            }
            *items = rest;
            ranking::rank(items, limit, context);
            if !chosen.is_empty() {
                items.extend(chosen);
                ranking::rank(items, limit, context);
            }
        };
        rank_keeping_chosen(&mut scored, MAX_CANDIDATES);
        rank_keeping_chosen(&mut singles, ranking::FIRST_SYLLABLE_CHARS_KEPT);
        // 同一个字别的切分也读得出（`kaif` 的 开 既是 kai 也是 k… a i f 的前缀），只留单字那条
        let chars: HashSet<&str> = singles.iter().map(|item| item.hit.text).collect();
        scored.retain(|item| !chars.contains(item.hit.text));
        scored.append(&mut singles);
        // 末尾只敲了个没打完的音节（`woganj` 的 j）时，盖住前面完整音节的词（我敢）也算整词；
        // 有一种全是完整音节的切法时（`mingtia` 读得成 ming ti a）末尾不算没打完
        let complete = if clean {
            whole
        } else {
            segmentations.first().map_or(whole, |segmentation| {
                let syllables = if segmentation.last_is_partial() {
                    segmentation.syllables.len().saturating_sub(1)
                } else {
                    segmentation.syllables.len()
                };
                span(&segmentations, &sources, 0, syllables)
            })
        };
        // 拆开音节的读法只有读音与敲的只差一个字母时才可能是要的（`nishui` 的 你说），比的是敲的同样长的开头
        let typed: String = segmentations
            .first()
            .map_or_else(String::new, |segmentation| {
                segmentation
                    .syllables
                    .iter()
                    .map(|x| x.text.as_str())
                    .collect()
            });
        let near_typed = |item: &Scored<'_>| {
            let reading: String = item.hit.syllables().collect();
            let typed = typed
                .get(..reading.len().min(typed.len()))
                .unwrap_or(&typed);
            edit_distance(&reading, typed) <= 1
        };
        ranking::arrange(
            &mut scored,
            whole,
            complete,
            first,
            clean,
            |item| choice(item) > 0,
            near_typed,
        );
        // 辅码态：词库候选按码段**反向**过滤（逐个问「有没有以码段开头的码」），无码词直接隐藏；
        // 命中的按「完全匹配码 > 码长降序 > 原词频序」重排（stable sort 保住 rank 排好的原序）。
        // 码段为空（刚敲下触发键）时不过滤，候选与纯拼音态一模一样。
        let aux_code = self.aux_filter();
        let mut items: Vec<Candidate> = Vec::with_capacity(scored.len());
        match aux_code {
            // 没在筛码：首条码只在显示开关开着或已在辅码态时挂，否则不逐候选查码
            None => items.extend(scored.into_iter().map(|item| {
                let first = if self.aux_show || self.aux_code.is_some() {
                    self.matching_code(item.hit.text, "")
                } else {
                    None
                };
                chinese_candidate(&item, first.as_deref())
            })),
            Some(code) => {
                let mut kept: Vec<(usize, Cow<'_, str>)> = scored
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        self.matching_code(item.hit.text, code)
                            .map(|hit| (index, hit))
                    })
                    .collect();
                kept.sort_by(|a, b| {
                    (b.1.len() == code.len())
                        .cmp(&(a.1.len() == code.len()))
                        .then_with(|| b.1.len().cmp(&a.1.len()))
                });
                items.extend(
                    kept.into_iter()
                        .map(|(index, hit)| chinese_candidate(&scored[index], Some(&hit))),
                );
            }
        }
        // 附加候选（英文尾段与补全、快捷、整句、emoji）都没有码：辅码筛词时一律不出
        if aux_code.is_none() {
            // 双拼 / 注音末尾落单的一键是「还没敲完」，不是拼错（`veg` → zhe g…）；词级已有整词
            // 逐音节对上全部输入（`veg` → 这个）时按中文读法：英文候选不排第一、也不给补全，
            // 空格不会把整串当英文上屏。全拼的落单字母另有「可能敲反了」的意思，照旧（见 correction）
            let covered_by_word = decoded.is_some()
                && correction::trailing_single_letter(segmentations.first())
                && segmentations.first().is_some_and(|best| {
                    let patterns = best.patterns();
                    items.iter().any(|c| {
                        c.kind == CandidateKind::Chinese
                            && c.syllables.len() == patterns.len()
                            && c.syllables
                                .iter()
                                .zip(&patterns)
                                .all(|(syllable, pattern)| pattern.accepts(syllable))
                    })
                });
            let english_unlikely = unlikely && !covered_by_word;
            // 中文优先：整句先进去占第一，英文词紧跟其后（第二）；关掉时英文词先进、整句排在开头的英文后面
            if self.chinese_first {
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
                self.insert_english(&mut items, english_unlikely);
            } else {
                self.insert_english(&mut items, english_unlikely);
                self.insert_sentence(
                    &mut items,
                    &segmentations,
                    correction.is_none(),
                    english_tail.as_ref().filter(|_| correction.is_none()),
                    head_wins,
                );
            }
            // 整句也是中文：英文候选按 `english_min_position` 让位（缺省不限制）
            self.place_english(&mut items);
            // 快捷候选按敲的键认（`rq` 日期），双拼下也是
            self.insert_shortcuts(&mut items, keys);
            self.insert_emoji(&mut items);
            self.insert_keyword_shortcuts(&mut items);
        }
        let rank = start.elapsed();

        // 按头段算时英文尾段不参与拼音候选，显示上跟在切分后面：`wo'xiang'xue'hao'rust`
        let tail = english_tail
            .as_ref()
            .filter(|_| head_wins)
            .map_or(tail, |t| &keys[t.head_len..]);
        let typed_display = decoded.as_ref().map(|d| d.marked());
        let typed_case =
            (decoded.is_none() && correction.is_none() && self.composition.has_shifted())
                .then(|| self.composition.typed_scope());
        Ok(Query {
            segmentations,
            spelled: 0,
            candidates: CandidateList { items },
            tail: tail.to_owned(),
            text: self.composition.text().to_owned(),
            cursor: self.composition.cursor(),
            rest,
            decoded_keys: self.shuangpin.is_some() || self.zhuyin,
            shuangpin_raw_preedit: self.shuangpin.is_some() && self.shuangpin_raw_preedit,
            shuangpin_raw_window: self.shuangpin.is_some() && self.shuangpin_raw_window,
            typed_display,
            typed_case,
            pinyin_choices: Vec::new(),
            correction,
            aux: self.aux_segment(),
            held: String::new(),
            timings: Timings {
                parse,
                lookup,
                rank,
            },
        })
    }
}
