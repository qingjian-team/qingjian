//! 键串与候选音节的对齐：算出候选吃掉多少键，以及哪几处是敲错变体。

use std::collections::HashSet;

use super::Engine;
use crate::correction::typo;
use crate::engine::alignment::Alignment;
use crate::parser;

impl Engine {
    /// 候选的音节逐个对到输入上：每个音节按原样 / 规范写法、模糊音或敲错变体（长的在前）、没打完的前缀消耗输入，
    /// 先找能把**每个音节都对上**的对齐（带回溯：`pingyin` 对 拼音 时 `pin` 原样只吃三个字母会剩下 `gyin`，
    /// 退回来按敲错变体 `ping` → `pin` 吃四个；`ljchlsg` 对 蜡炬成灰泪始干 时 `ch` 当 成 的声母会剩下 灰 对不上，
    /// 退回来只吃 `c`），找不到才退回逐个贪心对、对不上的地方停下。
    pub(in crate::engine) fn align(&self, input: &str, syllables: &[String]) -> Alignment {
        let mut typos = Vec::new();
        let mut failed = HashSet::new();
        self.align_full(input, 0, syllables, &mut typos, &mut failed)
            .unwrap_or_else(|| self.align_greedy(input, syllables))
    }

    /// 从 `pos` 起把剩下的音节全对上的第一种对齐（按每步的优先级深度优先）；对不上返回 `None`。
    /// `failed` 记下已经试过对不上的（输入位置, 剩几个音节），简拼每个音节能吃好几种前缀，不记会指数回溯。
    fn align_full(
        &self,
        input: &str,
        pos: usize,
        syllables: &[String],
        typos: &mut Vec<(String, String)>,
        failed: &mut HashSet<(usize, usize)>,
    ) -> Option<Alignment> {
        let Some((syllable, remaining)) = syllables.split_first() else {
            return Some(Alignment {
                consumed: pos,
                typos: typos.clone(),
            });
        };
        if failed.contains(&(pos, syllables.len())) {
            return None;
        }
        let (rest, start) = self.rest_at(input, pos);
        for (len, typo) in self.syllable_steps(rest, syllable) {
            // 消耗完输入后还有音节没对：不算全对上（候选比敲的长）
            if start + len == input.len() && !remaining.is_empty() {
                continue;
            }
            if typo {
                typos.push((rest[..len].to_owned(), syllable.clone()));
            }
            let found = self.align_full(input, start + len, remaining, typos, failed);
            if typo {
                typos.pop();
            }
            if found.is_some() {
                return found;
            }
        }
        failed.insert((pos, syllables.len()));
        None
    }

    /// 逐个贪心对：每个音节取第一种能对上的消耗，都对不上就取公共前缀，公共前缀也没有就停。
    fn align_greedy(&self, input: &str, syllables: &[String]) -> Alignment {
        let mut alignment = Alignment::default();
        let mut pos = 0;
        for syllable in syllables {
            let (rest, start) = self.rest_at(input, pos);
            if let Some((len, typo)) = self.syllable_steps(rest, syllable).into_iter().next() {
                if typo {
                    alignment
                        .typos
                        .push((rest[..len].to_owned(), syllable.clone()));
                }
                pos = start + len;
                continue;
            }
            let common = syllable
                .bytes()
                .zip(rest.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            if common == 0 {
                break;
            }
            pos = start + common;
        }
        alignment.consumed = pos;
        alignment
    }

    /// `pos` 处这个音节能看到的输入段（到下一个 `'` 为止）与它的起点（跳过开头的 `'`）。
    /// 缓冲区本身以 `'` 开头（`shan'gou` 选了 闪 剩下 `'gou`）时也要跳过，否则一个键都吃不掉。
    fn rest_at<'a>(&self, input: &'a str, pos: usize) -> (&'a str, usize) {
        let start = if input[pos..].starts_with('\'') {
            pos + 1
        } else {
            pos
        };
        let rest = &input[start..];
        let rest = &rest[..rest.find('\'').unwrap_or(rest.len())];
        (rest, start)
    }

    /// 一个音节可以怎么消耗输入段 `rest`：(消耗字节数, 是否靠敲错变体)，按优先级排：
    /// 整段的模糊音变体、原样或规范写法（`lue` / `lve`）、模糊音或敲错变体（长的在前）、整段是这个音节没打完的前缀，
    /// 最后是段里开头一截是这个音节的前缀（简拼，长的在前）。
    fn syllable_steps(&self, rest: &str, syllable: &str) -> Vec<(usize, bool)> {
        let mut steps = Vec::new();
        // 整段就是这个音节的模糊音变体时先吃满：`qing` 选 勤（qin）不能只吃 `qin` 把 `g` 留下
        if self.fuzzy.is_variant(rest, syllable) {
            steps.push((rest.len(), false));
        }
        let exact = rest.get(..syllable.len()).is_some_and(|typed| {
            typed == syllable
                || qingjian_dictionary::canonical_syllable(typed)
                    == qingjian_dictionary::canonical_syllable(syllable)
        });
        if exact {
            steps.push((syllable.len(), false));
        }
        // 没打完排在变体前面：`shijia` 选 时间 是 jian 没敲完，不是把 jian 敲成了 jia
        if !rest.is_empty() && rest.len() < syllable.len() && syllable.starts_with(rest) {
            steps.push((rest.len(), false));
        }
        for len in (1..=rest.len().min(parser::MAX_SYLLABLE_LEN)).rev() {
            if exact && len == syllable.len() {
                continue;
            }
            let typed = &rest[..len];
            if self.fuzzy.is_variant(typed, syllable) {
                steps.push((len, false));
            } else if typo::is_variant(typed, syllable) {
                steps.push((len, true));
            }
        }
        for len in (1..rest.len().min(syllable.len())).rev() {
            if syllable.starts_with(&rest[..len]) && !steps.iter().any(|&(l, _)| l == len) {
                steps.push((len, false));
            }
        }
        steps
    }
}
