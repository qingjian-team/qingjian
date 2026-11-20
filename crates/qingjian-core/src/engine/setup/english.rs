//! 英文模式、英文词表与英→中释义表。

use crate::engine::{ENGLISH_CANDIDATES_MAX, Engine, ModeKeys, Translator};
use crate::history::InputHistory;
use qingjian_dictionary::WordList;

impl Engine {
    pub fn history(&self) -> &InputHistory {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut InputHistory {
        &mut self.history
    }

    /// 进入 / 离开英文模式。英文模式下 [`Self::query`] 只给英文词表的候选，回车与空格仍由壳原样上屏敲的字母，
    /// 不发云联想，也不把原样上屏记成「不纠这个串」。
    pub fn set_english_mode(&mut self, on: bool) {
        self.english_mode = on;
    }

    pub fn english_mode(&self) -> bool {
        self.english_mode
    }

    /// 随包英文词表（不含用户打过的词）；没装是 `None`。
    pub fn english_words(&self) -> Option<&WordList> {
        self.english.as_ref()
    }

    pub fn with_english(mut self, words: WordList) -> Self {
        self.english = Some(words);
        self
    }

    /// 接英文候选用的释义表（英→中）。
    pub fn with_english_translator(mut self, translator: Box<dyn Translator>) -> Self {
        self.models.english_translator = translator;
        self
    }

    /// 运行时换英→中释义表（下载装上之后）。
    pub fn set_english_translator(&mut self, translator: Box<dyn Translator>) {
        self.models.english_translator = translator;
    }

    pub fn with_mode_keys(mut self, keys: ModeKeys) -> Self {
        self.modes = keys.sanitized();
        self
    }

    /// 非法组合（相同、或不是 v / u / i）整个退回缺省。
    pub fn set_mode_keys(&mut self, keys: ModeKeys) {
        self.modes = keys.sanitized();
    }

    pub fn mode_keys(&self) -> ModeKeys {
        self.modes
    }

    /// 中文模式下给不给英文词与补全（配置 `[general] english_in_chinese`，缺省开）；英文模式不受影响。
    pub fn set_english_in_chinese(&mut self, on: bool) {
        self.settings.english_in_chinese = on;
    }

    pub fn english_in_chinese(&self) -> bool {
        self.settings.english_in_chinese
    }

    /// 中文模式下英文候选（精确词 + 补全）最多几个（配置 `[general] english_max_in_chinese`，缺省
    /// [`ENGLISH_CANDIDATES_MAX`]）。1 只留最靠前的一条，越界按 1 与上限收拢。
    pub fn set_english_max_in_chinese(&mut self, max: usize) {
        self.settings.english_max_in_chinese = max.clamp(1, ENGLISH_CANDIDATES_MAX);
    }

    /// 有中文候选时英文最高排到第几位（配置 `[general] english_min_position`，缺省 1 即不限制）。
    /// 没有中文候选时英文照旧排第一；范围 1–9，越界收拢。
    pub fn set_english_min_position(&mut self, position: usize) {
        self.settings.english_min_position = position.clamp(1, 9);
    }

    /// 中英混输里中文候选是否总排在英文词前面（配置 `[general] chinese_first`，缺省关）。
    /// 关着时拼音「不像话」的输入英文词排第一（`hello` 先英文再 荷兰咯）；开了英文词固定第二。
    pub fn set_chinese_first(&mut self, on: bool) {
        self.settings.chinese_first = on;
    }

    pub fn chinese_first(&self) -> bool {
        self.settings.chinese_first
    }

    /// 英文候选旁显示音标（配置 `[general] show_english_pronunciation`，缺省关）。
    /// 关着时英文候选不带音标，与加这个功能之前完全一样。
    pub fn set_show_english_pronunciation(&mut self, on: bool) {
        self.settings.show_english_pronunciation = on;
    }

    pub fn show_english_pronunciation(&self) -> bool {
        self.settings.show_english_pronunciation
    }

    /// 英文音标表（随包 `english-ipa.qj`，`WordList::pronunciations_from_path` 读）。没设或设为空就不显示任何音标。
    /// 重复设置会换掉旧表——热加载时改的是同一个文件。
    pub fn set_english_pronunciations(&mut self, table: Option<WordList>) {
        self.settings.english_pronunciations = table;
    }

    pub fn english_pronunciations(&self) -> Option<&WordList> {
        self.settings.english_pronunciations.as_ref()
    }
}
