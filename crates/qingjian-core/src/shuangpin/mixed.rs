//! 双拼与全拼共用音节边界的解码；每个单元保存原始键，供逐词上屏换算消耗。

use std::cmp::Reverse;

use super::{Decoded, Scheme, Unit};
use crate::parser;

/// 每个位置最多保留的读法，避免混输歧义随输入长度指数增长。
const MIXED_BEAM: usize = 8;

impl Scheme {
    /// 同时尝试双拼两键和全拼音节；原来的纯双拼读法始终保留在第一条。
    pub fn decode_mixed(self, keys: &str) -> Vec<Decoded> {
        let original = self.decode(keys);
        if !keys.is_ascii() || keys.is_empty() {
            return vec![original];
        }
        let n = keys.len();
        let mut paths: Vec<Vec<Vec<Unit>>> = vec![Vec::new(); n + 1];
        paths[0].push(Vec::new());
        for start in 0..n {
            prune(&mut paths[start]);
            if paths[start].is_empty() {
                continue;
            }
            let rest = &keys[start..];
            let mut tokens = Vec::new();
            if rest.starts_with('\'') {
                tokens.push(Unit::separator());
            } else {
                let mut chars = rest.chars();
                let first = chars.next().expect("非空剩余串");
                if let Some(second) = chars.next().filter(|c| *c != '\'')
                    && let Some(pinyin) = self.syllable(first, second)
                {
                    tokens.push(Unit {
                        keys: rest[..2].to_owned(),
                        pinyin,
                        complete: true,
                    });
                }
                for len in (1..=rest.len().min(parser::MAX_SYLLABLE_LEN)).rev() {
                    let text = &rest[..len];
                    if parser::is_syllable(text) {
                        tokens.push(Unit {
                            keys: text.to_owned(),
                            pinyin: text.to_owned(),
                            complete: true,
                        });
                    }
                }
                // 未打完的全拼只允许出现在音节末尾，不能把内部字母当简拼任意拆开。
                let chunk_len = rest.find('\'').unwrap_or(rest.len());
                let chunk = &rest[..chunk_len];
                if parser::is_syllable_prefix(chunk) {
                    tokens.push(Unit {
                        keys: chunk.to_owned(),
                        pinyin: chunk.to_owned(),
                        complete: false,
                    });
                }
                if chunk_len == 1
                    && let Some(pinyin) = self.partial(first)
                {
                    tokens.push(Unit {
                        keys: first.to_string(),
                        pinyin,
                        complete: false,
                    });
                }
            }
            for unit in tokens {
                let end = start + unit.keys.len();
                let extended: Vec<_> = paths[start]
                    .iter()
                    .map(|base| {
                        let mut units = base.clone();
                        units.push(unit.clone());
                        units
                    })
                    .collect();
                paths[end].extend(extended);
            }
        }
        prune(&mut paths[n]);
        let mut decoded = vec![original];
        for units in std::mem::take(&mut paths[n]) {
            let next = Decoded::new(units, String::new());
            if !decoded.contains(&next) {
                decoded.push(next);
            }
        }
        decoded
    }
}

fn prune(paths: &mut Vec<Vec<Unit>>) {
    paths.sort_by_cached_key(|units| {
        (
            units.iter().filter(|u| !u.is_separator()).count(),
            units
                .iter()
                .filter(|u| !u.complete && !u.is_separator())
                .count(),
            units
                .iter()
                .map(|u| Reverse(u.keys.len()))
                .collect::<Vec<_>>(),
        )
    });
    paths.dedup();
    paths.truncate(MIXED_BEAM);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_both_spellings_and_mixed_syllables() {
        for keys in [
            "nihc",
            "nihao",
            "ni'hao",
            "nihcvsgo",
            "nihaovsgo",
            "nihczhongguo",
        ] {
            let want = if keys.len() <= 6 {
                "ni'hao"
            } else {
                "ni'hao'zhong'guo"
            };
            assert!(
                Scheme::Xiaohe
                    .decode_mixed(keys)
                    .iter()
                    .any(|d| d.pinyin() == want),
                "{keys}"
            );
        }
    }

    #[test]
    fn preserves_original_and_raw_key_consumption() {
        let paths = Scheme::Xiaohe.decode_mixed("kaifave");
        assert_eq!(paths[0], Scheme::Xiaohe.decode("kaifave"));
        let decoded = paths.iter().find(|d| d.pinyin() == "kai'fa'zhe").unwrap();
        assert_eq!(decoded.keys_for(6), 5);
        assert_eq!(decoded.keys_for(10), 7);
        let paths = Scheme::Xiaohe.decode_mixed("kai'fa've");
        let decoded = paths.iter().find(|d| d.pinyin() == "kai'fa'zhe").unwrap();
        assert_eq!(decoded.keys_for(6), 7);
    }

    #[test]
    fn every_scheme_accepts_full_pinyin_syllables() {
        for scheme in Scheme::ALL {
            for syllable in parser::SYLLABLES {
                assert!(
                    scheme
                        .decode_mixed(syllable)
                        .iter()
                        .any(|d| d.pinyin() == *syllable && d.is_complete()),
                    "{scheme}: {syllable}"
                );
            }
        }
    }

    #[test]
    fn supports_partial_full_pinyin_and_bounds_ambiguity() {
        assert!(
            Scheme::Xiaohe
                .decode_mixed("zhon")
                .iter()
                .any(|d| d.pinyin() == "zhon" && !d.is_complete())
        );
        assert!(Scheme::Xiaohe.decode_mixed(&"xian".repeat(40)).len() <= MIXED_BEAM + 1);
        assert_eq!(Scheme::Xiaohe.decode_mixed("中文")[0].tail(), "中文");
    }
}
