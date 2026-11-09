//! 设置的导出与导入（换电脑、备份用）：只搬设置，不带自定义短语（短语另有自己的导入导出）、云服务密钥
//! 与悬浮状态条在这台屏幕上的位置。导入前先当配置读一遍，读不通就不动现有文件。

use toml_edit::{DocumentMut, Item};

use super::Config;

use crate::i18n::tr_args;

/// 只属于这台电脑、导出时去掉、导入时保留本机原值的项（分节，键）；`custom_phrases` 是整张表。
const LOCAL: &[(&str, &str)] = &[
    ("predict", "api_key"),
    ("status_bar", "x"),
    ("status_bar", "y"),
];

/// 自定义短语的表名。
const PHRASES: &str = "custom_phrases";

/// 把配置文件的内容变成导出文件：注释与写法照旧，去掉短语、密钥与状态条位置。
pub fn export_settings(source: &str) -> Result<String, String> {
    let mut document = parse(source)?;
    document.remove(PHRASES);
    for (section, key) in LOCAL {
        if let Some(table) = document.get_mut(section).and_then(Item::as_table_like_mut) {
            table.remove(key);
        }
    }
    Ok(format!(
        "# 青简设置，在设置程序「高级」页导入；不含自定义短语、云服务密钥与状态条位置\n\n{document}"
    ))
}

/// 用导入的设置换掉现有配置：`current` 里的短语、密钥与状态条位置留着，其余照 `imported`。
/// 导入的内容读不成配置时报错，返回的新内容保证能读。
pub fn import_settings(current: &str, imported: &str) -> Result<String, String> {
    let mut document = parse(imported)?;
    toml::from_str::<Config>(imported)
        .map_err(|e| tr_args("settings_file.invalid", &[("error", &e.message())]))?;
    let current = parse(current)?;
    document.remove(PHRASES);
    if let Some(phrases) = current.get(PHRASES) {
        document.insert(PHRASES, phrases.clone());
    }
    for (section, key) in LOCAL {
        if let Some(table) = document.get_mut(section).and_then(Item::as_table_like_mut) {
            table.remove(key);
        }
        let kept = current
            .get(section)
            .and_then(Item::as_table_like)
            .and_then(|table| table.get(key))
            .cloned();
        if let Some(value) = kept {
            if document.get(section).is_none() {
                document.insert(section, Item::Table(toml_edit::Table::new()));
            }
            if let Some(table) = document.get_mut(section).and_then(Item::as_table_like_mut) {
                table.insert(key, value);
            }
        }
    }
    let merged = document.to_string();
    toml::from_str::<Config>(&merged)
        .map_err(|e| tr_args("settings_file.merge_failed", &[("error", &e.message())]))?;
    Ok(merged)
}

fn parse(source: &str) -> Result<DocumentMut, String> {
    source
        .parse::<DocumentMut>()
        .map_err(|e| tr_args("settings_file.unreadable", &[("error", &e.message())]))
}

#[cfg(test)]
mod tests {
    use super::{export_settings, import_settings};

    const HERE: &str = "# 我的注释\n[general]\npage_size = 5\n\n[predict]\napi_key = \"sk-here\"\n\n[status_bar]\nx = 10\ny = 20\n\n[[custom_phrases]]\ncode = \"dz\"\ntext = \"地址\"\n";

    #[test]
    fn export_drops_phrases_secrets_and_screen_positions() {
        let exported = export_settings(HERE).unwrap();
        assert!(exported.contains("# 我的注释"));
        assert!(exported.contains("page_size = 5"));
        for gone in ["sk-here", "x = 10", "custom_phrases", "地址"] {
            assert!(!exported.contains(gone), "{gone}");
        }
    }

    #[test]
    fn import_keeps_local_phrases_secrets_and_positions() {
        let other = "[general]\npage_size = 7\n\n[predict]\napi_key = \"sk-other\"\n\n[[custom_phrases]]\ncode = \"ee\"\ntext = \"：\"\n";
        let merged = import_settings(HERE, other).unwrap();
        let config: crate::Config = toml::from_str(&merged).unwrap();
        assert_eq!(config.general.page_size(), 7);
        assert_eq!(config.predict.api_key.as_deref(), Some("sk-here"));
        assert_eq!(
            (config.status_bar.x, config.status_bar.y),
            (Some(10), Some(20))
        );
        assert_eq!(config.custom_phrases.len(), 1);
        assert_eq!(config.custom_phrases[0].code, "dz");
        assert!(import_settings(HERE, "[general]\npage_size = \"多\"\n").is_err());
        assert!(import_settings(HERE, "不是 toml").is_err());
    }
}
