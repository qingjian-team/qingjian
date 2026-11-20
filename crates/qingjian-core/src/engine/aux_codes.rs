//! Engine 的辅码状态与码表。

use std::sync::Arc;

use qingjian_dictionary::AuxCodeLookup;

/// 辅码：当前码段与已装配的码表。
#[derive(Default)]
pub(crate) struct EngineAux {
    /// 辅码态：`None` 是拼音态，`Some` 是辅码态（空串 = 刚敲下触发键、码段还没开始）。
    /// 码段不进 `composition`：它与拼音分段记账、分段画（见 `AuxSegment`）。
    pub(crate) aux_code: Option<String>,

    /// 辅码码表，壳按用户目录 `codes/` 与配置装配；空表示没装码表（辅码态筛不出任何词）。
    pub(crate) aux_codes: Vec<Arc<dyn AuxCodeLookup>>,
}
