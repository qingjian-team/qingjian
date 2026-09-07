//! 模糊音：把用户敲的音节扩展成若干等价写法（`zi` 也查 `zhi`，`lan` 也查 `nan`，`fen` 也查 `feng`），
//! 词库按「每个位置多种写法」一次查出，排序时模糊命中的词频减半（见 `ranking`）。
//!
//! 声母规则对完整音节和前缀 / 简拼都生效（前缀 `zh` 开了 z/zh 就换成 `z`，它本来就覆盖 `zh`）；
//! 韵母规则只对完整音节生效：末尾未打完的音节按前缀查，`fen` 自然能出 `feng`。

mod expanded;
mod rules;

pub use expanded::Expanded;
pub use rules::FuzzyRules;
