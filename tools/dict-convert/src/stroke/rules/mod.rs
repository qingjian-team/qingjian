//! 大陆序覆盖表：CNS（台湾标准）笔顺按部件重写成大陆规范。
//!
//! 表在 `assets/stroke/prc-rules.tsv`，三类行（`#` 开头是注释，TAB 分隔）：
//!
//! - `rule\t部件\tCNS 模式\t大陆模式\t匹配位置`：部件重写，按行序逐字应用；
//! - `skip\t部件\t字…`：例外字，这些字里命中的模式不是那个部件，规则不适用；
//! - `char\t字\t序列`：整字覆盖（CNS 没有这个字时人工补录），优先于所有规则。
//!
//! 规则只做模式替换、不猜字形：改不动的整字形差异（台湾标准与大陆规范笔画数就不同的字）留给抽样对照的白名单，
//! 见 `assets/stroke/README.md`。

mod anchor;
mod rule;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;
use crate::stroke::rules::anchor::Anchor;
use crate::stroke::rules::rule::Rule;

/// CNS 序列里出现的笔画码：1 横 / 2 竖 / 3 撇 / 4 点与捺 / 5 折。
const STROKE_DIGITS: &str = "12345";

/// 大陆序覆盖表。
pub struct PrcRules {
    /// 部件重写规则，按覆盖表里的顺序。
    rules: Vec<Rule>,

    /// 整字覆盖：字 → 序列。
    overrides: HashMap<char, String>,
}

impl PrcRules {
    /// 读覆盖表。
    pub fn from_path(path: &Path) -> Result<Self, ConvertError> {
        Self::parse(path, BufReader::new(File::open(path)?))
    }

    /// 从任意读入流解析覆盖表（测试用）。
    pub(super) fn parse(path: &Path, reader: impl BufRead) -> Result<Self, ConvertError> {
        let mut rules: Vec<Rule> = Vec::new();
        let mut overrides: HashMap<char, String> = HashMap::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let number = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["rule", name, from, to, anchor] => {
                    rules.push(Rule {
                        name: (*name).to_owned(),
                        from: (*from).to_owned(),
                        to: (*to).to_owned(),
                        anchor: Anchor::parse(anchor)
                            .ok_or_else(|| malformed(path, number, "unknown anchor"))?,
                        skips: HashSet::new(),
                    });
                }
                ["skip", name, chars] => {
                    let rule = rules
                        .iter_mut()
                        .find(|rule| rule.name == *name)
                        .ok_or_else(|| malformed(path, number, "skip refers to an unknown rule"))?;
                    rule.skips
                        .extend(chars.chars().filter(|c| !c.is_whitespace()));
                }
                ["char", ch, seq] => {
                    let Some(ch) = single_char(ch) else {
                        return Err(malformed(path, number, "expected one character"));
                    };
                    if seq.is_empty() || !seq.chars().all(|c| STROKE_DIGITS.contains(c)) {
                        return Err(malformed(path, number, "expected a stroke sequence of 1-5"));
                    }
                    overrides.insert(ch, (*seq).to_owned());
                }
                _ => return Err(malformed(path, number, "expected rule / skip / char")),
            }
        }
        Ok(Self { rules, overrides })
    }

    /// 整字覆盖的序列（CNS 没有这个字时人工补录的那条）。
    pub fn override_sequence(&self, ch: char) -> Option<&str> {
        self.overrides.get(&ch).map(String::as_str)
    }

    /// 把一个字的 CNS 序列改成大陆序。
    pub fn apply(&self, ch: char, seq: &str) -> String {
        if let Some(overridden) = self.overrides.get(&ch) {
            return overridden.clone();
        }
        let mut out = seq.to_owned();
        for rule in &self.rules {
            if rule.skips.contains(&ch) {
                continue;
            }
            if let Some(rewritten) = rule.anchor.rewrite(&out, &rule.from, &rule.to) {
                out = rewritten;
            }
        }
        out
    }

    /// 规则条数，日志用。
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 整字覆盖条数，日志用。
    pub fn override_count(&self) -> usize {
        self.overrides.len()
    }
}

/// 第一个字段就是一个字（覆盖表、字表白名单与笔画表都按这个口径认行）。
pub(crate) fn single_char(field: &str) -> Option<char> {
    let mut chars = field.chars();
    let ch = chars.next()?;
    if chars.next().is_none() {
        Some(ch)
    } else {
        None
    }
}

/// 这类表里的坏行（覆盖表、笔画表共用同一套报错）。
pub(crate) fn malformed(path: &Path, line: usize, reason: &str) -> ConvertError {
    ConvertError::Format {
        path: PathBuf::from(path),
        line,
        reason: reason.to_owned(),
    }
}
