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

#[cfg(test)]
mod tests {
    use super::{HEADER, append, hour_line, parse_days, parse_hours};
    use jiff::civil::Date;
    use qingjian_core::Usage;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn sample() -> Usage {
        Usage {
            hanzi: 12,
            words: 3,
            english_words: 1,
            commits: 5,
            choices: 2,
            first_choices: 4,
            edited: 1,
            typed_hanzi: 9,
            keys: 30,
            typing_ms: 1200,
        }
    }

    fn numbers(usage: &Usage) -> (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (
            usage.hanzi,
            usage.words,
            usage.english_words,
            usage.commits,
            usage.choices,
            usage.first_choices,
            usage.edited,
            usage.typed_hanzi,
            usage.keys,
            usage.typing_ms,
        )
    }

    #[test]
    fn a_hour_line_round_trips() {
        let text = hour_line(date("2026-11-20"), 9, &sample());
        let (hours, lines) = parse_hours(&text);
        assert_eq!(lines, 1);
        assert_eq!(
            numbers(&hours[&(date("2026-11-20"), 9)]),
            numbers(&sample())
        );
        assert!(text.starts_with("2026-11-20\t9\t"));
    }

    #[test]
    fn the_header_has_as_many_columns_as_a_line() {
        let columns = |text: &str| text.trim_end().split('\t').count();
        assert_eq!(
            columns(HEADER),
            columns(&hour_line(date("2026-11-20"), 0, &Usage::default()))
        );
    }

    #[test]
    fn the_same_hour_adds_up_and_bad_lines_are_skipped() {
        let mut text = hour_line(date("2026-11-20"), 9, &sample());
        text.push_str(&hour_line(date("2026-11-20"), 9, &sample()));
        text.push('\n');
        text.push_str("2026-11-20\t24\t1\t1\n"); // 时越界
        text.push_str("2026-11-20\t9\t\n"); // 列不够
        text.push_str("不是日期\t9\t1\n"); // 日期坏
        let (hours, lines) = parse_hours(&text);
        assert_eq!(lines, 2);
        let total = &hours[&(date("2026-11-20"), 9)];
        assert_eq!((total.hanzi, total.keys), (24, 60));
    }

    #[test]
    fn the_old_daily_format_reads_the_first_columns() {
        let text = "# 日期\t汉字\t中文词\t英文词\t上屏次数\n\
                    2026-11-19\t3\t1\t0\t2\n\
                    2026-11-19\t1\t0\t0\t1\n";
        let days = parse_days(text);
        let day = &days[&date("2026-11-19")];
        assert_eq!((day.hanzi, day.words, day.commits), (4, 1, 3));
    }

    #[test]
    fn appending_writes_a_header_only_to_an_empty_file() {
        let dir = std::env::temp_dir().join(format!("qj-usage-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("usage-hours.tsv");
        append(&path, &hour_line(date("2026-11-20"), 9, &sample())).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(HEADER));
        assert_eq!(parse_hours(&text).1, 1);
        append(&path, &hour_line(date("2026-11-20"), 10, &sample())).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches('#').count(), 1);
        assert_eq!(parse_hours(&text).1, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_half_written_tail_gets_a_break_before_the_next_line() {
        let dir = std::env::temp_dir().join(format!("qj-usage-half-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("usage-hours.tsv");
        std::fs::write(&path, format!("{HEADER}2026-11-20\t9\t1")).unwrap();
        append(&path, &hour_line(date("2026-11-20"), 10, &sample())).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("2026-11-20\t9\t1\n2026-11-20\t10\t"));
        assert_eq!(parse_hours(&text).1, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
