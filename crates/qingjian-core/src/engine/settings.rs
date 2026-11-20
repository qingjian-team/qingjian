//! Engine 的配置开关：壳在装配与热加载时设置，查询与提交路径只读。
//!
//! 从 `Engine` 里拆出来，免得一个结构体同时装着词库、缓存、统计与二十多个开关。

use crate::engine::{DEFAULT_AUX_CODE_KEY, EMOJI_TOTAL, ENGLISH_CANDIDATES_MAX};
use qingjian_dictionary::WordList;

pub(crate) struct EngineSettings {
    /// 中文模式下整段是英文词（或像英文词的开头）时给不给英文候选与补全（配置 `[general] english_in_chinese`，缺省开）。
    pub(crate) english_in_chinese: bool,

    /// 中英混输时中文候选总在英文词前面（缺省关：拼音不像话的输入英文词排第一，常在中文模式里打英文词的人靠它）。
    pub(crate) chinese_first: bool,

    /// 拼写纠错（整段一处编辑的纠正与整句里的敲错边，含个人敲错表）开不开，配置 `[general] spelling_correction`，缺省开。
    pub(crate) spelling_correction: bool,

    /// 中文模式下英文候选（精确词 + 补全）最多几个（配置 `[general] english_max_in_chinese`，缺省 4）。
    pub(crate) english_max_in_chinese: usize,

    /// 英文候选旁显示音标（配置 `[general] show_english_pronunciation`，缺省关）。
    /// 开着时把音标填进 `Candidate::reading`，平台层的 annotation 会画在候选词后面。
    pub(crate) show_english_pronunciation: bool,

    /// 英文音标表（随包 `english-ipa.qj`），键是小写词、值是 `/ˈkɑrɡoʊ/`。没有就一律不显示音标。
    pub(crate) english_pronunciations: Option<WordList>,

    /// 有中文候选时英文最高排到第几位（配置 `[general] english_min_position`，缺省 1 即不限制）。
    pub(crate) english_min_position: usize,

    /// 中西文混排自动补空格（配置 `[general] auto_space`，缺省关）。见 [`crate::spacing`]。
    pub(crate) auto_space: bool,

    /// 中文模式下 Shift+字母进组句缓冲区（配置 `[general] shift_letter = "compose"`，缺省关）。
    /// 关着由壳直接把大写字母交给应用，Core 这一路就不该收——否则 `Cpan` 这种会被当成拼音。
    pub(crate) shift_letter_compose: bool,

    /// 辅码总开关（配置项 `[aux_code] enabled`，缺省关）：关着时触发键不进辅码态、纯拼音态也不挂码。
    pub(crate) aux_enabled: bool,

    /// 候选上是否显示码（配置项 `[general] aux_code_show`，缺省关）：纯拼音态逐候查首条码的短路开关，
    /// 由壳装配时告知（辅码态不受它管，看码有引导意义）。
    pub(crate) aux_show: bool,

    /// 拼音行按高亮候选显示带调拼音（配置项 `[general] show_tones`，缺省关），见 [`crate::engine::Engine::toned_syllables`]。
    pub(crate) show_tones: bool,

    /// 进辅码态的触发键，配置项 `[general] aux_code_key`，缺省 `;`。
    pub(crate) aux_code_key: char,

    /// 码段删空后是否留在辅码态（配置项 `[general] aux_code_keep_empty`，缺省开）：开 = 删空停在
    /// 辅码态（`;` 仍在、候选全回），空码段再按一次退格才退出；关 = 删空即回拼音态。
    pub(crate) aux_keep_empty: bool,

    /// 每次查询最多插几个 emoji（配置 `[general] emoji`，缺省 3）；0 不出 emoji，表照样加载。
    pub(crate) emoji_limit: usize,

    /// 每次查询最多插几个颜文字（配置 `[general] kaomoji`，缺省 1）。
    pub(crate) kaomoji_limit: usize,

    /// 每次查询最多插几个符号（配置 `[general] symbols`，缺省 1）。
    pub(crate) symbol_limit: usize,

    /// 是否学习连续上屏的五笔新词；不控制拼音、双拼学习或已有词的词频。
    pub(crate) auto_phrase: bool,

    /// 五笔连续组词窗口最多容纳的汉字数，范围为 2～10。
    pub(crate) auto_phrase_max_chars: usize,

    /// 双拼方案下 preedit 显示原始按键（如 `ljse`）还是展开成全拼（`lan'se`）。
    pub(crate) shuangpin_raw_preedit: bool,

    /// 双拼方案下候选窗口的拼音行显示原始按键还是全拼，与 `shuangpin_raw_preedit` 各管各的。
    pub(crate) shuangpin_raw_window: bool,

    /// 双拼方案下同时接受全拼（以及同一输入里两种打法的混用），见 [`crate::shuangpin`] 的混输一节。
    pub(crate) shuangpin_full_pinyin: bool,

    /// 选词后剩下拼音时先不上屏（[`crate::engine::Engine::set_hold_selections`]，缺省开）。
    pub(crate) hold_selections: bool,

    /// 上屏记不记进输入统计（[`crate::engine::Engine::set_metered`]）；壳的工具框里关掉。
    pub(crate) metered: bool,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            english_in_chinese: true,
            chinese_first: false,
            spelling_correction: true,
            english_max_in_chinese: ENGLISH_CANDIDATES_MAX,
            show_english_pronunciation: false,
            english_pronunciations: None,
            english_min_position: 1,
            auto_space: false,
            shift_letter_compose: false,
            aux_enabled: false,
            aux_show: false,
            show_tones: false,
            aux_code_key: DEFAULT_AUX_CODE_KEY,
            aux_keep_empty: true,
            emoji_limit: EMOJI_TOTAL,
            kaomoji_limit: 1,
            symbol_limit: 1,
            auto_phrase: true,
            auto_phrase_max_chars: 4,
            shuangpin_raw_preedit: false,
            shuangpin_raw_window: false,
            shuangpin_full_pinyin: false,
            hold_selections: true,
            metered: true,
        }
    }
}
