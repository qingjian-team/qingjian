//! 本 crate 给用户看的文字（选项名、报错）：`locales/` 下每种语言一份，界面语言由宿主进程设（`bamboo_slip_i18n::set_locale`）。

use std::fmt::Display;

bamboo_slip_i18n::catalog! {
    pub static CATALOG = {
        "zh-Hans" => "../locales/zh-Hans.toml",
        "zh-Hant" => "../locales/zh-Hant.toml",
        "en" => "../locales/en.toml",
        "ja" => "../locales/ja.toml",
        "es" => "../locales/es.toml",
        "vi" => "../locales/vi.toml",
        "de" => "../locales/de.toml",
    };
}

pub(crate) fn tr(key: &'static str) -> &'static str {
    CATALOG.get(key)
}

/// 带参数的文案，`{name}` 换成对应的值。
pub(crate) fn tr_args(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
    CATALOG.format(key, args)
}

#[cfg(test)]
mod tests {
    use super::CATALOG;

    #[test]
    fn catalogs_are_consistent() {
        let problems = CATALOG.problems();
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
