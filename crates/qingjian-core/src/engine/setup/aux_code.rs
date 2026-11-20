//! 辅码码表、触发键与开关。

use crate::engine::aux_code::is_valid_aux_code_key;
use crate::engine::{DEFAULT_AUX_CODE_KEY, Engine};
use qingjian_dictionary::AuxCodeLookup;
use std::sync::Arc;

impl Engine {
    /// 挂上辅码码表（用户导入的与随包的笔画表）。照 [`Self::set_extra_dictionaries`] 的模式：壳按目录与配置装配。
    pub fn with_aux_codes(mut self, tables: Vec<Arc<dyn AuxCodeLookup>>) -> Self {
        self.aux_codes = tables;
        self
    }

    /// 运行时换码表（导入 / 移除 / 开关之后）；顺带退出辅码态，免得筛的是一张已经不在的表。
    pub fn set_aux_codes(&mut self, tables: Vec<Arc<dyn AuxCodeLookup>>) {
        self.aux_codes = tables;
        self.aux_code = None;
    }

    pub fn aux_codes(&self) -> &[Arc<dyn AuxCodeLookup>] {
        &self.aux_codes
    }

    /// 换辅码触发键（配置项 `[general] aux_code_key`）。非法的键（含当前翻页键 `page_keys`，
    /// 配置项 `[general] page_keys`）退回缺省 `;`。
    pub fn set_aux_code_key(&mut self, key: char, page_keys: (char, char)) {
        self.aux_code_key = if is_valid_aux_code_key(key, page_keys) {
            key
        } else {
            DEFAULT_AUX_CODE_KEY
        };
    }

    /// 换「码删空后留在辅码态」开关（配置项 `[general] aux_code_keep_empty`，缺省开）。
    pub fn set_aux_keep_empty(&mut self, keep: bool) {
        self.aux_keep_empty = keep;
    }

    /// 换辅码总开关（配置项 `[aux_code] enabled`，缺省关）。关掉时辅码整线关：触发键不进辅码态，
    /// 纯拼音态也不逐候查首条码；开着但没有码表（`aux_codes` 为空）同样不进辅码态。
    pub fn set_aux_enabled(&mut self, enabled: bool) {
        self.aux_enabled = enabled;
    }

    /// 换「候选上显示码」开关（配置项 `[general] aux_code_show`，缺省关）：纯拼音态挂不挂首条码。
    pub fn set_aux_show(&mut self, show: bool) {
        self.aux_show = show;
    }
}
