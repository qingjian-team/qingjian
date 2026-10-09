//! 主题字族表（[`super::FontFamilies`]）里一条回退链的序号；0 是主题的缺省字族。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct FamilyId(pub(crate) u16);
