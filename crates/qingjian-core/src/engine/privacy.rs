//! 私密输入：密码框、浏览器无痕窗口这类应用声明「别记」的地方。壳判定（macOS 的 Secure Input 直接不组句；
//! Windows 的 `KEYBOARD_DISABLED` compartment 同样不组句，`IS_PRIVATE` / 密码类输入范围则照常组句但走这里），
//! Core 这一侧：不学习、不记输入日志、不发云端（联想 / 翻译 / 释义兜底）；排序仍用已有的个人数据。
//!
//! 私密输入有两路来源，各自一个开关，`private` 字段是两者的「或」：壳声明的 [`Engine::set_private`]，
//! 和连续命中 BIP-39 助记词表触发的启发式（[`Engine::note_english_commit`]，见 [`super::mnemonic`]）。
//! 分开存是因为壳的声明更权威——比如用户正在一个真正的密码框里，中途启发式那边因为敲的词不再连续命中
//! 而想撤，也绝不能把壳声明的私密状态一起撤掉；两路各自独立开关，只有都不要私密了才真正解除。

use super::learning::Learner;
use super::{BIP39_STREAK_THRESHOLD, Engine, mnemonic};

/// 私密状态是哪一路要求的。
enum PrivacySource {
    /// 壳声明的密码框 / 无痕窗口。
    System,
    /// 连续命中 BIP-39 助记词表触发的启发式。
    Heuristic,
}

impl Engine {
    /// 进入 / 离开私密输入。壳在焦点落到私密输入框（或离开它）时调；跨会话切换焦点时按各会话的状态重设。
    /// 此方法只设置写入开关，不清组句；已确认的隐私能力边界应先调用 [`Self::discard_input`]，
    /// 避免私密缓存被普通输入继续使用。恢复另一个独立会话时不需要丢弃该会话的输入。
    pub fn set_private(&mut self, private: bool) {
        self.set_private_from(PrivacySource::System, private);
    }

    pub fn is_private(&self) -> bool {
        self.private
    }

    /// 「学英文词」的单一收口：原样上屏的英文词（[`super::composing`]）、候选选中的英文词、句末英文词
    /// （都在 [`super::commit`]）都从这里进。命中 BIP-39 表就计一次连续命中；不管命中与否都照常喂给
    /// [`Self::learner`]——表内词要不要真的落盘，由 [`super::learning::MutedLearner::learn_english`]
    /// 内部再挡一次（那边同时也是 `system_private` / 学习开关生效的地方），这里只管「要不要转私密」。
    pub(in crate::engine) fn note_english_commit(&mut self, word: &str) {
        if mnemonic::is_bip39_word(word) {
            self.bip39_streak = self.bip39_streak.saturating_add(1);
            if self.bip39_streak >= BIP39_STREAK_THRESHOLD {
                self.set_private_from(PrivacySource::Heuristic, true);
            }
        } else {
            self.reset_bip39_streak();
        }
        self.learner.learn_english(word);
    }

    /// 上屏了跟「学英文词」无关的东西（中文词、形码、emoji、纯标点……）：连续命中数清零，
    /// 启发式私密也一并撤（如果是壳声明的私密，`system_private` 还在，`private` 不会被这句话动到）。
    pub(in crate::engine) fn reset_bip39_streak(&mut self) {
        self.bip39_streak = 0;
        self.set_private_from(PrivacySource::Heuristic, false);
    }

    fn set_private_from(&mut self, source: PrivacySource, active: bool) {
        match source {
            PrivacySource::System => self.system_private = active,
            PrivacySource::Heuristic => self.heuristic_private = active,
        }
        let combined = self.system_private || self.heuristic_private;
        if self.private == combined {
            return;
        }
        self.private = combined;
        self.learner.set_private(combined);
        self.logger.set_muted(combined);
        if combined {
            // 在飞的云结果不能再显示，前文也不能留
            self.prediction_sequence += 1;
            self.rescoring_before = None;
        }
    }
}
