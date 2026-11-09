//! 形码与双拼方案共用的部分：`scheme.toml` 的元信息，与壳手里「现在装着哪套方案」。

mod meta;
mod slot;

pub use meta::SchemeMeta;
pub use slot::SchemeSlot;
