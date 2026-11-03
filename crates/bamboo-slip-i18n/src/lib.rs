//! 界面文案多语言。每个用到界面文字的 crate 各有一套文案表（每种语言一份 TOML，编译时嵌入），
//! 用 [`catalog!`] 声明成一个静态 [`Catalog`]；界面语言是进程级的一项设置（[`set_locale`]），所有文案表共用。
//!
//! 查词的回退链：界面语言 → 英文 → 源语言（声明里的第一份，所有键都必须有）→ 键名本身。
//! 文案表的 TOML 可以分节，`[page] title = "…"` 的键是 `page.title`；`{name}` 是参数，由 [`Catalog::format`] 填。
//! 每份表顶上的 `[meta] name` 是这门语言自己的叫法（「Tiếng Việt」），给语言选择列表用，不算文案。

#![forbid(unsafe_code)]

mod catalog;
mod locale;

pub use catalog::Catalog;
pub use locale::{locale, pin_locale, set_locale, system_locale};

/// 声明一个文案表：第一份是源语言，路径相对于写这个宏的文件。
///
/// ```ignore
/// bamboo_slip_i18n::catalog! {
///     pub static CATALOG = {
///         "zh-Hans" => "../locales/zh-Hans.toml",
///         "en" => "../locales/en.toml",
///     };
/// }
/// ```
#[macro_export]
macro_rules! catalog {
    ($vis:vis static $name:ident = { $($language:literal => $path:literal),+ $(,)? };) => {
        $vis static $name: $crate::Catalog = $crate::Catalog::new(&[$(($language, include_str!($path))),+]);
    };
}

#[cfg(test)]
mod tests;
