//! Engine 的模型与提供方：翻译、联想、整句语言模型、重打分与选词模型。

use std::sync::Arc;

use crate::sentence::{LanguageModel, NoLanguageModel, PhraseModel, SentenceScorer};

use super::choice::{ChoiceRanker, ChoiceTier};
use super::prediction::{NoPredictor, Predictor};
use super::rescoring::{RescoreWorker, WakeSlot};
use super::translator::{NoTranslator, Translator};

/// Engine 注入的全部模型与提供方。
///
/// 缺省都是空实现 / `None`，壳按配置接上；换模型只动这一块。
pub(crate) struct EngineModels {
    /// 译文提供方，缺省为 `NoTranslator`。
    pub(crate) translator: Box<dyn Translator>,

    /// 英文候选的释义（英→中），缺省为 `NoTranslator`。英文候选的辅助语言是主语言中文，
    /// 与中文候选查学习语言的表分开，仍是「一个候选只显示一种辅助语言」。
    pub(crate) english_translator: Box<dyn Translator>,

    /// 联想提供方，缺省为 `NoPredictor`。
    pub(crate) predictor: Box<dyn Predictor>,

    /// 显式选区翻译的独立提供方；未设置时沿用联想服务。
    pub(crate) translation_predictor: Option<Box<dyn Predictor>>,

    /// 整句转换的语言模型，缺省为 `NoLanguageModel`（退化成一元词频）。
    pub(crate) language_model: Box<dyn LanguageModel>,

    /// 整句路径的同步神经重打分器（字级 Transformer，查询里当场打分；CLI 评测用）。
    pub(crate) sentence_scorer: Option<Box<dyn SentenceScorer>>,

    /// 异步重打分：后台线程里的打分器，壳在停顿后送任务、轮询结果（见 `rescoring`）。
    pub(crate) rescorer: Option<RescoreWorker>,

    /// 接的打分器是生成器（`SentenceScorer::generates`，含章·通变）：整句以生成为主、不重排词图路径。
    pub(crate) generative: bool,

    /// 后台模型交结果时叫醒壳（`Engine::set_rescoring_wake`），两个后台线程共用。
    pub(crate) wake: WakeSlot,

    /// 选词模型（试验）：候选窗前几条交给它重排一次，见 `choice`。
    pub(crate) choice: Option<ChoiceRanker>,

    /// 选词模型比几条、看多长前文。
    pub(crate) choice_tier: ChoiceTier,

    /// 短语级语言模型（知意的三元 n-gram）：选词重排里给整段候选打 ln P，没接时用二元模型逐词累加。
    pub(crate) phrase_model: Option<Arc<dyn PhraseModel>>,
}

impl Default for EngineModels {
    fn default() -> Self {
        Self {
            translator: Box::new(NoTranslator),
            english_translator: Box::new(NoTranslator),
            predictor: Box::new(NoPredictor),
            translation_predictor: None,
            language_model: Box::new(NoLanguageModel),
            sentence_scorer: None,
            rescorer: None,
            generative: false,
            wake: WakeSlot::default(),
            choice: None,
            choice_tier: ChoiceTier::default(),
            phrase_model: None,
        }
    }
}
