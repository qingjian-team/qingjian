use std::fmt;
use std::time::Duration;

/// 整句评测报告。
#[derive(Debug, Default)]
pub struct Report {
    /// 抽出来的句子数（去重后）。
    pub extracted: usize,

    /// 有字查不到读音、没法转拼音的句子数。
    pub untranscribable: usize,

    /// 评了的句子数。
    pub total: usize,

    /// 拼音现在切不动的句子数。
    pub unparsable: usize,

    /// 首选就是原句。
    pub top1: usize,

    /// 整句候选（第一个盖住全部拼音的候选）就是原句。
    pub sentence_hit: usize,

    /// 整句候选与原句逐字比对：对上的字数 / 总字数。
    pub chars_correct: usize,
    pub chars_total: usize,

    /// 查询耗时之和与最大值。
    pub query_time: Duration,
    pub slowest_query: Duration,

    /// 没命中首选的例子。
    pub misses: Vec<String>,
}

impl Report {
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "-".to_owned()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "整句评测（冷启动，不学习，不写文件）")?;
        let evaluated = self.evaluated();
        writeln!(
            f,
            "句子 {:>5} 条  首选 {:>6}  整句候选 {:>6}  字准确率 {:>6}",
            self.total,
            percent(self.top1, evaluated),
            percent(self.sentence_hit, evaluated),
            percent(self.chars_correct, self.chars_total),
        )?;
        if evaluated > 0 {
            writeln!(
                f,
                "查询平均 {:.1} ms，最慢 {:.1} ms",
                self.query_time.as_secs_f64() * 1000.0 / evaluated as f64,
                self.slowest_query.as_secs_f64() * 1000.0,
            )?;
        }
        if self.unparsable > 0 {
            writeln!(f, "其中 {} 条拼音切不动", self.unparsable)?;
        }
        if self.untranscribable > 0 {
            writeln!(
                f,
                "抽出 {} 条，{} 条有字查不到读音，跳过",
                self.extracted, self.untranscribable
            )?;
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
