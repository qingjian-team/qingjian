use super::TranslationDifficulty;
use crate::candidate::{Language, Translation};

/// 候选翻译 annotation 的提供方。实现必须是纯查表级别的开销，不能做网络请求。
pub trait Translator: Send {
    /// 当前学习语言。
    fn language(&self) -> Language;

    /// 查不到返回 `None`，候选框那一行留空。
    fn translate(&self, text: &str) -> Option<Translation>;

    /// 候选旁的学习译词。`seed` 只在本轮命中随机频率时给出，同一段输入、同一词保持不变。
    /// 只支持固定释义的译者沿用原结果；可选释义池的实现按难度筛选后再抽样。
    fn translate_for_learning(
        &self,
        text: &str,
        _difficulty: TranslationDifficulty,
        _seed: Option<u64>,
    ) -> Option<Translation> {
        self.translate(text)
    }

    /// 释义兜底从云端写好了一条释义：记进个人释义表，下次 [`Self::translate`] 查得到。只查随包表的实现留空即可。
    fn learn(&mut self, _word: &str, _translation: Translation) {}

    /// 把个人释义表落盘；失败只记日志。
    fn flush(&mut self) {}
}

/// 不做翻译。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTranslator;

impl Translator for NoTranslator {
    fn language(&self) -> Language {
        Language::Chinese
    }

    fn translate(&self, _text: &str) -> Option<Translation> {
        None
    }
}
