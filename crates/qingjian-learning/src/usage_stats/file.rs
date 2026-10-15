//! 统计文件的读写：一行一个时段，同一时段出现多次就加起来；格式不对的行跳过。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::Path;

use jiff::civil::Date;
use qingjian_core::Usage;

/// 按小时的文件开头那行。
pub const HEADER: &str =
    "# 日期\t时\t汉字\t中文词\t英文词\t上屏\t选词\t首选\t删过\t打出汉字\t键数\t打字毫秒\n";

/// 一个时段一行：`日期\t时\t汉字\t中文词\t英文词\t上屏\t选词\t首选\t删过\t打出汉字\t键数\t打字毫秒`。
pub fn hour_line(date: Date, hour: u8, usage: &Usage) -> String {
    let mut line = String::new();
    let _ = writeln!(
        line,
        "{date}\t{hour}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        usage.hanzi,
        usage.words,
        usage.english_words,
        usage.commits,
        usage.choices,
        usage.first_choices,
        usage.edited,
        usage.typed_hanzi,
        usage.keys,
        usage.typing_ms
    );
    line
}

/// 读按小时的文件；也返回读到几行有效记录，比时段数多出很多就该整理了。
pub fn parse_hours(text: &str) -> (BTreeMap<(Date, u8), Usage>, usize) {
    let mut hours = BTreeMap::new();
    let mut lines = 0;
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_hour_line(line) {
            Some((key, usage)) => {
                *hours.entry(key).or_default() += usage;
                lines += 1;
            }
            None => tracing::warn!(line = number + 1, "输入统计有坏行，跳过"),
        }
    }
    (hours, lines)
}

fn parse_hour_line(line: &str) -> Option<((Date, u8), Usage)> {
    let mut fields = line.split('\t');
    let date: Date = fields.next()?.parse().ok()?;
    let hour: u8 = fields.next()?.parse().ok().filter(|hour| *hour < 24)?;
    let mut number = || fields.next()?.parse::<u64>().ok();
    let usage = Usage {
        hanzi: number()?,
        words: number()?,
        english_words: number()?,
        commits: number()?,
        choices: number()?,
        first_choices: number()?,
        edited: number()?,
        typed_hanzi: number()?,
        keys: number()?,
        typing_ms: number()?,
    };
    Some(((date, hour), usage))
}

/// 读 0.2 以前按天的 `usage.tsv`：`日期\t汉字\t中文词\t英文词\t上屏次数`，只读不写。
pub fn parse_days(text: &str) -> BTreeMap<Date, Usage> {
    let mut days = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((date, usage)) = parse_day_line(line) {
            *days.entry(date).or_default() += usage;
        }
    }
    days
}

fn parse_day_line(line: &str) -> Option<(Date, Usage)> {
    let mut fields = line.split('\t');
    let date: Date = fields.next()?.parse().ok()?;
    let mut number = || fields.next()?.parse::<u64>().ok();
    let usage = Usage {
        hanzi: number()?,
        words: number()?,
        english_words: number()?,
        commits: number()?,
        ..Usage::default()
    };
    Some((date, usage))
}

/// 追加一行；文件是新建的先写表头。一行一次写完，进程被杀最多留半行，读的时候跳过；
/// 末尾是这样的半行时先换行，免得新的一行接在它后面一起作废。
pub fn append(path: &Path, line: &str) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .create(true)
        .append(true)
        .open(path)?;
    let mut text = String::new();
    if file.metadata()?.len() == 0 {
        text.push_str(HEADER);
    } else {
        let mut last = [0u8];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut last)?;
        if last[0] != b'\n' {
            text.push('\n');
        }
    }
    text.push_str(line);
    file.write_all(text.as_bytes())
}
