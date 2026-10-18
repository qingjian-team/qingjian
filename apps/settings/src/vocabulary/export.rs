//! 把生词本当前筛出来的词写成 CSV、Anki 导入文件或离线网页。

use std::fmt::Write as _;

use qingjian_learning::VocabularyEntry;

use super::Format;

pub fn render(format: Format, language: &str, words: &[(&str, &VocabularyEntry)]) -> String {
    match format {
        Format::Csv => csv(words),
        Format::Anki => anki(words),
        Format::Html => html(language, words),
    }
}

/// 带 BOM：Excel 不带 BOM 时按本地编码读，中文成乱码。
fn csv(words: &[(&str, &VocabularyEntry)]) -> String {
    let mut out = String::from("\u{feff}译词,中文,例句,看到,上屏,直接打出,首见,最近,认识\r\n");
    for (word, entry) in words {
        let fields = [
            (*word).to_owned(),
            entry.source.clone(),
            entry.example.clone(),
            entry.seen.to_string(),
            entry.committed.to_string(),
            entry.used.to_string(),
            entry.first.to_string(),
            entry.last.to_string(),
            if entry.known { "是" } else { "" }.to_owned(),
        ];
        let line: Vec<String> = fields.iter().map(|field| csv_field(field)).collect();
        out += &line.join(",");
        out += "\r\n";
    }
    out
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// Anki 2.1.55 起认文件头：制表符分隔、字段是 HTML，正面译词，背面中文与例句。
fn anki(words: &[(&str, &VocabularyEntry)]) -> String {
    let mut out = String::from("#separator:tab\n#html:true\n#columns:正面\t背面\n");
    for (word, entry) in words {
        let mut back = escape(&entry.source);
        if !entry.example.is_empty() {
            let _ = write!(back, "<br><br><i>{}</i>", escape(&entry.example));
        }
        let _ = writeln!(out, "{}\t{back}", escape(word));
    }
    out
}

fn html(language: &str, words: &[(&str, &VocabularyEntry)]) -> String {
    let today = jiff::Zoned::now().date();
    let mut rows = String::new();
    for (word, entry) in words {
        let known = if entry.known { " class=\"known\"" } else { "" };
        let _ = writeln!(
            rows,
            "<tr{known}><td class=\"word\">{}</td><td>{}</td><td class=\"example\">{}</td>\
             <td class=\"count\">{}</td><td class=\"date\">{}</td></tr>",
            escape(word),
            escape(&entry.source),
            escape(&entry.example),
            entry.seen,
            entry.last,
        );
    }
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>青简生词本</title>
<style>
:root {{ --text: #1f2a24; --muted: #6b7a72; --line: #dfe5e1; --page: #fbfcfb; --known: #9aa7a0; }}
@media (prefers-color-scheme: dark) {{
  :root {{ --text: #e3e9e5; --muted: #95a39b; --line: #2f3833; --page: #161a18; --known: #66726b; }}
}}
body {{ margin: 0; background: var(--page); color: var(--text);
  font: 15px/1.6 -apple-system, "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif; }}
main {{ max-width: 960px; margin: 0 auto; padding: 32px 16px; }}
h1 {{ font-size: 22px; margin: 0 0 4px; }}
p {{ color: var(--muted); margin: 0 0 24px; }}
table {{ width: 100%; border-collapse: collapse; }}
th, td {{ text-align: left; padding: 8px 10px; border-bottom: 1px solid var(--line); vertical-align: top; }}
th {{ color: var(--muted); font-weight: 600; font-size: 13px; }}
.word {{ font-weight: 600; white-space: nowrap; }}
.example {{ color: var(--muted); }}
.count, .date {{ white-space: nowrap; font-variant-numeric: tabular-nums; }}
tr.known td {{ color: var(--known); }}
@media (max-width: 640px) {{ .date, th:last-child {{ display: none; }} }}
</style>
</head>
<body>
<main>
<h1>青简生词本</h1>
<p>{language} · {count} 个词 · 导出于 {today}</p>
<table>
<thead><tr><th>译词</th><th>中文</th><th>例句</th><th>看到</th><th>最近</th></tr></thead>
<tbody>
{rows}</tbody>
</table>
</main>
</body>
</html>
"#,
        count = words.len(),
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, example: &str) -> VocabularyEntry {
        let date = "2026-10-16".parse().unwrap();
        VocabularyEntry {
            seen: 2,
            committed: 1,
            used: 0,
            first: date,
            last: date,
            known: false,
            source: source.to_owned(),
            example: example.to_owned(),
        }
    }

    #[test]
    fn csv_quotes_fields_with_commas_and_quotes() {
        let entry = entry("开发", "他说：\"开发，很难\"。");
        let text = render(Format::Csv, "英语", &[("develop", &entry)]);
        assert!(text.starts_with('\u{feff}'));
        assert!(text.contains(
            "develop,开发,\"他说：\"\"开发，很难\"\"。\",2,1,0,2026-10-16,2026-10-16,\r\n"
        ));
    }

    #[test]
    fn anki_puts_the_word_in_front_and_escapes_html() {
        let entry = entry("小于", "a<b");
        let text = render(Format::Anki, "英语", &[("less", &entry)]);
        assert!(text.ends_with("less\t小于<br><br><i>a&lt;b</i>\n"));
    }
}
