//! 英文词表：每行 `词\t编码[\t…]`（表头行、YAML 头这类没有制表符或编码不是字母的行自然跳过），`#` 为注释。只收纯字母的词。
//!
//! 多个输入文件时：同一文件内同一编码**优先全小写**（[`prefer_lowercase`]：`or` 不出 `OR`、`may` 不出 `May`），
//! 后面的文件可以**覆盖**前面已经收过的编码——展示写法补充表（`07_display_forms.tsv`：`Windows` / `GitHub` / `VSCode`）
//! 就是靠这条覆盖全小写的普通词形，而不动别的词的顺序。

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::ConvertError;

/// `frequency` 是可选的 `编码\t词频` 表，有就写进第三列。
pub fn convert(
    inputs: &[PathBuf],
    frequency: Option<&Path>,
    output: &Path,
) -> Result<(), ConvertError> {
    let frequencies: HashMap<String, u32> = match frequency {
        Some(path) => std::fs::read_to_string(path)?
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let (code, count) = l.split_once('\t')?;
                Some((code.to_owned(), count.trim().parse().ok()?))
            })
            .collect(),
        None => HashMap::new(),
    };
    // 编码 → 在 `rows` 里的下标；`rows` 按首次出现的顺序，输出顺序与输入一致
    let mut index_by_code: HashMap<String, usize> = HashMap::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for path in inputs {
        let before = rows.len();
        // 编码 → 这个文件里留下的写法与它的常见度级别
        let mut seen_in_file: HashMap<String, (String, Option<u32>)> = HashMap::new();
        for raw in std::fs::read_to_string(path)?.lines() {
            let line = raw.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let word = fields.next().unwrap_or_default().trim();
            let Some(code) = fields.next().map(str::trim) else {
                continue;
            };
            // 数据包的表头：`word\tkey\tfrequency_level\t…`
            if word == "word" && code == "key" {
                continue;
            }
            let code = code.to_ascii_lowercase();
            if word.is_empty() || !word.bytes().all(|b| b.is_ascii_alphabetic()) {
                continue;
            }
            let level = fields.next().and_then(|l| l.trim().parse::<u32>().ok());
            // 同一文件内同一编码只留一种写法，见 `prefer_lowercase`
            if let Some(kept) = seen_in_file.get_mut(&code) {
                if prefer_lowercase(word, level, &kept.0, kept.1) {
                    *kept = (word.to_owned(), level);
                    rows[index_by_code[&code]].0 = word.to_owned();
                }
                continue;
            }
            seen_in_file.insert(code.clone(), (word.to_owned(), level));
            match index_by_code.get(&code) {
                // 后面的文件覆盖前面的：展示写法补充表用这条把 `windows` 顶成 `Windows`
                Some(&index) => rows[index].0 = word.to_owned(),
                None => {
                    index_by_code.insert(code.clone(), rows.len());
                    rows.push((word.to_owned(), code));
                }
            }
        }
        tracing::info!(path = %path.display(), entries = rows.len() - before, "已读取");
    }
    let mut file = BufWriter::new(std::fs::File::create(output)?);
    writeln!(
        file,
        "# 由 qingjian-dict-convert english 生成。词\\t编码\\t词频（wordfreq 的 Zipf 频率 ×1000）"
    )?;
    for (word, code) in &rows {
        let count = frequencies.get(code).copied().unwrap_or(0);
        writeln!(file, "{word}\t{code}\t{count}")?;
    }
    file.flush()?;
    tracing::info!(path = %output.display(), entries = rows.len(), "写出完成");
    Ok(())
}

/// 同一文件里同一编码又来一种写法 `word`（常见度级别 `level`，越小越常见，没有算最不常见）时，要不要换掉留着的 `kept`。
///
/// 数据包把大写写法排在小写前面（`OR` 在 `or` 前、`May` 在 `may` 前），先到先得会让普通词出成缩写或专名。
/// 所以全小写的写法优先；只有多于一个字母、且级别**严格**更常见的大写写法留下（`Java` 50 对 `java` 60、`HTML` 50 对没有级别的 `html`）。
/// 单个字母一律小写，`I` 靠展示写法补充表。
fn prefer_lowercase(word: &str, level: Option<u32>, kept: &str, kept_level: Option<u32>) -> bool {
    let lower = |w: &str| !w.bytes().any(|b| b.is_ascii_uppercase());
    if !lower(word) || lower(kept) {
        return false;
    }
    kept.len() == 1 || kept_level.unwrap_or(u32::MAX) >= level.unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(name: &str, body: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("qingjian-english-{}-{name}", std::process::id()));
        std::fs::write(&path, body).unwrap();
        path
    }

    fn converted(tag: &str, inputs: &[PathBuf]) -> Vec<(String, String)> {
        let out = std::env::temp_dir().join(format!(
            "qingjian-english-{}-{tag}-out.tsv",
            std::process::id()
        ));
        convert(inputs, None, &out).unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        let rows: Vec<(String, String)> = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let mut fields = l.split('\t');
                Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
            })
            .collect();
        std::fs::remove_file(out).unwrap();
        rows
    }

    #[test]
    fn skips_the_header_and_keeps_first_seen_casing_within_a_file() {
        let a = write(
            "a.tsv",
            "word\tkey\tfrequency_level\nwindows\twindows\t35\nWindows\twindows\t50\nact\tact\t5300\n",
        );
        let rows = converted("header", std::slice::from_ref(&a));
        std::fs::remove_file(a).unwrap();
        // 表头不进来；同一文件内先到先得，`windows` 保持小写、`act` 的小写写法也没被 `ACT` 顶掉
        assert_eq!(
            rows,
            [
                ("windows".to_owned(), "windows".to_owned()),
                ("act".to_owned(), "act".to_owned())
            ]
        );
    }

    #[test]
    fn lowercase_wins_unless_the_capitalized_form_is_more_common() {
        let a = write(
            "a3.tsv",
            "OR	or	35
or	or	35
B	b	35
b	b	40
Java	java	50
java	java	60
             HTML	html	50
html	html
John	john	35
john	john	40
Google	google	50
google	google	50
",
        );
        let rows = converted("lowercase", std::slice::from_ref(&a));
        std::fs::remove_file(a).unwrap();
        let words: Vec<&str> = rows.iter().map(|(word, _)| word.as_str()).collect();
        // 位置仍是编码第一次出现的位置
        assert_eq!(words, ["or", "b", "Java", "HTML", "John", "google"]);
    }

    #[test]
    fn later_files_override_and_reuse_the_existing_position() {
        let a = write("a2.tsv", "windows\twindows\nact\tact\n");
        let b = write("b2.tsv", "word\tkey\nWindows\twindows\nVSCode\tvscode\n");
        let rows = converted("override", &[a.clone(), b.clone()]);
        std::fs::remove_file(a).unwrap();
        std::fs::remove_file(b).unwrap();
        // 覆盖保持原来的位置与顺序，新编码追加在后
        assert_eq!(
            rows,
            [
                ("Windows".to_owned(), "windows".to_owned()),
                ("act".to_owned(), "act".to_owned()),
                ("VSCode".to_owned(), "vscode".to_owned()),
            ]
        );
    }
}
