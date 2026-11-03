//! 界面文案：`locales/` 下每种语言一份。

use std::fmt::Display;

bamboo_slip_i18n::catalog! {
    pub static CATALOG = {
        "zh-Hans" => "../locales/zh-Hans.toml",
        "en" => "../locales/en.toml",
    };
}

pub fn tr(key: &'static str) -> &'static str {
    CATALOG.get(key)
}

/// 带参数的文案，`{name}` 换成对应的值。
pub fn tr_args(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
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
