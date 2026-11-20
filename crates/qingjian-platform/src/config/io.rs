//! 配置文件读写：加载、原地改标量 / 数组键、模板落盘，以及旧外观写法（`theme` 写外观词）的迁移。

use std::path::Path;

use toml_edit::DocumentMut;

use super::{Appearance, CodeRules, Config, DEFAULT_THEME, TEMPLATE};
use crate::error::ConfigError;

impl Config {
    /// 用户给形码方案 `id` 改的按键规则；没改过时全是 `None`。
    pub fn code_rules(&self, id: &str) -> CodeRules {
        self.code.get(id).copied().unwrap_or_default()
    }

    /// 读配置。文件不存在按默认值；存在但解析失败报错，不要静默吞掉用户的笔误。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let config: Self = toml::from_str(&source).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 配置或环境变量里的密钥登记给日志掩码；各进程都从这里加载配置，登记在这一处就够
        if let Some(key) = config.predict.resolve_api_key() {
            crate::logs::secrets::register(&key);
        }
        // 界面语言同理：输入法进程里的候选菜单、提示与设置程序都按这一项
        bamboo_slip_i18n::set_locale(config.general.ui_locale());
        Ok(config)
    }

    /// 原地改一个布尔键，见 [`Self::set_value`]。
    pub fn set_bool(path: &Path, section: &str, key: &str, value: bool) -> Result<(), ConfigError> {
        Self::set_value(path, section, key, value)
    }

    /// 原地改一个键（`[section] key = value`），其余内容、注释与顺序原样保留：
    /// 菜单和设置窗口落盘都走这里。文件不存在时从模板起步；文件有语法错误就报错不写，
    /// 不能替用户「修复」成丢了注释的文件。
    pub fn set_value(
        path: &Path,
        section: &str,
        key: &str,
        value: impl Into<toml_edit::Value>,
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        // 旧写法（外观写在 theme 里）要在改键之前认出来：写的正是 theme 时，新值会把它盖掉
        let legacy = (section == "general" && matches!(key, "appearance" | "theme"))
            .then(|| legacy_appearance(&document))
            .flatten();
        section_table(&mut document, section)[key] = toml_edit::value(value);
        if let Some(old) = legacy {
            migrate_legacy_theme(&mut document, old);
        }
        // 写临时文件再改名：输入法进程随时可能被杀，不能留半个配置文件
        write_file(path, &document.to_string())
    }

    /// 原地把一个键改成字符串数组（`[section] key = ["a", "b"]`），其余内容、注释与顺序原样保留。
    /// 设置界面改词库列表（`[dictionaries] domains` / `disabled`）走这里，[`Self::set_value`] 只能写标量。
    pub fn set_array<S: AsRef<str>>(
        path: &Path,
        section: &str,
        key: &str,
        values: &[S],
    ) -> Result<(), ConfigError> {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let mut document: DocumentMut = source.parse().map_err(|source| ConfigError::Edit {
            path: path.to_owned(),
            source: Box::new(source),
        })?;
        let mut array = toml_edit::Array::new();
        for value in values {
            array.push(value.as_ref());
        }
        section_table(&mut document, section)[key] = toml_edit::value(array);
        write_file(path, &document.to_string())
    }

    /// 文件不存在时写出模板（目录一并建），返回是否写了。
    pub fn write_template_if_missing(path: &Path) -> Result<bool, ConfigError> {
        if path.exists() {
            return Ok(false);
        }
        write_file(path, TEMPLATE)?;
        Ok(true)
    }
}

/// 模板里 `appearance` 那一行上方的注释，旧配置迁移时照抄（有测试保证与 [`TEMPLATE`] 一致）。
pub(crate) const APPEARANCE_COMMENT: &str = "# 候选窗口外观：system 跟随系统 / light 浅色 / dark 深色\n";

