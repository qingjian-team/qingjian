//! 生词本的时间线：`vocab-log.tsv`，一次相遇一行 `时间\t语言\t中文\t说法\t读音`，按时间追加。
//! 输入法只往末尾追加（落盘时一次写一批），设置程序整读、清空某门语言时整个重写。

mod logged_encounter;

pub use logged_encounter::LoggedEncounter;

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use qingjian_core::Language;
use qingjian_core::storage::{read_text_lossy, write_atomic_str};

use crate::error::LearningError;

pub const ENCOUNTER_LOG_FILE: &str = "vocab-log.tsv";

/// 追加一批；文件不存在就建。
pub fn append(path: &Path, encounters: &[LoggedEncounter]) -> Result<(), LearningError> {
    if encounters.is_empty() {
        return Ok(());
    }
    let mut text = String::new();
    for encounter in encounters {
        line(&mut text, encounter);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

/// 整读，按文件里的顺序（时间先后）；坏行跳过。读不了返回空。
pub fn read(path: &Path) -> Vec<LoggedEncounter> {
    let text = match read_text_lossy(path) {
        Ok(Some(text)) => text,
        Ok(None) => return Vec::new(),
        Err(error) => {
            tracing::warn!(%error, "生词本时间线读不了");
            return Vec::new();
        }
    };
    text.lines().filter_map(parse_line).collect()
}

/// 删掉某门语言的全部记录（整个重写）。
pub fn remove_language(path: &Path, language: Language) -> Result<(), LearningError> {
    if !path.exists() {
        return Ok(());
    }
    let mut text = String::new();
    for encounter in read(path).iter().filter(|e| e.language != language) {
        line(&mut text, encounter);
    }
    write_atomic_str(path, &text)?;
    Ok(())
}

fn line(out: &mut String, encounter: &LoggedEncounter) {
    let clean = |text: &str| text.replace(|c: char| c.is_control(), " ");
    let _ = writeln!(
        out,
        "{}\t{}\t{}\t{}\t{}",
        encounter.time,
        encounter.language.code(),
        clean(&encounter.text),
        clean(&encounter.translation),
        clean(&encounter.reading),
    );
}

fn parse_line(line: &str) -> Option<LoggedEncounter> {
    let mut fields = line.split('\t');
    let time = fields.next()?.parse().ok()?;
    let language = fields.next()?.parse().ok()?;
    let text = fields.next()?.to_owned();
    let translation = fields.next()?.to_owned();
    let reading = fields.next().unwrap_or_default().to_owned();
    (!text.is_empty() && !translation.is_empty()).then_some(LoggedEncounter {
        time,
        language,
        text,
        translation,
        reading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_reads_and_removes_a_language() {
        let dir = std::env::temp_dir().join(format!("qingjian-encounters-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(ENCOUNTER_LOG_FILE);
        let _ = std::fs::remove_file(&path);
        let at = |time: &str, language, text: &str, translation: &str| LoggedEncounter {
            time: time.parse().unwrap(),
            language,
            text: text.to_owned(),
            translation: translation.to_owned(),
            reading: String::new(),
        };
        append(
            &path,
            &[at(
                "2026-10-16T16:52:00",
                Language::English,
                "生词本",
                "new word\tbook",
            )],
        )
        .unwrap();
        append(
            &path,
            &[at(
                "2026-10-16T16:53:05",
                Language::Japanese,
                "生词本",
                "単語帳",
            )],
        )
        .unwrap();
        std::fs::write(&path, std::fs::read_to_string(&path).unwrap() + "坏行\n").unwrap();
        let all = read(&path);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].translation, "new word book");
        assert_eq!(all[1].time.to_string(), "2026-10-16T16:53:05");
        remove_language(&path, Language::English).unwrap();
        let left = read(&path);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].language, Language::Japanese);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
