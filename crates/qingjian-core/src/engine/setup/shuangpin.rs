//! 双拼方案、键位与显示形式。

use crate::engine::Engine;
use crate::shuangpin::{Layout, Scheme};
use std::sync::Arc;

impl Engine {
    /// 设内置双拼方案，`None` 回到全拼。
    pub fn set_shuangpin(&mut self, scheme: Option<Scheme>) {
        self.set_shuangpin_layout(scheme.map(|scheme| Arc::new(scheme.layout().clone())));
    }

    /// 设双拼键位（内置或用户方案），`None` 回到全拼。纠错缓存按作用域记而作用域的含义变了，一并清掉。
    pub fn set_shuangpin_layout(&mut self, layout: Option<Arc<Layout>>) {
        if self.scheme.shuangpin != layout {
            self.chain.clear_code_words();
        }
        self.scheme.shuangpin = layout;
        *self.caches.correction.borrow_mut() = None;
    }

    pub fn shuangpin(&self) -> Option<&Layout> {
        self.scheme.shuangpin.as_deref()
    }
    /// 换双拼 preedit 显示形式：开着时显示原始按键（如 `ljse`），关着（缺省）展开成全拼（`lan'se`）。
    pub fn set_shuangpin_raw_preedit(&mut self, enabled: bool) {
        self.settings.shuangpin_raw_preedit = enabled;
    }

    pub fn shuangpin_raw_preedit(&self) -> bool {
        self.settings.shuangpin_raw_preedit
    }

    /// 换双拼候选窗口拼音行的显示形式：开着时显示原始按键，关着（缺省）展开成全拼。
    pub fn set_shuangpin_raw_window(&mut self, enabled: bool) {
        self.settings.shuangpin_raw_window = enabled;
    }

    pub fn shuangpin_raw_window(&self) -> bool {
        self.settings.shuangpin_raw_window
    }

    /// 双拼方案下是否同时接受全拼。开着时同一段输入里的音节可以两种打法混用（`wodemingzi`），
    /// 一边双拼一边全拼的两个人共用一份设置；关着（缺省）只有双拼一种读法。
    pub fn set_shuangpin_full_pinyin(&mut self, enabled: bool) {
        self.settings.shuangpin_full_pinyin = enabled;
        *self.caches.correction.borrow_mut() = None;
    }

    pub fn shuangpin_full_pinyin(&self) -> bool {
        self.settings.shuangpin_full_pinyin
    }
}
