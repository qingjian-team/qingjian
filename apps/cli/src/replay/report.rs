use std::fmt;

use qingjian_core::InputSource;

use super::tally::Tally;

/// 回放报告：按来源分组的计数，加上几条没命中的例子。
#[derive(Debug, Default)]
pub struct Report {
    /// 词库词。
    pub word: Tally,

    /// 本地整句。
    pub sentence: Tally,

    /// 英文候选。
    pub english: Tally,

    /// 快捷候选与 emoji。
    pub other: Tally,

    /// 不评的来源（云端词、云端整句、原样上屏、译词）各自的条数。
    pub skipped: Vec<(InputSource, usize)>,

    /// 撤销条数。
    pub retracts: usize,

    /// 解析不了的行数。
    pub unparsable: usize,

    /// 没命中首选的例子。
    pub misses: Vec<String>,
}

impl Report {
    /// 这个来源的计数板；不评的来源返回 `None`（另计到 `skipped`）。
    pub fn tally_for(&mut self, source: InputSource) -> Option<&mut Tally> {
        match source {
            InputSource::Word => Some(&mut self.word),
            InputSource::Sentence => Some(&mut self.sentence),
            InputSource::English => Some(&mut self.english),
            InputSource::Shortcut | InputSource::Emoji => Some(&mut self.other),
            InputSource::Cloud
            | InputSource::CloudSentence
            | InputSource::Raw
            | InputSource::Translation => None,
        }
    }

    pub fn skip(&mut self, source: InputSource) {
        match self.skipped.iter_mut().find(|(s, _)| *s == source) {
            Some((_, count)) => *count += 1,
            None => self.skipped.push((source, 1)),
        }
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

fn write_tally(f: &mut fmt::Formatter<'_>, name: &str, tally: &Tally) -> fmt::Result {
    if tally.total == 0 {
        return Ok(());
    }
    let evaluated = tally.evaluated();
    writeln!(
        f,
        "{name:<6} {:>5} 条  首选 {:>6}  前五 {:>6}  更靠后 {:>4}  不在候选 {:>4}  平均名次 {}",
        tally.total,
        percent(tally.top1, evaluated),
        percent(tally.top1 + tally.top5, evaluated),
        tally.found_later,
        tally.missing,
        tally
            .mean_rank()
            .map_or("-".to_owned(), |rank| format!("{rank:.2}")),
    )?;
    if tally.unparsable > 0 {
        writeln!(
            f,
            "       其中 {} 条现在切不动（方案或模式键变了）",
            tally.unparsable
        )?;
    }
    if tally.corrected_then > 0 {
        writeln!(
            f,
            "       当时纠错生效 {} 条，现在仍纠 {} 条",
            tally.corrected_then, tally.corrected_now
        )?;
    }
    Ok(())
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "回放评测（内存学习，不写文件）")?;
        write_tally(f, "词", &self.word)?;
        write_tally(f, "整句", &self.sentence)?;
        write_tally(f, "英文", &self.english)?;
        write_tally(f, "其他", &self.other)?;
        if !self.skipped.is_empty() {
            let parts: Vec<String> = self
                .skipped
                .iter()
                .map(|(source, count)| format!("{source:?} {count}"))
                .collect();
            writeln!(f, "不评的来源：{}", parts.join("，"))?;
        }
        if self.retracts > 0 {
            writeln!(f, "退格撤销 {} 次", self.retracts)?;
        }
        if self.unparsable > 0 {
            writeln!(f, "解析不了 {} 行", self.unparsable)?;
        }
        if !self.misses.is_empty() {
            writeln!(f, "\n没命中首选的例子：")?;
            for miss in &self.misses {
                writeln!(f, "  {miss}")?;
            }
        }
        Ok(())
    }
}