/// 模板里 `theme` 那一行上方的注释，同上。
pub(crate) const THEME_COMMENT: &str = "# 候选窗口主题（主题 id）：qingjian 青简绿 / system-blue 系统蓝 / wechat 微信绿 / sujian 素笺 / sakura 樱花 / dafeiyu 大肥鱼 / miku 初音未来 / bocchi 波奇酱。只对青简渲染器生效\n";

/// 旧写法里 `theme` 写的外观；新写法（或没写）为 `None`。
fn legacy_appearance(document: &DocumentMut) -> Option<Appearance> {
    let value = document.get("general")?.get("theme")?.as_str()?;
    Appearance::from_key(value.trim())
}

/// 写 `appearance` 或 `theme` 时顺手迁移旧写法（2026-09-18 之前外观写在 `theme` 里，旧值是 `old`）：
/// 外观补写到 `appearance`（写的是 `theme` 时它还没有，不补就丢了），`theme` 还是外观词就换成内置主题，
/// 两行注释换成模板里的，免得手改配置的人看到「外观」注释下面是 `theme`。迁过之后 `theme` 不再是外观词，不会再迁。
fn migrate_legacy_theme(document: &mut DocumentMut, old: Appearance) {
    let Some(general) = document
        .get_mut("general")
        .and_then(toml_edit::Item::as_table_mut)
    else {
        return;
    };
    if !general.contains_key("appearance") {
        general["appearance"] = toml_edit::value(old.key());
    }
    let still_legacy = general
        .get("theme")
        .and_then(toml_edit::Item::as_str)
        .is_some_and(|value| Appearance::from_key(value.trim()).is_some());
    if still_legacy {
        general["theme"] = toml_edit::value(DEFAULT_THEME);
    }
    if let Some(mut key) = general.key_mut("theme") {
        key.leaf_decor_mut().set_prefix(THEME_COMMENT);
    }
    if let Some(mut key) = general.key_mut("appearance") {
        key.leaf_decor_mut().set_prefix(APPEARANCE_COMMENT);
    }
}

/// 原子写配置文件；数据目录还没有就先建（新账户第一次打开设置时输入法可能还没跑过）。
/// 取（没有就建）分节 `section`，`code.wubi86` 这样带点的是嵌套分节。
/// 一律建成标准表：否则 toml_edit 会写成顶层的行内表 `predict = { enabled = true }`；
/// 嵌套时外层是隐式的，文件里只出现 `[code.wubi86]`、不多出一个空的 `[code]`。
/// 用户手写的行内表（`fuzzy = { z_zh = true }`）原样转成标准表，同节其余的键都留着。
fn section_table<'a>(document: &'a mut DocumentMut, section: &str) -> &'a mut toml_edit::Table {
    let mut table = document.as_table_mut();
    let mut parts = section.split('.').peekable();
    while let Some(part) = parts.next() {
        if let Some(item) = table.get_mut(part).filter(|item| item.is_inline_table()) {
            if let Ok(mut child) = std::mem::take(item).into_table() {
                child.set_implicit(parts.peek().is_some());
                *item = toml_edit::Item::Table(child);
            }
            // `fuzzy = {` 里键后的空格属于行内写法，留着会写出 `[fuzzy ]`。
            if let Some(mut key) = table.key_mut(part) {
                key.leaf_decor_mut().clear();
            }
        }
        if !table.get(part).is_some_and(|item| item.is_table()) {
            let mut child = toml_edit::Table::new();
            child.set_implicit(parts.peek().is_some());
            table.insert(part, toml_edit::Item::Table(child));
        }
        table = table[part].as_table_mut().expect("刚确认过是表");
    }
    table
}

pub(crate) fn write_file(path: &Path, text: &str) -> Result<(), ConfigError> {
    let write = || {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        qingjian_core::storage::write_atomic_str(path, text)
    };
    write().map_err(|source| ConfigError::Write {
        path: path.to_owned(),
        source,
    })
}
