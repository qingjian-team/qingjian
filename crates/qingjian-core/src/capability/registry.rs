//! 统一能力提供方：把 Engine 现有各 trait 对象注入点收敛到一张注册表。
//!
//! 注册表分两类存放：
//! - 已经是 `Arc<dyn …>` 且 `Send + Sync` 的现有能力（选词模型、短语模型、辅码码表）
//!   走类型化字段，取回时不用擦除；
//! - 其余具体实现走 [`Any`](std::any::Any) 擦除槽位，按 [`CapabilityKind`] 登记与取回。
//!
//! 没登记的能力一律视为缺省空操作：访问器返回 `None`，与 Engine 现有 `No*` 缺省实现
//! 的语义一致。注册表自身是 [`Send`] + [`Sync`]。

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::dictionary::AuxCodeLookup;
use crate::engine::ChoiceModel;
use crate::sentence::PhraseModel;

/// 一种可注入能力的稳定标识。枚举只用于归类与登记，不参与行为匹配。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CapabilityKind {
    /// 候选译文提供方（中 → 学习语言）。
    Translator,

    /// 英文候选释义提供方（英 → 中）。
    EnglishTranslator,

    /// 句首联想提供方。
    Predictor,

    /// 显式选区翻译的独立联想提供方。
    TranslationPredictor,

    /// 词级语言模型。
    LanguageModel,

    /// 整句同步 / 异步神经打分器。
    SentenceScorer,

    /// 用户词频学习方。
    Learner,

    /// 输入统计累计方。
    UsageMeter,

    /// 生词本记录方。
    VocabularyTracker,

    /// 释义兜底提供方。
    GlossFiller,

    /// 输入日志落盘方。
    InputLogger,

    /// 面板常用条目记录方。
    PanelHistory,

    /// 短语级语言模型。
    PhraseModel,

    /// 选词模型。
    ChoiceModel,

    /// 辅码码表。
    AuxCodeLookup,

    /// 语音识别方（壳直接持有，不经 Engine）。
    Transcriber,
}

/// 擦除槽位里存的共享句柄。
type Slot = Arc<dyn Any + Send + Sync>;

/// 可选能力注册表。
///
/// 空注册表等价于全部缺省空操作；构造 [`Self::noop`] 即「什么都没接」的初始状态。
#[derive(Default)]
pub struct Capabilities {
    /// 具体实现的擦除登记槽位。
    slots: HashMap<CapabilityKind, Slot>,

    /// 选词模型（现有 `Send + Sync` trait 对象，类型化存放）。
    choice_model: Option<Arc<dyn ChoiceModel>>,

    /// 短语级语言模型（现有 `Send + Sync` trait 对象，类型化存放）。
    phrase_model: Option<Arc<dyn PhraseModel>>,

    /// 辅码码表（现有 `Send + Sync` trait 对象集合，类型化存放）。
    aux_codes: Vec<Arc<dyn AuxCodeLookup>>,
}

impl fmt::Debug for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // trait 对象不要求 Debug，这里只列出已登记的能力种类，不打开句柄。
        f.debug_struct("Capabilities")
            .field("kinds", &self.kinds())
            .finish()
    }
}

impl Capabilities {
    /// 空注册表：所有能力都缺省（空操作）。
    pub fn noop() -> Self {
        Self::default()
    }

    /// 登记一条具体能力实现；同一种能力重复登记覆盖旧值。
    ///
    /// 注意：擦除槽位按具体类型取回，trait 对象请用对应的类型化访问器
    /// （[`Self::choice_model`]、[`Self::phrase_model`]、[`Self::aux_codes`]）。
    pub fn register<T: Any + Send + Sync>(&mut self, kind: CapabilityKind, value: T) {
        self.slots.insert(kind, Arc::new(value));
    }

    /// 登记一条已经共享的具体能力实现。
    pub fn register_shared<T: Any + Send + Sync>(&mut self, kind: CapabilityKind, value: Arc<T>) {
        self.slots.insert(kind, value);
    }

    /// 取回已登记的具体能力实现；没登记或类型不符返回 `None`。
    pub fn get<T: Any + Send + Sync>(&self, kind: CapabilityKind) -> Option<&T> {
        self.slots
            .get(&kind)
            .and_then(|slot| slot.downcast_ref::<T>())
    }

    /// 取回已登记能力的共享句柄；没登记或类型不符返回 `None`。
    pub fn get_shared<T: Any + Send + Sync>(&self, kind: CapabilityKind) -> Option<Arc<T>> {
        self.slots
            .get(&kind)
            .and_then(|slot| slot.clone().downcast::<T>().ok())
    }

    /// 是否登记了这种能力。
    pub fn contains(&self, kind: CapabilityKind) -> bool {
        match kind {
            CapabilityKind::ChoiceModel => self.choice_model.is_some(),
            CapabilityKind::PhraseModel => self.phrase_model.is_some(),
            CapabilityKind::AuxCodeLookup => !self.aux_codes.is_empty(),
            other => self.slots.contains_key(&other),
        }
    }

