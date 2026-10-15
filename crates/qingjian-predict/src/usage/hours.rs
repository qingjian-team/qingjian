//! 按小时的云服务用量（`api-usage-hourly.tsv`）：每个请求有结果时追加一行，文件大了整理成一个时段一行。
//! 累计数仍以 `api-usage.json` 为准，这里给统计页画每天、每小时的图。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use jiff::civil::Date;
use qingjian_core::storage::{read_text_lossy, write_atomic_str};

use super::ApiUsageHour;

/// 累计文件同目录下的按小时文件。
pub const API_USAGE_HOURLY_FILE: &str = "api-usage-hourly.tsv";

const HEADER: &str = "# 日期\t时\t请求\t失败\t输入\t输出\t合计\n";

/// 文件超过这么大就整理一次。
const COMPACT_BYTES: u64 = 256 * 1024;

/// 按日期与时（0–23）排好的用量。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiUsageHours {
    pub hours: BTreeMap<(Date, u8), ApiUsageHour>,
}

impl ApiUsageHours {
    /// 读数据目录里的按小时用量；文件不存在为空，坏行跳过。
    pub fn load(dir: &Path) -> io::Result<Self> {
        let text = read_text_lossy(&dir.join(API_USAGE_HOURLY_FILE))?;
        Ok(Self {
            hours: text.as_deref().map(parse).unwrap_or_default(),
        })
    }

    /// 每天的合计。
    pub fn days(&self) -> BTreeMap<Date, ApiUsageHour> {
        let mut days = BTreeMap::new();
        for ((date, _), hour) in &self.hours {
            *days.entry(*date).or_insert_with(ApiUsageHour::default) += *hour;
        }
        days
    }

    /// 某天 24 个小时各自的用量。
    pub fn hours_on(&self, date: Date) -> [ApiUsageHour; 24] {
        let mut out = [ApiUsageHour::default(); 24];
        for ((_, hour), usage) in self.hours.range((date, 0)..=(date, 23)) {
            out[*hour as usize] = *usage;
        }
        out
    }
}

/// 累计文件旁边的按小时文件。
pub(super) fn path_beside(json: &Path) -> PathBuf {
    json.with_file_name(API_USAGE_HOURLY_FILE)
}

/// 记一个有结果的请求到现在这个小时；调用方持着用量文件锁。
pub(super) fn append_now(path: &Path, usage: ApiUsageHour) -> io::Result<()> {
    let now = jiff::Zoned::now().datetime();
    append(path, &line(now.date(), now.hour() as u8, &usage))?;
    if fs::metadata(path)?.len() > COMPACT_BYTES {
        compact(path)?;
    }
    Ok(())
}

fn line(date: Date, hour: u8, usage: &ApiUsageHour) -> String {
    let mut line = String::new();
    let _ = writeln!(
        line,
        "{date}\t{hour}\t{}\t{}\t{}\t{}\t{}",
        usage.requests, usage.failures, usage.input_tokens, usage.output_tokens, usage.total_tokens
    );
    line
}

fn parse(text: &str) -> BTreeMap<(Date, u8), ApiUsageHour> {
    let mut hours = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, usage)) = parse_line(line) {
            *hours.entry(key).or_insert_with(ApiUsageHour::default) += usage;
        }
    }
    hours
}

fn parse_line(line: &str) -> Option<((Date, u8), ApiUsageHour)> {
    let mut fields = line.split('\t');
    let date: Date = fields.next()?.parse().ok()?;
    let hour: u8 = fields.next()?.parse().ok().filter(|hour| *hour < 24)?;
    let mut number = || fields.next()?.parse::<u64>().ok();
    let usage = ApiUsageHour {
        requests: number()?,
        failures: number()?,
        input_tokens: number()?,
        output_tokens: number()?,
        total_tokens: number()?,
    };
    Some(((date, hour), usage))
}

/// 追加一行，文件新建时先写表头；末尾是被打断的半行时先换行。
fn append(path: &Path, line: &str) -> io::Result<()> {
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

fn compact(path: &Path) -> io::Result<()> {
    let hours = read_text_lossy(path)?
        .as_deref()
        .map(parse)
        .unwrap_or_default();
    let mut text = String::from(HEADER);
    for ((date, hour), usage) in &hours {
        text.push_str(&line(*date, *hour, usage));
    }
    write_atomic_str(path, &text)
}
