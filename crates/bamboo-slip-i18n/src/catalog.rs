//! 一套文案表：每种语言一份嵌入的 TOML，第一次查词时整套解析好，之后按界面语言查。

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

use crate::locale;

/// 界面语言缺的键先退到英文再退到源语言：译者还没跟上的新文案，外国用户看英文比看中文好懂。
const FALLBACK: &str = "en";

pub struct Catalog {
    /// (语言标签, TOML 原文)；第一份是源语言。
    files: &'static [(&'static str, &'static str)],

    /// 与 `files` 同序，每份展平成「点分键 → 文案」。
    tables: OnceLock<Vec<HashMap<String, String>>>,
}

impl Catalog {
    pub const fn new(files: &'static [(&'static str, &'static str)]) -> Self {
        Self {
            files,
            tables: OnceLock::new(),
        }
    }

    /// 按界面语言查文案，查不到依次退到英文、源语言；都没有就返回键名，界面上一眼能看出漏了哪条。
    pub fn get(&'static self, key: &'static str) -> &'static str {
        let tables = self.tables();
        let wanted = self.index(locale());
        let fallback = self.files.iter().position(|(tag, _)| *tag == FALLBACK);
        [wanted, fallback, Some(0)]
            .into_iter()
            .flatten()
            .find_map(|index| tables[index].get(key))
            .map_or(key, String::as_str)
    }

    /// 查文案后把 `{name}` 换成参数。
    pub fn format(&'static self, key: &'static str, args: &[(&str, &dyn Display)]) -> String {
        let mut text = self.get(key).to_owned();
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), &value.to_string());
        }
        text
    }

    /// 这个键在各语言里的文案（去重）：按标题找窗口这类事，界面语言改过以后旧窗口还是旧标题。
    pub fn variants(&'static self, key: &str) -> Vec<&'static str> {
        let mut texts: Vec<&'static str> = Vec::new();
        for text in self.tables().iter().filter_map(|table| table.get(key)) {
            if !texts.contains(&text.as_str()) {
                texts.push(text);
            }
        }
        texts
    }

    /// 有哪些语言：(标签, 这门语言自己的叫法)，源语言在前。
    pub fn languages(&'static self) -> Vec<(&'static str, &'static str)> {
        let tables = self.tables();
        self.files
            .iter()
            .zip(tables)
            .map(|((tag, _), table)| (*tag, table.get("meta.name").map_or(*tag, String::as_str)))
            .collect()
    }

    /// 给测试用：各份能不能解析、有没有源语言没有的废键、有没有源语言没有的占位参数。
    /// 别的语言缺键不算错（回退到英文 / 源语言），英文缺键算，它是回退链的一环。
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut tables = Vec::new();
        for (tag, text) in self.files {
            match parse(text) {
                Ok(table) => tables.push(table),
                Err(error) => {
                    problems.push(format!("{tag}: {error}"));
                    tables.push(HashMap::new());
                }
            }
        }
        let Some((source, rest)) = tables.split_first() else {
            return problems;
        };
        for ((tag, _), table) in self.files[1..].iter().zip(rest) {
            for (key, text) in table {
                match source.get(key) {
                    None => problems.push(format!("{tag}: unknown key {key}")),
                    // 译文可以按语言习惯省掉某个参数，但不能用原文没有的
                    Some(original)
                        if placeholders(text)
                            .iter()
                            .any(|name| !placeholders(original).contains(name)) =>
                    {
                        problems.push(format!("{tag}: unknown placeholder in {key}"));
                    }
                    Some(_) => {}
                }
            }
            if *tag == FALLBACK {
                problems.extend(
                    source
                        .keys()
                        .filter(|key| !table.contains_key(*key))
                        .map(|key| format!("{tag}: missing key {key}")),
                );
            }
        }
        problems.sort();
        problems
    }

    fn tables(&self) -> &Vec<HashMap<String, String>> {
        self.tables.get_or_init(|| {
            self.files
                .iter()
                .map(|(tag, text)| {
                    parse(text).unwrap_or_else(|error| {
                        tracing::error!(language = tag, %error, "Failed to parse UI text catalog");
                        HashMap::new()
                    })
                })
                .collect()
        })
    }

    /// 界面语言对应这套表的第几份：先整个标签，再去掉地区（`vi-VN` → `vi`）。
    fn index(&self, wanted: &str) -> Option<usize> {
        let find = |tag: &str| {
            self.files
                .iter()
                .position(|(own, _)| own.eq_ignore_ascii_case(tag))
        };
        find(wanted).or_else(|| find(wanted.split('-').next()?))
    }
}

fn parse(text: &str) -> Result<HashMap<String, String>, toml::de::Error> {
    let root: toml::Table = toml::from_str(text)?;
    let mut table = HashMap::new();
    flatten(&mut table, String::new(), root);
    Ok(table)
}

fn flatten(out: &mut HashMap<String, String>, prefix: String, table: toml::Table) {
    for (key, value) in table {
        let key = if prefix.is_empty() {
            key
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::Table(inner) => flatten(out, key, inner),
            toml::Value::String(text) => {
                out.insert(key, text);
            }
            other => {
                out.insert(key, other.to_string());
            }
        }
    }
}

