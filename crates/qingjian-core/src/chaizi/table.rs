//! 拆字表 + 笔顺表 + 部件别名，与它们的索引。
//!
//! 三张表都是 `&'static str`（`include_str!`），解析时只切出切片，用到时才在后面按 `'` / `,` 切。

use std::collections::{BTreeMap, HashMap};

use super::align;

/// 一个候选的档位：全读音最自然，混输次之，全笔画再次，倒着的笔画最后。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// 每个部件都按读音打。
    Reading,
    /// 读音与笔画混着打。
    Mixed,
    /// 全是笔画。
    Stroke,
    /// 反过来打的笔画（倒笔画）。
    Reversed,
}

/// 问字模式的一个命中。
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    /// 命中的字。
    pub ch: char,

    /// 这个字最常见的读音（不带声调，Core 出口再标）。
    pub reading: &'static str,

    /// 档位。
    pub tier: Tier,

    /// 输入被吃掉了几个字节：越接近整段越像用户要的字。
    pub consumed: usize,

    /// 整字笔顺还差几笔（拆法对齐恒为 0）：打得刚好完整的那几个字排前面。
    pub remaining: usize,

    /// 输入刚好把这条拆法（或整字笔顺）走完：打完了的字排在「还差一点」的前面。
    pub complete: bool,

    /// 来自整字笔顺表（而不是部件拆法）：整字笔顺对上的最像用户要的字，倒着打也一样。
    pub whole: bool,

    /// 这条拆法来自繁体字典（瞭 的 目尞 落到了 了 上）：按设置的简繁排在简体拆法之后。
    pub traditional: bool,
}

/// 一种拆法：部件连写（`木木`）与每个部件的读音（`mu'mu`，部件内多读音用 `,`）。
pub(super) struct Split {
    pub entry: u32,
    pub parts: &'static str,
    pub readings: &'static str,

    /// 来自繁体字典：简体设置下这些拆法出的字排在后面。
    pub traditional: bool,
}

/// 部件别名：额外的读音（囗 认 kou）与笔顺（CNS 里没有的笔画符号）。
struct Alias {
    readings: Box<[&'static str]>,
    strokes: Option<&'static str>,
}

pub struct ChaiziTable {
    /// 字与它最常见的读音，按字频从高到低；下标即字频序。
    entries: Vec<(char, &'static str)>,

    /// 全部拆法。
    splits: Vec<Split>,

    /// 首字符 → 拆法下标：读音的首字母或首笔，用来把候选缩到一个桶。
    by_first: HashMap<char, Vec<u32>>,

    /// 部件读音连写（不带分隔符）→ 字的下标，升序。老的「整串读音」口径。
    joined: BTreeMap<String, Vec<u32>>,

    /// 字 → 笔顺（`h` / `s` / `p` / `n` / `z`）。
    strokes: HashMap<char, &'static str>,

    /// 字符 → 别名。
    parts: HashMap<char, Alias>,

    /// 笔顺表按字频序：`(字, 读音, 笔顺, 字频序)`，纯笔画的候选从这儿出。
    stroke_order: Vec<(char, &'static str, &'static str, u32)>,
}

impl ChaiziTable {
    /// 解析三张表。行首 `#` 是注释，空行跳过。
    pub fn parse(chaizi: &'static str, stroke: &'static str, parts: &'static str) -> Self {
        let mut entries: Vec<(char, &'static str)> = Vec::new();
        let mut index_of: HashMap<char, u32> = HashMap::new();
        let mut splits: Vec<Split> = Vec::new();
        let mut joined: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for line in chaizi.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.split('\t');
            let (Some(ch), Some(reading), Some(parts), Some(readings)) = (
                fields.next().and_then(single_char),
                fields.next(),
                fields.next(),
                fields.next(),
            ) else {
                continue;
            };
            let entry = match index_of.get(&ch) {
                Some(&known) => known,
                None => {
                    index_of.insert(ch, entries.len() as u32);
                    entries.push((ch, reading.split(' ').next().unwrap_or_default()));
                    entries.len() as u32 - 1
                }
            };
            // 第五列是每条拆法的来源（jt / t）：老表没有这一列，按简体算
            let origin = fields.next().unwrap_or_default();
            for (at, (one, list)) in parts.split(' ').zip(readings.split(' ')).enumerate() {
                if one.is_empty() || one.chars().count() != list.split('\'').count() {
                    continue;
                }
                splits.push(Split {
                    entry,
                    parts: one,
                    readings: list,
                    traditional: origin.split(' ').nth(at) == Some("ft"),
                });
                joined.entry(joined_key(list)).or_default().push(entry);
            }
        }
        for rows in joined.values_mut() {
            rows.sort_unstable();
            rows.dedup();
        }

        let mut strokes: HashMap<char, &'static str> = HashMap::new();
        let mut stroke_order: Vec<(char, &'static str, &'static str, u32)> = Vec::new();
        let mut unknown = 0u32;
        for line in stroke.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.split('\t');
            let (Some(ch), Some(sequence)) = (fields.next().and_then(single_char), fields.next())
            else {
                continue;
            };
            strokes.insert(ch, sequence);
            let rank = match index_of.get(&ch) {
                Some(&known) => (known, entries[known as usize].1),
                None => {
                    unknown += 1;
                    (entries.len() as u32 + unknown, "")
                }
            };
            stroke_order.push((ch, rank.1, sequence, rank.0));
        }
        stroke_order.sort_unstable_by_key(|(_, _, _, rank)| *rank);

