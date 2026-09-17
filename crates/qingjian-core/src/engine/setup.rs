//! 注入与开关：词库、模糊音、双拼、翻译 / 学习 / 联想等 trait 实现的挂接，以及相应的只读访问。

use super::*;
use crate::engine::decoded::EngineDecoded;

impl Engine {
    /// 设置中文模式的标点转换。
    pub fn set_full_width_punctuation(&mut self, enabled: bool) {
        self.full_width_punctuation = enabled;
    }

    /// 原子更新自定义短语，非法规则保持旧值。
    pub fn set_custom_phrases(&mut self, phrases: Vec<crate::CustomPhrase>) -> Result<(), String> {
        crate::custom_phrase::validate_phrases(&phrases)?;
        self.custom_phrases = phrases;
        Ok(())
    }

    /// 设双拼方案，`None` 回到全拼。纠错缓存按作用域记而作用域的含义变了，一并清掉。
    pub fn set_shuangpin(&mut self, scheme: Option<Scheme>) {
        self.shuangpin = scheme;
        *self.correction_cache.borrow_mut() = None;
    }

    pub fn shuangpin(&self) -> Option<Scheme> {
        self.shuangpin
    }

    /// 設置是否啟用注音模式。開啟後鍵盤輸入按大千佈局解析。
    /// 学习开关（`[general] learning`）：关掉后不再记词频、用户词、个人 n-gram 与敲错表，已学的照常参与排序；
    /// 私密输入是另一个独立的开关（[`Self::set_private`]）。
    pub fn set_learning(&mut self, enabled: bool) {
        self.learner.set_disabled(!enabled);
    }

    pub fn set_zhuyin_mode(&mut self, on: bool) {
        self.zhuyin = on;
        self.forget_span_cache();
    }

    /// 設置是否啟用繁體輸出模式。
    pub fn set_traditional_mode(&mut self, on: bool) {
        self.traditional = on;
        if on && self.opencc.is_none() {
            match ferrous_opencc::OpenCC::from_config(ferrous_opencc::config::BuiltinConfig::S2tw) {
                Ok(opencc) => self.opencc = Some(opencc),
                Err(error) => tracing::warn!(%error, "繁体转换器初始化失败，候选仍是简体"),
            }
        }
    }

    /// 目前是否處於注音模式。
    pub fn is_zhuyin_mode(&self) -> bool {
        self.zhuyin
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
            .filter(|scheme| scheme.uses_semicolon())
            .is_some_and(|scheme| scheme.decode(body).pending_initial())
    }

    /// 有效的模式键：双拼下换成大写字母。
    pub(super) fn modes(&self) -> ModeKeys {
        if self.shuangpin.is_some() {
            self.modes.shifted()
        } else {
            self.modes
        }
    }

    /// 缓冲区为空时敲的大写字母该不该进表达式 / 问字模式：只在双拼下、且是模式键的大写时。
    /// 壳只在中文模式、Caps 灭时问。
    pub fn takes_mode_letter(&self, c: char) -> bool {
        let modes = self.modes();
        self.shuangpin.is_some() && !self.zhuyin && (c == modes.expression || c == modes.question)
    }

    /// 缓冲区为空时敲 `?` 该不该进问字模式（配置 `[shortcut] question_mark`）：壳据此决定问号是入口还是标点。
    pub fn takes_question_mark(&self) -> bool {
        self.modes().question_mark
    }