fn placeholders(text: &str) -> Vec<&str> {
    let mut names: Vec<&str> = text
        .split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str =
        "meta.name = \"简体中文\"\napp.title = \"青简\"\ngreet = \"你好，{name}\"\n";
    const ENGLISH: &str =
        "meta.name = \"English\"\napp.title = \"Qingjian\"\ngreet = \"Hello, {name}\"\n";

    static FILES: &[(&str, &str)] = &[("zh-Hans", SOURCE), ("en", ENGLISH)];
    static CATALOG: Catalog = Catalog::new(FILES);

    #[test]
    fn index_matches_the_whole_tag_ignoring_case() {
        assert_eq!(CATALOG.index("zh-Hans"), Some(0));
        assert_eq!(CATALOG.index("EN"), Some(1));
        assert_eq!(CATALOG.index("fr"), None);
    }

    #[test]
    fn index_falls_back_to_the_language_part() {
        assert_eq!(CATALOG.index("en-US"), Some(1));
        assert_eq!(CATALOG.index("vi-VN"), None);
        assert_eq!(CATALOG.index("zh-Hans-CN"), None);
    }

    #[test]
    fn parse_flattens_nested_tables() {
        let table = parse("[window]\ntitle = \"青简\"\n[window.menu]\nfile = \"文件\"\n").unwrap();
        assert_eq!(table.get("window.title").map(String::as_str), Some("青简"));
        assert_eq!(
            table.get("window.menu.file").map(String::as_str),
            Some("文件")
        );
    }

    #[test]
    fn parse_stores_non_string_values_as_text() {
        let table = parse("count = 3\nsize = 1.5\nflag = true\n").unwrap();
        assert_eq!(table.get("count").map(String::as_str), Some("3"));
        assert_eq!(table.get("size").map(String::as_str), Some("1.5"));
        assert_eq!(table.get("flag").map(String::as_str), Some("true"));
    }

    #[test]
    fn parse_reports_broken_toml() {
        assert!(parse("= 1").is_err());
        assert!(parse("a = \"未闭合").is_err());
    }

    #[test]
    fn variants_dedupe_across_languages() {
        static DUP: &[(&str, &str)] = &[
            ("zh-Hans", "a = \"同一个\"\nb = \"源\""),
            ("en", "a = \"同一个\"\nb = \"other\""),
        ];
        static DUP_CATALOG: Catalog = Catalog::new(DUP);
        assert_eq!(DUP_CATALOG.variants("a"), vec!["同一个"]);
        assert_eq!(DUP_CATALOG.variants("b"), vec!["源", "other"]);
        assert!(DUP_CATALOG.variants("missing").is_empty());
    }

    #[test]
    fn languages_use_their_own_names() {
        assert_eq!(
            CATALOG.languages(),
            vec![("zh-Hans", "简体中文"), ("en", "English")]
        );
    }

    #[test]
    fn languages_fall_back_to_the_tag() {
        static BARE: &[(&str, &str)] = &[("zh-Hans", "a = \"x\"")];
        static BARE_CATALOG: Catalog = Catalog::new(BARE);
        assert_eq!(BARE_CATALOG.languages(), vec![("zh-Hans", "zh-Hans")]);
    }

    #[test]
    fn format_replaces_every_placeholder() {
        let text = CATALOG.format("greet", &[("name", &"小明")]);
        assert!(!text.contains("{name}"));
        assert!(text.contains("小明"));
    }

    #[test]
    fn problems_accepts_consistent_catalogs() {
        assert!(CATALOG.problems().is_empty(), "{:?}", CATALOG.problems());
    }

    #[test]
    fn problems_flags_an_unknown_key() {
        static BAD: &[(&str, &str)] = &[("zh-Hans", "a = \"x\""), ("en", "a = \"x\"\nb = \"y\"")];
        static BAD_CATALOG: Catalog = Catalog::new(BAD);
        assert_eq!(BAD_CATALOG.problems(), vec!["en: unknown key b"]);
    }

    #[test]
    fn problems_flags_an_unknown_placeholder() {
        static BAD: &[(&str, &str)] = &[("zh-Hans", "a = \"x\""), ("en", "a = \"x {who}\"")];
        static BAD_CATALOG: Catalog = Catalog::new(BAD);
        assert_eq!(BAD_CATALOG.problems(), vec!["en: unknown placeholder in a"]);
    }

    #[test]
    fn problems_flags_english_missing_a_key() {
        static BAD: &[(&str, &str)] = &[("zh-Hans", "a = \"x\"\nb = \"y\""), ("en", "a = \"x\"")];
        static BAD_CATALOG: Catalog = Catalog::new(BAD);
        assert_eq!(BAD_CATALOG.problems(), vec!["en: missing key b"]);
    }

    #[test]
    fn problems_allow_other_languages_to_skip_a_key() {
        static FINE: &[(&str, &str)] = &[
            ("zh-Hans", "a = \"x\"\nb = \"y\""),
            ("en", "a = \"x\"\nb = \"y\""),
            ("ja", "a = \"x\""),
        ];
        static FINE_CATALOG: Catalog = Catalog::new(FINE);
        assert!(
            FINE_CATALOG.problems().is_empty(),
            "{:?}",
            FINE_CATALOG.problems()
        );
    }

    #[test]
    fn problems_report_a_broken_source() {
        static BAD: &[(&str, &str)] = &[("zh-Hans", "= 1")];
        static BAD_CATALOG: Catalog = Catalog::new(BAD);
        let problems = BAD_CATALOG.problems();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("zh-Hans: "), "{problems:?}");
    }
}
