//! 不查询候选、不提交、不记录的原样文本读取。
use crate::engine::{Engine, RawPreedit};
use crate::zhuyin;

impl Engine {
    /// 读取整段尚未上屏的原样文本，包括光标后的内容，不改变输入或学习状态。
    ///
    /// 普通方案保留实际键串、大小写及显式分隔符，不包含待处理辅码。
    /// 注音沿用原样提交的符号输出：单元内按键前缀产生的符号数，映射到完整
    /// 单元的同序字符边界；轻声重排仍用该逻辑位置，一声空格不移动显示光标。
    /// 未知键沿用解码器聚到末尾的行为，光标跟随其输出位置，可能不单调。
    /// 首位始终为 0，末位始终为完整文本末尾。
    pub fn raw_preedit(&self) -> RawPreedit {
        if self.is_zhuyin_mode() && !self.english_mode {
            let (decoded, cursor_bytes) =
                zhuyin::decode_at(self.composition.text(), self.composition.cursor());
            RawPreedit {
                text: decoded.marked(),
                cursor_bytes,
            }
        } else {
            let text = self.composition.typed_text();
            let mut cursor_bytes = self.composition.cursor().min(text.len());
            while !text.is_char_boundary(cursor_bytes) {
                cursor_bytes -= 1;
            }
            RawPreedit { text, cursor_bytes }
        }
    }
}
