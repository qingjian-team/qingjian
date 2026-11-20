//! 一段按键的读法解码：双拼 / 注音 / 九键，以及剩余拼音的显示形式。

use crate::engine::decoded::EngineDecoded;
use crate::engine::marked_rest;
use crate::engine::Engine;

impl Engine {
    /// 双拼开着时把一段键解成全拼；全拼下为 `None`，调用方原样用键。
    pub(in crate::engine) fn decode(&self, keys: &str) -> Option<EngineDecoded> {
        if self.zhuyin {
            Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(keys)))
        } else {
            self.shuangpin
                .as_deref()
                .map(|layout| EngineDecoded::Shuangpin(layout.decode_fuzzy(keys, &self.fuzzy)))
        }
    }

    /// 一段键的全部读法：纯双拼（或注音）一条，开了混输时还有全拼与两种打法交替的读法。
    /// 第一条是主读法，拼音行、尾巴与上屏消耗都按它算；其余读法只参与候选与整句比分数。
    pub(in crate::engine) fn decode_readings(&self, keys: &str) -> Vec<EngineDecoded> {
        match (
            self.zhuyin,
            self.shuangpin.as_deref(),
            self.shuangpin_full_pinyin,
        ) {
            (false, Some(layout), true) => layout
                .decode_mixed(keys, &self.fuzzy)
                .into_iter()
                .map(EngineDecoded::Shuangpin)
                .collect(),
            _ => self.decode(keys).into_iter().collect(),
        }
    }

    /// 光标后剩余拼音的显示形式：双拼先解码；能切就按音节用 `'` 连上，切不动就原样。
    /// 只用形码时剩余段是编码，原样显示。
    pub(in crate::engine) fn marked_rest(&self, rest: &str) -> String {
        if self.code_only() || (self.shuangpin.is_some() && self.shuangpin_raw_preedit) {
            return rest.to_owned();
        }
        if self.nine_key {
            return self.nine_key_rest(rest);
        }
        match self.decode(rest) {
            Some(decoded) => decoded.marked(),
            None => marked_rest(rest),
        }
    }
}
