//! 辅码态：触发键的判定、码段的增删、码表查询。码段独立于拼音缓冲区（见 [`Engine`] 的 `aux_code` 字段）。

use super::{AuxSegment, Engine};
use crate::parser;

/// 触发键校验：单字符、ASCII 可打印、不是字母数字、不是 `'`（拼音隔音符）、不是当前翻页键。
/// 翻页键按当前配置动态排除（`[ ]`、`,.` 只有真配成 `[general] page_keys` 才拒）。设置页与配置加载共用。
pub fn is_valid_aux_code_key(key: char, page_keys: (char, char)) -> bool {
    key.is_ascii_graphic()
        && !key.is_ascii_alphanumeric()
        && key != '\''
        && key != page_keys.0
        && key != page_keys.1
}

impl Engine {
    /// 敲下 `key` 是不是要进辅码态。五条边界：
    ///
    /// - 辅码开着且有可用码表（`[aux_code] enabled` 缺省关；没装表或表全被关掉都不触发，`;` 保持原生行为）；
    /// - 配的触发键，且当前不在辅码态（已经在里面时再敲是幂等的，不算触发）；
    /// - 这个键没被键盘方案吃掉：微软 / 搜狗双拼里能当韵母的 `;` 优先当韵母（`x;` = xing）；
    /// - 光标在拼音段末尾（光标停在中间时只按光标前那段算候选，不是完整的输入）；
    /// - 作用域能完整切分成音节（简拼、残缺音节、英文直输段、表达式 / 问字模式都不触发）。
    pub fn aux_trigger(&self, key: char) -> bool {
        self.aux_enabled
            && !self.aux_codes.is_empty()
            && key == self.aux_code_key
            && self.aux_code.is_none()
            && !self.english_mode
            && !(key == ';' && self.takes_semicolon())
            && self.composition.cursor() >= self.composition.text().len()
            && self.aux_scope_ready()
    }

    /// 进辅码态：码段清空（候选先不过滤），preedit 多出触发键那一段。
    /// 触发键不进缓冲区，壳在 `;` 特判旁调这里。
    pub fn enter_aux(&mut self) {
        // 已经在辅码态里再调一次没有效果（边界 14 的幂等）：码段原样留着
        if self.aux_code.is_none() {
            self.aux_code = Some(String::new());
        }
    }

    /// 码段当前内容；不在辅码态时是空串。
    pub fn aux_code(&self) -> &str {
        self.aux_code.as_deref().unwrap_or_default()
    }

    /// 是否在辅码态。码段为空也算：刚敲下触发键、候选还没开始筛，或删空码段停在辅码态（开关开着）。
    pub fn in_aux(&self) -> bool {
        self.aux_code.is_some()
    }

    /// 当前的触发键。
    pub fn aux_code_key(&self) -> char {
        self.aux_code_key
    }

    /// 码段追加一个字母。非 `a-z` 返回 `false`，壳按原语义处理这个键（翻页键、标点）。
    pub fn push_aux_code(&mut self, c: char) -> bool {
        if !c.is_ascii_lowercase() {
            return false;
        }
        match &mut self.aux_code {
            Some(code) => {
                code.push(c);
                true
            }
            None => false,
        }
    }

    /// Esc：清码段回拼音态，拼音与候选保持。
    pub fn clear_aux(&mut self) {
        self.aux_code = None;
    }

    /// 正在筛的码段：辅码态且码段非空时才有。码段空（刚触发或删空停住）不过滤，候选不变。
    pub(super) fn aux_filter(&self) -> Option<&str> {
        self.aux_code.as_deref().filter(|code| !code.is_empty())
    }

    /// 码表插桩：`word` 有没有以 `prefix` 开头的码，有就返回命中的那条（多张表按注入顺序取第一条）。
    pub(super) fn matching_code<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str> {
        self.aux_codes
            .iter()
            .find_map(|table| table.code_with_prefix(word, prefix))
    }

    /// preedit 里的辅码两段；不在辅码态时为 `None`。
    pub(super) fn aux_segment(&self) -> Option<AuxSegment> {
        self.aux_code
            .as_ref()
            .map(|code| AuxSegment::new(self.aux_code_key, code.clone()))
    }

    /// 作用域能不能完整切分成音节：双拼 / 注音按解码结果，全拼按 [`parser::is_fully_segmentable`]。
    fn aux_scope_ready(&self) -> bool {
        let scope = self.composition.scope();
        if scope.is_empty() || self.expression_mode() || self.question_mode() || self.raw_mode() {
            return false;
        }
        match self.decode(scope) {
            Some(decoded) => decoded.is_complete(),
            None => parser::is_fully_segmentable(scope),
        }
    }
}
