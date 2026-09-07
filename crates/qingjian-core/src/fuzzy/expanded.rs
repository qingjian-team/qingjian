use qingjian_dictionary::SyllablePattern;

/// 一串模式扩展后的结果：每个位置若干写法，第一种是用户敲的（代价 0），其余是模糊音（扣 ln 2）
/// 或敲错变体（按类别与个人敲错表定代价）。拥有字符串，借出 [`SyllablePattern`]。
#[derive(Debug, Clone, Default)]
pub struct Expanded {
    /// 每个位置的 (写法, 代价) 与「是否完整音节」（同一位置的写法完整性相同）。
    positions: Vec<(Vec<(String, f64)>, bool)>,

    /// 有没有任何一个位置带了敲的原样以外的写法：没有就不必逐条命中算代价。
    alternatives: bool,
}

impl Expanded {
    pub(super) fn new(positions: impl IntoIterator<Item = (Vec<(String, f64)>, bool)>) -> Self {
        let positions: Vec<_> = positions.into_iter().collect();
        let alternatives = positions.iter().any(|(forms, _)| forms.len() > 1);
        Self {
            positions,
            alternatives,
        }
    }

    /// 给 `index` 位置加一种写法；已有同样写法时只留代价低的那个。不完整的位置（简拼、前缀）不加。
    pub fn push_alternative(&mut self, index: usize, text: &str, cost: f64) {
        let Some((forms, complete)) = self.positions.get_mut(index) else {
            return;
        };
        if !*complete {
            return;
        }
        match forms.iter_mut().find(|(t, _)| t == text) {
            Some(existing) => existing.1 = existing.1.min(cost),
            None => forms.push((text.to_owned(), cost)),
        }
        self.alternatives = true;
    }

    /// 词库多写法查询要的形状。
    pub fn positions(&self) -> Vec<Vec<SyllablePattern<'_>>> {
        self.positions
            .iter()
            .map(|(forms, complete)| {
                forms
                    .iter()
                    .map(|(text, _)| SyllablePattern {
                        text,
                        complete: *complete,
                    })
                    .collect()
            })
            .collect()
    }

    /// 有没有敲的原样以外的写法。
    pub fn has_alternatives(&self) -> bool {
        self.alternatives
    }

    /// `index` 位置命中音节 `syllable` 的代价：敲的原样 0，模糊音 / 敲错变体按写法记的代价；哪种写法都对不上（不该发生）算 0。
    pub fn cost(&self, index: usize, syllable: &str) -> f64 {
        let Some((forms, complete)) = self.positions.get(index) else {
            return 0.0;
        };
        forms
            .iter()
            .find(|(text, _)| {
                if *complete {
                    text == syllable
                } else {
                    syllable.starts_with(text.as_str())
                }
            })
            .map_or(0.0, |(_, cost)| *cost)
    }

    /// 一条命中的总代价：各位置代价之和。没有任何替代写法时直接 0。
    pub fn penalty<'a>(&self, syllables: impl IntoIterator<Item = &'a str>) -> f64 {
        if !self.alternatives {
            return 0.0;
        }
        syllables
            .into_iter()
            .enumerate()
            .map(|(index, syllable)| self.cost(index, syllable))
            .sum()
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_follow_the_matched_form() {
        let mut expanded = Expanded::new([
            (vec![("mei".to_owned(), 0.0)], true),
            (
                vec![("gan".to_owned(), 0.0), ("gang".to_owned(), 0.7)],
                true,
            ),
            (vec![("x".to_owned(), 0.0)], false),
        ]);
        expanded.push_alternative(1, "guan", 4.5);
        expanded.push_alternative(1, "gang", 4.5); // 已有更便宜的模糊音写法，保留 0.7
        expanded.push_alternative(2, "xi", 4.0); // 不完整的位置不加
        assert!(expanded.has_alternatives());
        assert_eq!(expanded.penalty(["mei", "gan", "xi"]), 0.0);
        assert_eq!(expanded.penalty(["mei", "guan", "xia"]), 4.5);
        assert_eq!(expanded.penalty(["mei", "gang", "xi"]), 0.7);
        assert_eq!(expanded.positions()[1].len(), 3);
        let plain = Expanded::new([(vec![("ni".to_owned(), 0.0)], true)]);
        assert!(!plain.has_alternatives());
        assert_eq!(plain.penalty(["mi"]), 0.0);
    }
}
