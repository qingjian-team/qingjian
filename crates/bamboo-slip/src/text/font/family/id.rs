//! 字族表（[`super::FamilyTable`]）里一条回退链的序号；0 是缺省字族。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FamilyId(pub u16);