    /// 一条能力都没登记。
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
            && self.choice_model.is_none()
            && self.phrase_model.is_none()
            && self.aux_codes.is_empty()
    }

    /// 已登记能力的种类（顺序不保证，重复种类只列一次）。
    pub fn kinds(&self) -> Vec<CapabilityKind> {
        let mut kinds: Vec<_> = self.slots.keys().copied().collect();
        if self.choice_model.is_some() {
            kinds.push(CapabilityKind::ChoiceModel);
        }
        if self.phrase_model.is_some() {
            kinds.push(CapabilityKind::PhraseModel);
        }
        if !self.aux_codes.is_empty() {
            kinds.push(CapabilityKind::AuxCodeLookup);
        }
        kinds.sort();
        kinds.dedup();
        kinds
    }

    /// 接上选词模型（示例接线点：对应 `Engine::set_choice_model`）。
    pub fn with_choice_model(model: Arc<dyn ChoiceModel>) -> Self {
        let mut capabilities = Self::default();
        capabilities.set_choice_model(Some(model));
        capabilities
    }

    /// 接上或卸掉选词模型。
    pub fn set_choice_model(&mut self, model: Option<Arc<dyn ChoiceModel>>) {
        self.choice_model = model;
    }

    /// 取回已登记的选词模型；没接返回 `None`。
    pub fn choice_model(&self) -> Option<Arc<dyn ChoiceModel>> {
        self.choice_model.clone()
    }

    /// 接上短语级语言模型（示例接线点：对应 `Engine::set_phrase_model`）。
    pub fn with_phrase_model(model: Arc<dyn PhraseModel>) -> Self {
        let mut capabilities = Self::default();
        capabilities.set_phrase_model(Some(model));
        capabilities
    }

    /// 接上或卸掉短语级语言模型。
    pub fn set_phrase_model(&mut self, model: Option<Arc<dyn PhraseModel>>) {
        self.phrase_model = model;
    }

    /// 取回已登记的短语级语言模型；没接返回 `None`。
    pub fn phrase_model(&self) -> Option<Arc<dyn PhraseModel>> {
        self.phrase_model.clone()
    }

    /// 接上辅码码表（示例接线点：对应 `Engine::with_aux_codes`）。
    pub fn with_aux_codes(tables: Vec<Arc<dyn AuxCodeLookup>>) -> Self {
        let mut capabilities = Self::default();
        capabilities.set_aux_codes(tables);
        capabilities
    }

    /// 更换辅码码表。
    pub fn set_aux_codes(&mut self, tables: Vec<Arc<dyn AuxCodeLookup>>) {
        self.aux_codes = tables;
    }

    /// 取回已登记的辅码码表；没装返回空切片。
    pub fn aux_codes(&self) -> &[Arc<dyn AuxCodeLookup>] {
        &self.aux_codes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期断言：注册表必须能跨线程传递与共享。
    const _: fn() = || {
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<Capabilities>();
    };

    struct Echo;
    impl ChoiceModel for Echo {
        fn align(&self, _keys: &str, _syllables: &[String]) -> Option<Vec<u32>> {
            None
        }

        fn letter_units(&self, _keys: &str) -> Vec<u32> {
            Vec::new()
        }

        fn score(&self, _context: &str, _typed: &[u32], _texts: &[&str]) -> Vec<f32> {
            Vec::new()
        }
    }

    struct StubPhrase;
    impl PhraseModel for StubPhrase {
        fn score(&self, _context: &str, _texts: &[&str]) -> Vec<f64> {
            Vec::new()
        }
    }

    #[test]
    fn noop_registry_reports_no_capabilities() {
        let capabilities = Capabilities::noop();
        assert!(capabilities.is_empty());
        assert!(!capabilities.contains(CapabilityKind::ChoiceModel));
        assert!(!capabilities.contains(CapabilityKind::PhraseModel));
        assert!(!capabilities.contains(CapabilityKind::AuxCodeLookup));
        assert!(capabilities.kinds().is_empty());
        assert!(capabilities.choice_model().is_none());
        assert!(capabilities.phrase_model().is_none());
        assert!(capabilities.aux_codes().is_empty());
    }

    #[test]
    fn typed_adapters_register_and_fetch_existing_traits() {
        let mut capabilities = Capabilities::with_choice_model(Arc::new(Echo));
        capabilities.set_phrase_model(Some(Arc::new(StubPhrase)));

        assert!(capabilities.contains(CapabilityKind::ChoiceModel));
        assert!(capabilities.contains(CapabilityKind::PhraseModel));
        assert!(capabilities.choice_model().is_some());
        assert!(capabilities.phrase_model().is_some());

        let kinds = capabilities.kinds();
        assert!(kinds.contains(&CapabilityKind::ChoiceModel));
        assert!(kinds.contains(&CapabilityKind::PhraseModel));

        capabilities.choice_model().unwrap();
        capabilities.phrase_model().unwrap();
    }

    #[test]
    fn erased_slots_round_trip_concrete_values() {
        #[derive(Debug, PartialEq)]
        struct Setting(u32);

        let mut capabilities = Capabilities::noop();
        capabilities.register(CapabilityKind::Learner, Setting(7));

        assert!(capabilities.contains(CapabilityKind::Learner));
        assert_eq!(
            capabilities.get::<Setting>(CapabilityKind::Learner),
            Some(&Setting(7))
        );
        assert!(
            capabilities
                .get_shared::<Setting>(CapabilityKind::Learner)
                .is_some()
        );

        // 类型不符取回 `None`。
        assert!(
            capabilities
                .get::<String>(CapabilityKind::Learner)
                .is_none()
        );
    }

    #[test]
    fn replacing_a_typed_slot_keeps_latest_value() {
        let mut capabilities = Capabilities::noop();
        capabilities.set_choice_model(Some(Arc::new(Echo)));
        assert!(capabilities.choice_model().is_some());

        capabilities.set_choice_model(None);
        assert!(capabilities.choice_model().is_none());
        assert!(!capabilities.contains(CapabilityKind::ChoiceModel));
    }
}
