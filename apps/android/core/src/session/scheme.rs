//! 输入方案与语言的切换：九宫格、注音、双拼、五笔、繁体、日语、中英。

use std::str::FromStr;

use qingjian_core::{CodeScheme, RomajiComposer, ShuangpinScheme, TraditionalVariant};
use qingjian_dictionary::CodeTable;

use super::Session;
use crate::frame::Frame;

impl Session {
    /// 切换九宫格方案：组句中的输入先原样上屏（数字与字母的缓冲区互不通用）。
    pub fn set_nine_key(&mut self, on: bool) -> Frame {
        let raw = self.engine.take_raw();
        self.engine.set_nine_key(on);
        self.refresh(raw)
    }

    /// 切换注音（大千）方案：组句中的输入先原样上屏。
    pub fn set_zhuyin(&mut self, on: bool) -> Frame {
        let raw = self.engine.take_raw();
        self.engine.set_zhuyin_mode(on);
        self.refresh(raw)
    }

    /// 双拼方案（`scheme` 是 Core 的方案名，如 `xiaohe`；空串或认不出为全拼）与 preedit 写法
    /// （`raw_preedit` 显示敲的键，否则展开成全拼，同 PC 的 `shuangpin_raw_preedit`）：组句中的输入先原样上屏。
    pub fn set_shuangpin(&mut self, scheme: &str, raw_preedit: bool) -> Frame {
        let raw = self.engine.take_raw();
        self.engine
            .set_shuangpin(ShuangpinScheme::from_str(scheme).ok());
        self.engine.set_shuangpin_raw_preedit(raw_preedit);
        self.refresh(raw)
    }

    /// 五笔（86 版）开关，与 PC 的 `[general] code = "wubi86"` 同一套：`mixed` 时拼音也开着（混输，编码打全的五笔候选在前），
    /// 否则只用形码。组句中的输入先原样上屏。码表读不了就留在拼音，原因写日志。
    pub fn set_wubi(&mut self, on: bool, mixed: bool) -> Frame {
        let raw = self.engine.take_raw();
        let scheme = if on {
            CodeTable::from_path(&self.code_table)
                .inspect_err(|error| {
                    tracing::error!(%error, path = %self.code_table.display(), "五笔码表读取失败，仍用拼音");
                })
                .map(|table| CodeScheme::new(WUBI_LOG_KEY, table))
                .ok()
        } else {
            None
        };
        self.engine.set_phonetic(scheme.is_none() || mixed);
        self.engine.set_code_scheme(scheme);
        self.refresh(raw)
    }

    /// 繁体输出开关（功能面板「繁体」）：开着按键盘设置里选的字形（台湾 / 香港 / 通用，缺省台湾）出候选。
    pub fn set_traditional(&mut self, on: bool) -> Frame {
        let variant = if on {
            self.traditional_variant
        } else {
            TraditionalVariant::Off
        };
        self.engine.set_traditional(variant);
        self.refresh(String::new())
    }

    /// 切换日语罗马音：拼音组句中的字母先原样上屏，日语缓冲区里的先上屏平假名。
    pub fn set_japanese(&mut self, on: bool) -> Frame {
        let mut commit = self.engine.take_raw();
        if let Some(kana) = self.kana.take() {
            commit.push_str(&kana.hiragana());
        }
        self.kana = on.then(RomajiComposer::default);
        self.refresh(commit)
    }

    /// 中 / 英切换：组句中的字母先原样上屏。
    pub fn set_english_mode(&mut self, on: bool) -> Frame {
        let raw = self.engine.take_raw();
        self.engine.set_english_mode(on);
        self.refresh(raw)
    }
}

/// 输入日志里五笔的方案键，与 PC 的随包 86 五笔一致（回放靠它认出形码）。
const WUBI_LOG_KEY: &str = "wubi";