        let mut alias: HashMap<char, Alias> = HashMap::new();
        for line in parts.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.split('\t');
            let Some(ch) = fields.next().and_then(single_char) else {
                continue;
            };
            let readings: Box<[&'static str]> = fields
                .next()
                .unwrap_or_default()
                .split(' ')
                .filter(|one| !one.is_empty())
                .collect();
            let sequence: String = fields
                .next()
                .unwrap_or_default()
                .split(' ')
                .filter(|one| !one.is_empty())
                .collect();
            let strokes = (!sequence.is_empty())
                .then(|| &*Box::leak(sequence.into_boxed_str()) as &'static str);
            alias.insert(ch, Alias { readings, strokes });
        }

        let mut by_first: HashMap<char, Vec<u32>> = HashMap::new();
        for (index, split) in splits.iter().enumerate() {
            let Some(part) = split.parts.chars().next() else {
                continue;
            };
            let mut seen: Vec<char> = Vec::new();
            let push =
                |lead: char, seen: &mut Vec<char>, by_first: &mut HashMap<char, Vec<u32>>| {
                    if !seen.contains(&lead) {
                        seen.push(lead);
                        by_first.entry(lead).or_default().push(index as u32);
                    }
                };
            for reading in split
                .readings
                .split('\'')
                .next()
                .unwrap_or_default()
                .split(',')
            {
                if let Some(lead) = reading.chars().next() {
                    push(lead, &mut seen, &mut by_first);
                }
            }
            if let Some(one) = alias.get(&part) {
                for reading in &*one.readings {
                    if let Some(lead) = reading.chars().next() {
                        push(lead, &mut seen, &mut by_first);
                    }
                }
            }
            // 第一个部件也可能按笔顺打（`uphspnkou` = 禾的笔顺 + 口的读音）
            let sequence = strokes
                .get(&part)
                .copied()
                .or_else(|| alias.get(&part).and_then(|one| one.strokes));
            if let Some(lead) = sequence.and_then(|sequence| sequence.chars().next()) {
                push(lead, &mut seen, &mut by_first);
            }
        }

        Self {
            entries,
            splits,
            by_first,
            joined,
            strokes,
            parts: alias,
            stroke_order,
        }
    }

    /// 按 `body` 找字，见 [`super::lookup`]。
    pub fn lookup(&self, body: &str, limit: usize) -> Vec<Hit> {
        if body.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut scored: Vec<(Hit, u32)> = Vec::new();
        if let Some(bucket) = body
            .chars()
            .next()
            .and_then(|lead| self.by_first.get(&lead))
        {
            for index in bucket {
                let split = &self.splits[*index as usize];
                if let Some(hit) = align::align(self, split, body) {
                    scored.push((hit, split.entry));
                }
            }
        }
        // 整串全是笔画键：再按整字笔顺找一遍，正序在前、倒序在后
        if body.chars().all(is_stroke) {
            self.collect_strokes(body, Tier::Stroke, &mut scored);
            let reversed: String = body.chars().rev().collect();
            if reversed != body {
                self.collect_strokes(&reversed, Tier::Reversed, &mut scored);
            }
        }
        scored.sort_by_key(sort_key);
        let mut seen: Vec<u32> = Vec::new();
        let mut hits = Vec::new();
        for (hit, entry) in scored {
            if seen.contains(&entry) {
                continue;
            }
            // 只吃掉小半段的匹配是「另一条路的巧合」：`usangemu`（三个木是什么字）不该被
            // 「桑」的读音 `sang` 拆出一个候选来，那是要问云端的整句问题。
            // 四分之三以上（`umum` 要吃 `mumu` 的 4/5）仍算「敲到一半」，照常出。
            // 吃得太少的丢掉，免得 `uface`、`usangemu` 这类输入冒出几个不相干的字。
            // 读数打一半、笔画打一半（``ushangxia`` 的 卡 是 上卜，只吃了 5/8）都是常用姿势，
            // 门槛给得松；带笔画的匹配容易凑（一个 `s` 就能对上很多字），门槛按 3/4 卡。
            let (needed, total) = match hit.tier {
                Tier::Reading => (3, 5),
                Tier::Mixed | Tier::Stroke | Tier::Reversed => (3, 4),
            };
            if hit.consumed < body.len() && hit.consumed * total <= body.len() * needed {
                continue;
            }
            seen.push(entry);
            hits.push(hit);
            if hits.len() >= limit {
                break;
            }
        }
        hits
    }

