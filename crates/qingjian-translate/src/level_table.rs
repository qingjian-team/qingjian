use std::collections::HashMap;
use std::path::Path;

use crate::GlossaryError;

/// 词汇等级表：译词 → 等级（CEFR A1–C2 / JLPT N5–N1），给「统计」页按级数词汇用。
///
/// 文件是 TSV：`# levels\tA1\tA2…` 一行给出等级从易到难的顺序，之后每行 `词\t等级`；`#` 开头是注释。
/// 查询时英文按小写比，日文查不到再试去掉词尾的 する / な / だ（释义表里是 `開発する`，等级表里是 `開発`）。
#[derive(Debug, Default, Clone)]
pub struct LevelTable {
    /// 等级名，从易到难。
    levels: Vec<String>,

    /// 词 → 等级在 `levels` 里的下标。
    words: HashMap<String, usize>,

    /// 每级有多少词。
    sizes: Vec<u64>,
}

/// 日文查不到时试着去掉的词尾。
const JAPANESE_SUFFIXES: [&str; 3] = ["する", "な", "だ"];

impl LevelTable {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, GlossaryError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// 解析；没有 `# levels` 行时等级按首次出现的顺序排。格式不对的行报错（这是随包数据，不该坏）。
    pub fn parse(source: &str) -> Result<Self, GlossaryError> {
        let mut table = Self::default();
        for (index, line) in source.lines().enumerate() {
            if let Some(rest) = line.strip_prefix("# levels\t") {
                table.levels = rest.split('\t').map(str::to_owned).collect();
                table.sizes = vec![0; table.levels.len()];
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((word, level)) = line.split_once('\t') else {
                return Err(GlossaryError::Line {
                    line: index + 1,
                    reason: "expected `word\\tlevel`".to_owned(),
                });
            };
            let rank = match table.levels.iter().position(|l| l == level) {
                Some(rank) => rank,
                None => {
                    table.levels.push(level.to_owned());
                    table.sizes.push(0);
                    table.levels.len() - 1
                }
            };
            if table.words.insert(word.to_lowercase(), rank).is_none() {
                table.sizes[rank] += 1;
            }
        }
        Ok(table)
    }

    /// 等级名，从易到难。
    pub fn levels(&self) -> &[String] {
        &self.levels
    }

    /// 第 `rank` 级有多少词。
    pub fn size(&self, rank: usize) -> u64 {
        self.sizes.get(rank).copied().unwrap_or(0)
    }

    /// `word` 的等级下标（越小越易）；表里没有返回 `None`。
    pub fn rank(&self, word: &str) -> Option<usize> {
        let lower = word.to_lowercase();
        if let Some(rank) = self.words.get(&lower) {
            return Some(*rank);
        }
        JAPANESE_SUFFIXES
            .iter()
            .filter_map(|suffix| lower.strip_suffix(suffix))
            .find_map(|stem| self.words.get(stem).copied())
    }

    /// `word` 的等级名。
    pub fn level(&self, word: &str) -> Option<&str> {
        self.rank(word).map(|rank| self.levels[rank].as_str())
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_and_looks_up_with_normalization() {
        let table = LevelTable::parse(
            "# 注释\n# levels\tA1\tA2\tB1\nhello\tA1\nDevelop\tA2\ndevelopment\tB1\n開発\tB1\n",
        )
        .unwrap();
        assert_eq!(table.levels(), ["A1", "A2", "B1"]);
        assert_eq!(table.len(), 4);
        assert_eq!(table.level("Hello"), Some("A1"));
        assert_eq!(table.level("develop"), Some("A2"));
        assert_eq!(table.level("開発する"), Some("B1"));
        assert_eq!(table.level("nothing"), None);
        assert_eq!(table.size(2), 2);
        assert_eq!(table.size(9), 0);
    }

    #[test]
    fn levels_default_to_order_of_appearance_and_bad_lines_are_errors() {
        let table = LevelTable::parse("x\tN5\ny\tN4\n").unwrap();
        assert_eq!(table.levels(), ["N5", "N4"]);
        assert!(matches!(
            LevelTable::parse("no-tab\n"),
            Err(GlossaryError::Line { line: 1, .. })
        ));
    }
}
