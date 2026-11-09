//! 一套辅码从哪来：导入的 `.qj` 还是辅码方案。

use std::path::PathBuf;

use crate::aux_schemes::AuxSchemeEntry;

/// 一套辅码的来处。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuxSource {
    /// `codes/` 里导入的 `.qj`。
    File(PathBuf),

    /// `schemes/` 里的辅码方案，码表外还带词码规则。
    Scheme(AuxSchemeEntry),
}
