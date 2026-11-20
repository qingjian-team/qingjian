//! 编码方案与辅码：拼音侧方案、形码侧方案、双拼显示选项、辅码触发键与三条不再写出去的旧键。
//!
//! 这一组是 [`super::GeneralConfig`] 的扁平子集，序列化时由 `#[serde(flatten)]` 并回
//! `[general]` 分节，键名与旧扁平结构完全一致。

use serde::{Deserialize, Serialize};

/// 编码方案与辅码设置（`[general]` 下的方案相关键）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSchemes {
    /// 辅码触发键：拼音打完之后敲它进辅码态，缺省 `;`。校验 = 单字符、ASCII 可打印、
    /// 非字母数字、非翻页键（见 [`qingjian_core::is_valid_aux_code_key`]）。
    pub aux_code_key: String,

    /// 候选上是否显示码（与译文拼成一条注记）。缺省关：竖排会挤、横排更难放下。
    pub aux_code_show: bool,

    /// 码段删空后是否留在辅码态：开（缺省）时 `;` 仍在、候选全部回来，空码段再按一次退格才退出；
    /// 关则删空即回纯拼音态。
    pub aux_code_keep_empty: bool,

    /// 拼音侧方案：`pinyin`（全拼，缺省）/ `xiaohe` / `ziranma` / `microsoft` / `sogou` / `abc` / `xiaolang` / `shoudao`
    /// / `jiajia` / `ziguang` / `zhuyin`
    /// / `none`（关，只用形码），见 [`super::scheme::Scheme`]。用不认识的写法时按全拼并警告。
    /// 缺省是空串：文件里没写这一项时要去看旧键，见 [`super::GeneralConfig::scheme`]。
    pub scheme: String,

    /// 双拼方案下 preedit 显示原始按键字母（如 `ljse`）还是展开成全拼音节（`lan'se`）。
    /// 缺省关（展开成全拼音节）；常在双拼中打英文词或需要核对按键的人可打开。
    pub shuangpin_raw_preedit: bool,

    /// 双拼方案下候选窗口的拼音行显示原始按键还是全拼，与 `shuangpin_raw_preedit` 各管各的，两项组合出四种显示。缺省关（全拼）。
    pub shuangpin_raw_window: bool,

    /// 双拼方案下同时接受全拼（同一段输入里两种打法也能混用）。缺省关：只用选定的双拼方案。
    /// 开着时家里有人打双拼、有人打全拼可以共用一份设置。
    pub shuangpin_full_pinyin: bool,

    /// 形码侧方案：`off` 为关，`wubi86` 为内置五笔，其他是用户目录 `schemes/<id>/` 下的方案（见 [`crate::code_schemes`]）。
    /// **与拼音同时开着就是混输**，见 [`super::GeneralConfig::mixed`]。缺省是空串：文件里没写这一项时要去看旧键，见 [`super::GeneralConfig::code`]。
    pub code: String,

    /// 旧键（2026-10-20 之前的 `[general] wubi`，空串为关、`wubi86` 为开）：只在 [`super::GeneralConfig::code`] 里用，不再写出去。
    pub wubi: Option<String>,

    /// 旧键（2026-09-16 之前是 `[general] shuangpin`，空串为全拼）：只在 [`super::GeneralConfig::scheme`] 里用来推断方案，
    /// 不再写出去；`scheme` 写了值就不看它。当时 `shuangpin` 与 `zhuyin` 是两个字段表达同一个维度。
    pub shuangpin: Option<String>,

    /// 旧键（同上的 `[general] zhuyin`）：同上。
    pub zhuyin: Option<bool>,
}

impl Default for GeneralSchemes {
    fn default() -> Self {
        Self {
            aux_code_key: qingjian_core::DEFAULT_AUX_CODE_KEY.to_string(),
            aux_code_show: false,
            aux_code_keep_empty: true,
            scheme: String::new(),
            shuangpin_raw_preedit: false,
            shuangpin_raw_window: false,
            shuangpin_full_pinyin: false,
            code: String::new(),
            wubi: None,
            shuangpin: None,
            zhuyin: None,
        }
    }
}