    /// 整字笔顺以 `body` 开头的字。
    fn collect_strokes(&self, body: &str, tier: Tier, out: &mut Vec<(Hit, u32)>) {
        for (ch, reading, sequence, rank) in &self.stroke_order {
            if sequence.starts_with(body) {
                out.push((
                    Hit {
                        ch: *ch,
                        reading,
                        tier,
                        consumed: body.len(),
                        remaining: sequence.chars().count() - body.chars().count(),
                        complete: sequence.len() == body.len(),
                        whole: true,
                        traditional: false,
                    },
                    *rank,
                ));
            }
        }
    }

    /// 老的「整串读音」口径：`umumu` → 部件读音连写 `mumu` 的整串或前缀。
    pub fn has_reading_match(&self, keys: &str) -> bool {
        if keys.is_empty() {
            return false;
        }
        if self.joined.contains_key(keys) {
            return true;
        }
        self.joined
            .range::<str, _>((std::ops::Bound::Excluded(keys), std::ops::Bound::Unbounded))
            .next()
            .is_some_and(|(key, _)| key.starts_with(keys))
    }

    /// 字本身。
    pub(super) fn char_of(&self, index: u32) -> char {
        self.entries[index as usize].0
    }

    /// 字的读音。
    pub(super) fn reading_of(&self, index: u32) -> &'static str {
        self.entries[index as usize].1
    }

    /// 一个部件的笔顺：先查笔顺表，再查部件别名的补录。
    pub(super) fn strokes_of(&self, ch: char) -> Option<&'static str> {
        self.strokes
            .get(&ch)
            .copied()
            .or_else(|| self.parts.get(&ch).and_then(|alias| alias.strokes))
    }

    /// 一个部件额外的读音（`囗` 认 `kou`）。
    pub(super) fn alias_readings(&self, ch: char) -> &[&'static str] {
        self.parts
            .get(&ch)
            .map(|alias| &*alias.readings)
            .unwrap_or_default()
    }
}

/// 排序键：吃得最多 → 整字笔顺 → 走完没走完 → 档位 → 还差几笔 → 字频。
///
/// 档位（全读音 > 混合 > 全笔画 > 倒序）排在「还差几笔」之前：打 uhsp 三笔时，木（正序、差一笔）
/// 与 片（倒序、也差一笔）不该让倒着的那个插到正序前面。
fn sort_key(scored: &(Hit, u32)) -> (std::cmp::Reverse<usize>, bool, usize, Tier, bool, bool, u32) {
    (
        std::cmp::Reverse(scored.0.consumed),
        !scored.0.whole,
        // 还差几笔在档位之前：uhzs 倒着打的 口 是三笔全对（remaining 0），
        // 屯 是正序吃了三笔还差一笔——不能让正序把倒着打完整对上的挤出 50 条。
        scored.0.remaining,
        // 档位在「走完没走完」之前：ushui 想找三点水的字，炭（s 当一笔竖 + hui 当灰）
        // 虽然把两个部件都走完了，也不该压过按读音出的 没——整段读得通时读音优先。
        scored.0.tier,
        !scored.0.complete,
        // 繁体字典来的、又没走完的拆法往后挪一位：umu 的 了（繁体 瞭 的 目尞，只走到 目）
        // 不该压过 本。走完的不分（uzouyan 用 辶言 找到 这/這，两个部件都走到，按字频排）。
        // 方向固定，不看简繁设置——设置只管出口换不换字形，问字怎么排序是另一件事。
        scored.0.traditional && !scored.0.complete,
        scored.1,
    )
}

/// 是不是笔画键。
pub(super) fn is_stroke(ch: char) -> bool {
    matches!(ch, 'h' | 's' | 'p' | 'n' | 'z')
}

/// 把部件读音列表连起来：`mu'mu` → `mumu`。多读音的部件取第一个。
fn joined_key(list: &str) -> String {
    if !list.contains('\'') && !list.contains(',') {
        return list.to_owned();
    }
    list.split('\'')
        .filter_map(|one| one.split(',').next())
        .collect()
}

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}
