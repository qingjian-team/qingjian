//! Emoji / 符号开关、拼写纠错与输入统计、日志、面板历史。

use crate::emoji::{EmojiTable, builtin, is_emoji};
use crate::engine::{
    EMOJI_TOTAL, Engine, GlossFiller, InputLogger, UsageMeter, UsageSummary, VocabularyTracker,
};
use crate::panel::PanelHistory;

impl Engine {
    /// 带上 emoji 表：表里的普通符号（`≈` `¥`）筛掉、改由符号表出，并上内置的口语叫法；内置的颜文字、符号表与它们的词一起启用。
    pub fn with_emoji(mut self, mut table: EmojiTable) -> Self {
        table.retain(is_emoji);
        table.merge(builtin::emoji_supplement());
        self.emoji = Some(table);
        self.kaomoji = Some(&builtin::KAOMOJI);
        self.symbols = Some(&builtin::SYMBOLS);
        self.companion_words = Some(&builtin::WORDS);
        self.forget_span_cache();
        self
    }

    /// 每次查询最多插入几个 emoji（0–3）；0 关闭，越界值按 3 处理。
    /// 保留词表，重新开启时无需加载；不改变普通候选的排序。
    pub fn set_emoji_limit(&mut self, limit: usize) {
        self.settings.emoji_limit = limit.min(EMOJI_TOTAL);
    }

    /// 每次查询最多插入几个颜文字（0–3），规则同 [`Engine::set_emoji_limit`]。
    pub fn set_kaomoji_limit(&mut self, limit: usize) {
        self.settings.kaomoji_limit = limit.min(EMOJI_TOTAL);
    }

    /// 每次查询最多插入几个符号（0–3），规则同 [`Engine::set_emoji_limit`]。
    pub fn set_symbol_limit(&mut self, limit: usize) {
        self.settings.symbol_limit = limit.min(EMOJI_TOTAL);
    }

    /// 拼写纠错开关（配置 `[general] spelling_correction`，缺省开）。关掉后拼音只按敲的原样（与模糊音）读：
    /// 不做整段纠正，整句词图也不加敲错边，学来的敲错表不再起作用（表本身留着）。
    pub fn set_spelling_correction(&mut self, on: bool) {
        if self.settings.spelling_correction != on {
            self.settings.spelling_correction = on;
            *self.caches.correction.borrow_mut() = None;
            self.forget_span_cache();
        }
    }

    pub fn spelling_correction(&self) -> bool {
        self.settings.spelling_correction
    }

    /// 中西文之间是否自动补空格（配置 `[general] auto_space`，缺省关）。
    pub fn set_auto_space(&mut self, on: bool) {
        self.settings.auto_space = on;
    }

    pub fn auto_space(&self) -> bool {
        self.settings.auto_space
    }

    /// 中文模式下 Shift+字母是否进组句缓冲区（配置 `[general] shift_letter`，缺省关）。
    /// 开着时大写按小写参与匹配、原样上屏时还原，`Cpan` 与 `cpan` 一样出「C盘」；
    /// 关着时壳直接把大写字母交给应用，进这里的字母就按它自己的样子匹配。
    pub fn set_shift_letter_compose(&mut self, on: bool) {
        self.settings.shift_letter_compose = on;
    }

    pub fn shift_letter_compose(&self) -> bool {
        self.settings.shift_letter_compose
    }

    pub fn with_input_logger(mut self, logger: Box<dyn InputLogger>) -> Self {
        self.logger.replace(logger);
        self
    }

    /// 运行时换输入日志的落盘方（开关、清空之后）。旧的先 flush。
    pub fn set_input_logger(&mut self, logger: Box<dyn InputLogger>) {
        self.logger.flush();
        self.logger.replace(logger);
    }

    pub fn input_logger_mut(&mut self) -> &mut dyn InputLogger {
        self.logger.inner_mut()
    }

    /// 符号面板「常用」按这份记录排，从面板上屏的条目记进去（见 [`Self::commit`]）。
    pub fn with_panel_history(mut self, history: Box<dyn PanelHistory>) -> Self {
        self.panel_history = history;
        self
    }

    pub fn with_usage_meter(mut self, meter: Box<dyn UsageMeter>) -> Self {
        self.meter = meter;
        self
    }

    /// 上屏记不记进输入统计。壳把输入引到自己的工具框（快捷翻译的原文、常用语编辑框）时关掉：
    /// 那些字不进应用，算进「打了多少字」就虚高了；学习照常（选词是真的）。
    pub fn set_metered(&mut self, metered: bool) {
        self.settings.metered = metered;
    }

    /// 输入统计的汇总（偏好设置「统计」页）。
    pub fn usage_summary(&self) -> UsageSummary {
        self.meter.summary()
    }

    pub fn with_vocabulary_tracker(mut self, tracker: Box<dyn VocabularyTracker>) -> Self {
        self.vocabulary = tracker;
        self
    }

    pub fn with_gloss_filler(mut self, filler: Box<dyn GlossFiller>) -> Self {
        self.gloss_filler = filler;
        self
    }

    /// 运行时换释义兜底（随云联想开关）。
    pub fn set_gloss_filler(&mut self, filler: Box<dyn GlossFiller>) {
        self.gloss_filler = filler;
    }
}
