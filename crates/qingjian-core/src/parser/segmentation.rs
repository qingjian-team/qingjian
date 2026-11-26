use std::fmt;

use qingjian_dictionary::{SyllablePattern, canonical_syllable};

use super::umlaut_spelling;

/// 切分出的一个音节。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Syllable {
    /// 用户敲的字母。
    pub text: String,

    /// 是完整音节；否则是声母（简拼 `k`）或未打完的前缀（`zho`）。
    pub complete: bool,

    /// 零声母简拼：这个键**代表**的音节（敲 `a`，代表 `an`）。
    ///
    /// 标准简拼规则里零声母音节用韵头的单韵母代表 —— `a` 代表 `a` / `ai` / `an` / `ang` / `ao`。
    /// `text` 保持敲的字母（`a`）不变，`implied` 只在查询时用，而且是完整音节：
    /// `aquan` 才有机会切出 `a(an)` + `quan` → 安全。
    pub implied: Option<String>,

    /// 这条简写是用户自己用隔音符封出来的一段（`e'liao` 的 `e`），不是从长串里猜的（`aquan` 的 `a`）。
    ///
    /// 两种在排序上待遇不同：猜出来的简写要排在同样的全拼切分后面（别把 `人格面具` 挤成 `仍恩面具`），
    /// 而 `e'liao` 这种打法本身就是简写 —— 用户敲了隔音符就是在说「这是一个独立的音节」，
    /// 该让 `饵料`（`e` = `er`）排在 `饿殍`（`e` = `e`）前面。
    pub implied_separate: bool,
}

impl Syllable {
    pub fn complete(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: true,
            implied: None,
            implied_separate: false,
        }
    }

    pub fn partial(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: false,
            implied: None,
            implied_separate: false,
        }
    }

    /// 零声母简拼：敲的是 `typed`（`a`），代表的是 `syllable`（`an`）。
    ///
    /// 标成完整音节 —— 这个键就代表那个字，不是什么「打了一半的 an」。
    /// `separate` 为真时这一段是用户自己用 `'` 隔开的（`e'liao` 的 `e`），排序上不降权。
    pub fn implied(typed: &str, syllable: &str, separate: bool) -> Self {
        Self {
            text: typed.to_owned(),
            complete: true,
            implied: Some(syllable.to_owned()),
            implied_separate: separate,
        }
    }

    pub fn pattern(&self) -> SyllablePattern<'_> {
        // 有 `implied` 就按它查：`a` 这个键当 `an` 用
        match &self.implied {
            Some(syllable) => SyllablePattern {
                text: canonical_syllable(syllable),
                complete: true,
            },
            None => SyllablePattern {
                text: canonical_syllable(&self.text),
                complete: self.complete,
            },
        }
    }
}

/// 一种切分方式。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Segmentation {
    /// 音节，顺序与输入一致。
    pub syllables: Vec<Syllable>,
}

impl Segmentation {
    pub fn patterns(&self) -> Vec<SyllablePattern<'_>> {
        self.syllables.iter().map(Syllable::pattern).collect()
    }

    /// 覆盖的输入字母数（不含 `'`）。
    pub fn letters(&self) -> usize {
        self.syllables.iter().map(|s| s.text.len()).sum()
    }

    pub fn incomplete_count(&self) -> usize {
        self.syllables.iter().filter(|s| !s.complete).count()
    }

    pub fn last_is_partial(&self) -> bool {
        self.syllables.last().is_some_and(|s| !s.complete)
    }

    /// 拼音行显示用：各音节换成 [`umlaut_spelling`] 的写法后用 `'` 连接，`ce'lve` → `ce'lüe`。
    pub fn spelled(&self) -> String {
        self.syllables
            .iter()
            .map(|s| umlaut_spelling(&s.text))
            .collect::<Vec<_>>()
            .join("'")
    }

    /// 用分隔符连接各音节，给 marked text 用：`kai'fa`、`k'f`。
    pub fn joined(&self, separator: &str) -> String {
        self.syllables
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(separator)
    }
}

impl fmt::Display for Segmentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, syllable) in self.syllables.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            f.write_str(&syllable.text)?;
            if !syllable.complete {
                f.write_str("…")?;
            }
        }
        Ok(())
    }
}
