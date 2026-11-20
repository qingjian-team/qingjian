//! Engine 的输入方案：模糊音、双拼 / 注音 / 九宫格 / 形码，以及拼音侧开关。

use std::sync::Arc;

use crate::code_scheme::CodeScheme;
use crate::fuzzy::FuzzyRules;
use crate::shuangpin::Layout;

/// Engine 当前的输入方案与编码开关。
///
/// 换方案只动这一块，查询侧从 `self.scheme` 统一读。
pub(crate) struct EngineScheme {
    /// 模糊音开关，缺省全关。
    pub(crate) fuzzy: FuzzyRules,

    /// 双拼方案，`None` 为全拼。开着时缓冲区里是双拼键，查词前先解成全拼（见 `crate::shuangpin`）。
    pub(crate) shuangpin: Option<Arc<Layout>>,

    /// 注音模式开关，開著時緩衝區裡是注音大千鍵位，查詞前先解成拼音（見 `crate::zhuyin`）。
    pub(crate) zhuyin: bool,

    /// 九宫格方案：缓冲区里是数字键（可混着已选定的字母），见 `crate::nine_key`。
    pub(crate) nine_key: bool,

    /// 九宫格要保住的读法：(当时的缓冲区, 各音节在缓冲区里的起点与写法)。退格把选定的音节退回成数字、
    /// 点拼音行把光标挪进中间时记下；之后只删键或挪光标时候选与拼音行仍按这些写法排
    /// （`ni` 退回成 `64` 还读 ni，不换成默认的 mi），见 `unpin_nine_key`、`keep_nine_key_reading`。
    pub(crate) nine_key_unpinned: Option<(String, Vec<(usize, String)>)>,

    /// 形码方案（五笔或用户导入的）。`Some` 时编码参与查询，按前缀查表（见 `Engine::query_code`）。
    pub(crate) code: Option<CodeScheme>,

    /// 拼音侧（全拼 / 双拼 / 注音）参不参与查询，缺省参与。
    ///
    /// 与 `code` 组合出三种情形：只有拼音（形码关）、只有形码（拼音关，`[general] scheme = "none"`）、
    /// **两边都开 = 混输**（编码打全的形码候选在前，见 `Engine::query_mixed`）。两个都关着时按拼音走。
    pub(crate) phonetic: bool,
}

impl Default for EngineScheme {
    fn default() -> Self {
        Self {
            fuzzy: FuzzyRules::default(),
            shuangpin: None,
            zhuyin: false,
            nine_key: false,
            nine_key_unpinned: None,
            code: None,
            phonetic: true,
        }
    }
}