    /// 双拼开着时把一段键解成全拼；全拼下为 `None`，调用方原样用键。
    pub(super) fn decode(&self, keys: &str) -> Option<EngineDecoded> {
        if self.zhuyin {
            Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(keys)))
        } else {
            self.shuangpin
                .map(|scheme| EngineDecoded::Shuangpin(scheme.decode(keys)))
        }
    }

    /// 光标后剩余拼音的显示形式：双拼先解码；能切就按音节用 `'` 连上，切不动就原样。
    pub(super) fn marked_rest(&self, rest: &str) -> String {
        match self.decode(rest) {
            Some(decoded) => decoded.marked(),
            None => marked_rest(rest),
        }
    }

    pub fn with_emoji(mut self, table: EmojiTable) -> Self {
        self.emoji = Some(table);
        self
    }

    pub fn with_fuzzy(mut self, rules: FuzzyRules) -> Self {
        self.fuzzy = rules;
        self
    }

    /// 换模糊音规则：格子缓存里的代价随写法变，一起作废。
    pub fn set_fuzzy(&mut self, rules: FuzzyRules) {
        if self.fuzzy != rules {
            self.forget_span_cache();
        }
        self.fuzzy = rules;
    }

    pub fn fuzzy(&self) -> FuzzyRules {
        self.fuzzy
    }

    pub fn with_predictor(mut self, predictor: Box<dyn Predictor>) -> Self {
        self.predictor = predictor;
        self
    }

    /// 运行时换掉 Predictor（菜单开关云联想 / 配置热加载）；正在等的联想一并作废。
    pub fn set_predictor(&mut self, predictor: Box<dyn Predictor>) {
        self.cancel_prediction();
        self.predictor = predictor;
    }

    /// 挂上同步的整句重打分器（字级 Transformer，查询里当场打分，评测用）。`weight` 是神经分的权重 λ，
    /// `margin` 是参与重排的路径分门槛（nat），`context` 是给模型看的前文字符数；
    /// `None` 用缺省 [`NEURAL_WEIGHT`] / [`NEURAL_MARGIN`] / [`RESCORE_CONTEXT_CHARS`]。
    pub fn with_sentence_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) -> Self {
        self.sentence_scorer = Some(scorer);
        self.rescorer = None;
        self.set_neural_parameters(weight, margin, context);
        self
    }

    /// 挂上异步的整句重打分器：打分在后台线程，查询不等它，壳在停顿后 [`Self::request_rescoring`]、
    /// 结果到了 [`Self::poll_rescoring`] 后再查一次。参数同 [`Self::with_sentence_scorer`]。
    pub fn with_async_sentence_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) -> Self {
        self.set_async_sentence_scorer(Some(scorer));
        self.set_neural_parameters(weight, margin, context);
        self
    }

    /// 运行时换 / 卸异步重打分器（壳里模型在后台加载完才接上，配置关掉就卸）。
    pub fn set_async_sentence_scorer(&mut self, scorer: Option<Box<dyn SentenceScorer>>) {
        self.sentence_scorer = None;
        self.rescorer = scorer.map(super::rescoring::RescoreWorker::spawn);
        *self.neural_cache.borrow_mut() = super::rescoring::NeuralCache::default();
        self.forget_span_cache();
    }

    /// 换一组个人 n-gram 插值参数（回放调参用）；整句格子缓存作废。
    pub fn set_interpolation(&mut self, interpolation: Interpolation) {
        self.interpolation = interpolation;
        self.forget_span_cache();
    }

    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// 换一组敲错纠正代价（回放调参用）；整句格子缓存与纠错缓存作废。
    pub fn set_typo_costs(&mut self, costs: TypoCosts) {
        self.typo_costs = costs;
        *self.correction_cache.borrow_mut() = None;
        self.forget_span_cache();
    }

    pub fn typo_costs(&self) -> TypoCosts {
        self.typo_costs
    }

    /// 整句转换与词级排序用的个人部分：学习器的个人 n-gram 配上当前插值参数。
    pub(super) fn personal(&self) -> Personal<'_> {
        Personal {
            ngram: self.learner.user_ngram(),
            interpolation: self.interpolation,
        }
    }

    /// 神经分的权重 λ（0 到 1）。
    pub fn set_neural_weight(&mut self, weight: f64) {
        self.neural_weight = weight.clamp(0.0, 1.0);
        self.forget_span_cache();
    }

    fn set_neural_parameters(
        &mut self,
        weight: Option<f64>,
        margin: Option<f64>,
        context: Option<usize>,
    ) {
        self.neural_weight = weight.unwrap_or(NEURAL_WEIGHT).clamp(0.0, 1.0);
        self.neural_margin = margin.unwrap_or(NEURAL_MARGIN).max(0.0);
        self.neural_context = context.unwrap_or(RESCORE_CONTEXT_CHARS);
        self.forget_span_cache();
    }

    pub fn with_language_model(mut self, model: Box<dyn LanguageModel>) -> Self {
        self.language_model = model;
        self
    }

    /// 静态语言模型（没接就是 [`NoLanguageModel`]）：评测工具拿它按 [`crate::sentence::segment_text`] 切汉字文本。
    pub fn language_model(&self) -> &dyn LanguageModel {
        &*self.language_model
    }

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

    pub fn with_english(mut self, words: WordList) -> Self {
        self.english = Some(words);
        self
    }

    pub fn with_translator(mut self, translator: Box<dyn Translator>) -> Self {
        self.translator = translator;
        self
    }

    /// 运行时换学习语言的释义表。
    /// 接英文候选用的释义表（英→中）。
    pub fn with_english_translator(mut self, translator: Box<dyn Translator>) -> Self {
        self.english_translator = translator;
        self
    }

    pub fn set_translator(&mut self, translator: Box<dyn Translator>) {
        self.translator = translator;
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

    /// 中英混输里中文候选是否总排在英文词前面（配置 `[general] chinese_first`，缺省关）。
    /// 关着时拼音「不像话」的输入英文词排第一（`hello` 先英文再 荷兰咯）；开了英文词固定第二。
    pub fn set_chinese_first(&mut self, on: bool) {
        self.chinese_first = on;
    }

    pub fn chinese_first(&self) -> bool {
        self.chinese_first
    }

    pub fn with_learner(mut self, learner: Box<dyn Learner>) -> Self {
        self.learner.replace(learner);
        self.forget_span_cache();
        self
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

    pub fn with_usage_meter(mut self, meter: Box<dyn UsageMeter>) -> Self {
        self.meter = meter;
        self
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

    pub fn dictionary(&self) -> &Dictionary {
        &self.dictionary
    }

    /// 换掉全部附加词库（导入、移除、开关之后）。格子缓存随之作废。
    pub fn set_extra_dictionaries(&mut self, dictionaries: Vec<Dictionary>) {
        self.extra_dictionaries = dictionaries;
        self.forget_span_cache();
    }

    pub fn extra_dictionaries(&self) -> &[Dictionary] {
        &self.extra_dictionaries
    }

    /// 查词用的全部词库：主词库、附加词库、用户词。
    pub(super) fn all_dictionaries(&self) -> Vec<&Dictionary> {
        let mut all = Vec::with_capacity(self.extra_dictionaries.len() + 2);
        all.push(&self.dictionary);
        all.extend(self.extra_dictionaries.iter());
        if let Some(user) = self.learner.user_words() {
            all.push(user);
        }
        all
    }

    /// 全部词库的词频之和，词频归一化成概率时用。
    pub(super) fn total_frequency(&self) -> u64 {
        self.all_dictionaries()
            .iter()
            .map(|d| d.total_frequency())
            .sum()
    }

    pub fn learner(&self) -> &dyn Learner {
        self.learner.inner()
    }

    /// 拿到可变的 Learner 就当它要改：格子缓存一起作废。
    pub fn learner_mut(&mut self) -> &mut dyn Learner {
        self.forget_span_cache();
        self.learner.inner_mut()
    }

    pub fn learning_language(&self) -> Language {
        self.translator.language()
    }
}
