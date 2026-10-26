//! 键位提示的数据：双拼的键位、五笔的字根图。壳按它画键盘，什么时候弹出由壳按配置决定。

use super::Engine;
use crate::root_chart::RootChart;
use crate::shuangpin::KeyHint;

/// 86 五笔在 [`crate::CodeScheme::key`] 里的写法（沿用老日志的 `wubi`）。
const WUBI86_LOG_KEY: &str = "wubi";

impl Engine {
    /// 双拼的键位：开着双拼、缓冲区在解拼音时才有。光标停在中间时按光标前的那段算，与候选一致。
    pub fn key_hint(&self) -> Option<KeyHint> {
        let layout = self.shuangpin.as_deref()?;
        if !self.hint_applies() {
            return None;
        }
        Some(KeyHint::for_keys(layout, self.composition.scope()))
    }

    /// 五笔字根图：挂着 86 五笔（只用五笔或混输）时才有，别的形码方案没有字根图；混输时壳优先画它，双拼的键位不画。
    pub fn root_chart(&self) -> Option<&'static RootChart> {
        let wubi86 = self
            .code
            .as_ref()
            .is_some_and(|code| code.key == WUBI86_LOG_KEY);
        (wubi86 && self.hint_applies()).then(RootChart::wubi86)
    }

    /// 英文、直输、表达式、问字、辅码与符号面板都不画键盘。
    fn hint_applies(&self) -> bool {
        !(self.zhuyin
            || self.english_mode()
            || self.raw_mode()
            || self.expression_mode()
            || self.question_mode()
            || self.in_aux()
            || self.panel.is_some())
    }
}
