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

    /// 重打条数（组句内退格重打 + 上屏后删掉重打）。
    pub retypes: usize,

    /// 会话行数、上文断开数、直通字符数。
    pub sessions: usize,
    pub breaks: usize,
    pub passthrough_chars: usize,

    /// 云端联想展示了几次、其中紧接着被接受（上屏来源是云端词 / 云端整句）几次。
    pub predictions: usize,
    pub predictions_accepted: usize,

    /// 上一条是联想、还没等到接下来的上屏。
    pub prediction_pending: bool,

    /// 旧格式的空行（键与文本都空的原样上屏）。
    pub empty: usize,

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
    if let Some(((hit_pages, hit_ms), (miss_pages, miss_ms))) = tally.implicit_signal() {
        writeln!(
            f,
            "       当时首选命中 {}：平均翻页 {hit_pages:.2} / 耗时 {hit_ms:.0} ms；没命中 {}：平均翻页 {miss_pages:.2} / 耗时 {miss_ms:.0} ms",
            percent(tally.then_top1, tally.total),
            tally.total - tally.then_top1,
        )?;
    }
    if tally.rescored_then > 0 {
        let plain = tally.total - tally.rescored_then;
        let plain_top1 = tally.then_top1 - tally.rescored_then_top1;
        writeln!(
            f,
            "       当时经过神经重排 {} 条首选命中 {}，未重排 {} 条首选命中 {}",
            tally.rescored_then,
            percent(tally.rescored_then_top1, tally.rescored_then),
            plain,
            percent(plain_top1, plain),
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
        if self.retypes > 0 {
            writeln!(f, "退格重打 {} 次", self.retypes)?;
        }
        if self.predictions > 0 {
            writeln!(
                f,
                "云端联想展示 {} 次，紧接着被接受 {} 次（{}）",
                self.predictions,
                self.predictions_accepted,
                percent(self.predictions_accepted, self.predictions)
            )?;
        }
        if self.sessions + self.breaks + self.passthrough_chars > 0 {
            writeln!(
                f,
                "会话 {} 次，上文断开 {} 次，直通字符 {} 个",
                self.sessions, self.breaks, self.passthrough_chars
            )?;
        }
        if self.empty > 0 {
            writeln!(f, "旧格式的空行 {} 条（不算数）", self.empty)?;
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
