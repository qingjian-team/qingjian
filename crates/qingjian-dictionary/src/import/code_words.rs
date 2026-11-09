//! 导入形码词库：用户自己的五笔词条（`词\t编码\t词频`，或第二列是编码的 Rime `.dict.yaml`），
//! 规整成 TSV 存进形码词库目录，载入方案时跟随包码表合在一起。
//!
//! 文件第一行记着是给哪套形码方案的（`# scheme: wubi86`）：别的方案编码对不上，不往里合。

use std::io::{BufRead, BufReader};
use std::path::Path;

use super::{Imported, rime, strip_extensions};
use crate::code_table::CodeTable;
use crate::error::DictionaryError;

const SCHEME_HEADER: &str = "# scheme:";

/// 把 `source` 导入到 `dest_dir`，记成方案 `scheme` 的词；同名的覆盖。
pub fn import_code_words(
    source: &Path,
    dest_dir: &Path,
    scheme: &str,
) -> Result<Imported, DictionaryError> {
    let stem = source
        .file_name()
        .and_then(|n| n.to_str())
        .map(strip_extensions)
        .filter(|s| !s.is_empty())
        .ok_or(DictionaryError::Corrupt("source has no usable file name"))?;
    let text = std::fs::read_to_string(source)?;
    let (tsv, name) = if rime::looks_like_rime(&text) {
        let parsed = rime::to_tsv(&text);
        (parsed.tsv, parsed.name)
    } else {
        (text, None)
    };
    let table = CodeTable::parse(&tsv)?;
    if table.is_empty() {
        return Err(DictionaryError::NoCodeEntries);
    }
    let mut out = format!("{SCHEME_HEADER} {scheme}\n");
    for line in tsv.lines().map(str::trim) {
        if !line.is_empty() && !line.starts_with('#') {
            out.push_str(line);
            out.push('\n');
        }
    }
    std::fs::create_dir_all(dest_dir)?;
    let target = dest_dir.join(format!("{stem}.tsv"));
    std::fs::write(&target, out)?;
    Ok(Imported {
        path: target,
        name: name.unwrap_or(stem),
        entries: table.len(),
    })
}

/// 形码词库是给哪套方案的：读第一行；没写的是 86 五笔（手放进目录的）。
pub fn code_words_scheme(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut first = String::new();
    BufReader::new(file).read_line(&mut first).ok()?;
    Some(
        first
            .trim()
            .strip_prefix(SCHEME_HEADER)
            .map_or("wubi86", str::trim)
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_wubi_words_with_the_scheme_header() {
        let dir = std::env::temp_dir().join(format!("qingjian-code-words-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("wubi-test.tsv");
        std::fs::write(&source, "# 形码码表\r\n拉混\truji\t1\r\n青简\tgetu\t1\r\n").unwrap();
        let imported = import_code_words(&source, &dir.join("code_dicts"), "wubi86").unwrap();
        assert_eq!(imported.path.file_name().unwrap(), "wubi-test.tsv");
        assert_eq!(imported.entries, 2);
        assert_eq!(code_words_scheme(&imported.path).as_deref(), Some("wubi86"));
        let table = CodeTable::from_path(&imported.path).unwrap();
        assert_eq!(table.lookup("getu", 10)[0].text, "青简");

        let bare = dir.join("bare.tsv");
        std::fs::write(&bare, "青简\tgetu\t1\n").unwrap();
        assert_eq!(code_words_scheme(&bare).as_deref(), Some("wubi86"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
