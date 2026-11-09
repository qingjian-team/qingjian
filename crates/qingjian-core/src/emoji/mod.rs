//! 跟在候选词后面的 emoji、颜文字与符号（笑 → 😄，哭 → (T_T)，约等于 → ≈）。
//! emoji 主表由 Unicode CLDR 的中文 annotations 生成（`assets/emoji/emoji-zh.tsv`，`词\temoji emoji …`，`dict-convert emoji` 转换），
//! 其中的普通符号由 [`is_emoji`] 筛掉、改归符号表；口语叫法、颜文字、符号三张表编进程序（`builtin`）。

pub(crate) mod builtin;
mod items;
mod property;
mod skin_tone;
mod table;

pub use items::Items;
pub use property::is_emoji;
pub use skin_tone::skin_tone_variants;
pub use table::EmojiTable;
