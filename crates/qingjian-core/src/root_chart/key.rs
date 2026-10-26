//! 字根图上的一个键。

use super::RootGlyph;

#[derive(Debug, Clone, PartialEq)]
pub struct RootKey {
    pub key: char,

    /// 区位号（11–55）；Z 是万能键，没有区位。
    pub zone: Option<u8>,

    pub roots: Vec<RootGlyph>,
}
