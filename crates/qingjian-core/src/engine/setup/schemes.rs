//! 输入方案与按键规则：注音、九键、繁体、形码 / 拼音开关，以及模式键的判断。

use crate::code_scheme::CodeScheme;
use crate::engine::{Engine, ModeKeys};
use crate::traditional::{Traditional, TraditionalVariant};
use qingjian_dictionary::CodeTable;

impl Engine {
    /// 换「候选上显示声调」开关（配置项 `[general] show_tones`，缺省关）。
    pub fn set_show_tones(&mut self, show: bool) {
        self.settings.show_tones = show;
    }

    /// 应用读不到光标前文时，本应用多少秒内上屏的字还当前文（配置项 `[model] context_memory_seconds`，0 表示一直算）。
    pub fn set_context_memory(&mut self, seconds: u64) {
        self.history.set_fresh_for(seconds);
    }

    /// 設置是否啟用注音模式。開啟後鍵盤輸入按大千佈局解析。
    pub fn set_zhuyin_mode(&mut self, on: bool) {
        if self.zhuyin != on {
            self.chain.clear_code_words();
        }
        self.zhuyin = on;
        self.forget_span_cache();
    }

    /// 目前是否處於注音模式。
    pub fn is_zhuyin_mode(&self) -> bool {
        self.zhuyin
    }

    /// 九宫格方案开关（手机 9 键）。与双拼、注音一样是输入方案，不是模式。
    pub fn set_nine_key(&mut self, on: bool) {
        if self.nine_key != on {
            self.chain.clear_code_words();
        }
        self.nine_key = on;
        self.forget_span_cache();
    }

    pub fn is_nine_key(&self) -> bool {
        self.nine_key
    }

    /// 设繁体输出：候选与上屏文本在出 Core 时换成繁体，词库与学习数据仍是简体（见 [`crate::traditional`]）。
    /// 变体没变就不重建，建一次要把 OpenCC 的字典装起来。
    pub fn set_traditional(&mut self, variant: TraditionalVariant) {
        if variant != self.traditional.variant() {
            self.traditional = Traditional::new(variant);
        }
    }

    /// 当前的繁体输出变体。
    pub fn traditional(&self) -> TraditionalVariant {
        self.traditional.variant()
    }

    /// 换形码方案，`None` 回到拼音的诸方案。编码与拼音是两套键，纠错缓存一并清掉。
    pub fn set_code_scheme(&mut self, scheme: Option<CodeScheme>) {
        self.chain.reset();
        self.code = scheme;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    /// 只改当前形码方案的按键规则，不换码表；没装形码时什么也不做。
    pub fn set_code_rules(&mut self, auto_commit: bool, top: bool) {
        if let Some(code) = &mut self.code {
            code.auto_commit = auto_commit;
            code.top = top;
        }
    }

    /// 当前的形码码表；`None` 表示走拼音（全拼 / 双拼 / 注音）。
    pub fn code_table(&self) -> Option<&CodeTable> {
        self.code.as_ref().map(|scheme| &scheme.table)
    }

    /// 当前的形码方案。
    pub fn code_scheme(&self) -> Option<&CodeScheme> {
        self.code.as_ref()
    }

    /// 是否处在形码方案下。
    pub fn is_code_mode(&self) -> bool {
        self.code.is_some()
    }

    /// 拼音侧参不参与查询。形码开着时把它关掉就是「只用形码」（`[general] scheme = "none"`）；
    /// 两边都开是混输，见 [`Self::set_code_scheme`] 与 `query_mixed`。
    pub fn set_phonetic(&mut self, on: bool) {
        if self.phonetic != on {
            self.chain.reset();
        }
        self.phonetic = on;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    pub fn is_phonetic(&self) -> bool {
        self.phonetic
    }

    /// 判斷注音模式下目前是否還需要輸入聲調。
    /// 供殼（平台層）用來判斷空白鍵是應該進緩衝區作為聲調，還是直接用來選詞。
    pub fn zhuyin_needs_tone(&self) -> bool {
        if !self.zhuyin {
            return false;
        }
        let raw = self.composition.text();
        if raw.is_empty() {
            return false;
        }
        let decoded = crate::zhuyin::decode(raw);
        if let Some(last) = decoded.units().last() {
            !last.complete && last.pinyin != "'"
        } else {
            false
        }
    }

    /// 组句中敲 `;` 是否该进缓冲区：微软 / 搜狗双拼里它是 ing 的韵母键，只在末尾有落单的声母时收，
    /// 其他时候仍是标点。问字模式（`?x`）看的是前缀之后的部分。
    pub fn takes_semicolon(&self) -> bool {
        let body = self
            .modes()
            .question_body(self.composition.scope(), self.zhuyin);
        self.shuangpin
            .as_deref()
            .filter(|layout| layout.uses_semicolon())
            .is_some_and(|layout| layout.decode_fuzzy(body, &self.fuzzy).pending_initial())
    }

    /// 只用形码：码表挂着、拼音侧关着。
    pub(in crate::engine) fn code_only(&self) -> bool {
        self.code.is_some() && !self.phonetic
    }

    /// 混输：码表挂着、拼音侧也开着。
    pub(in crate::engine) fn mixed(&self) -> bool {
        self.code.is_some() && self.phonetic
    }

    /// 有效的模式键：只用形码时所有字母都是字根键，只剩 `?` 开头的问字；
    /// 双拼与混输下小写字母各有用处，换成大写字母。
    pub(in crate::engine) fn modes(&self) -> ModeKeys {
        if self.code_only() {
            self.modes.letterless()
        } else if self.shuangpin.is_some() || self.mixed() {
            self.modes.shifted()
        } else {
            self.modes
        }
    }

    /// 缓冲区为空时敲的大写字母该不该进表达式 / 问字模式：只在双拼或混输下、且是模式键的大写时。
    /// 壳只在中文模式、Caps 灭时问。
    pub fn takes_mode_letter(&self, c: char) -> bool {
        let modes = self.modes();
        (self.shuangpin.is_some() || self.mixed())
            && !self.zhuyin
            && (c == modes.expression || c == modes.question)
    }

    /// 缓冲区为空时敲 `?` 该不该进问字模式（配置 `[shortcut] question_mark`）：壳据此决定问号是入口还是标点。
    pub fn takes_question_mark(&self) -> bool {
        self.modes().question_mark
    }
}
