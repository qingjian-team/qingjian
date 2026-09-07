use std::fmt;

use qingjian_dictionary::SyllablePattern;

/// 切分出的一个音节。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Syllable {
    /// 用户敲的字母。
    pub text: String,

    /// 是完整音节；否则是声母（简拼 `k`）或未打完的前缀（`zho`）。
    pub complete: bool,
}

impl Syllable {
    pub fn complete(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: true,
        }
    }

    pub fn partial(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            complete: false,
        }
    }

    pub fn pattern(&self) -> SyllablePattern<'_> {
        SyllablePattern {
            text: &self.text,
            complete: self.complete,
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
