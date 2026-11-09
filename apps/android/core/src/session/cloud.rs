//! 云端：快捷翻译（接上 / 断开、发请求、取译文，工具框开着时不计输入统计）与释义兜底。

use qingjian_core::{NoGlossFiller, NoPredictor};
use qingjian_predict::{CloudGlossFiller, CloudPredictor};

use super::Session;
use crate::cloud::CloudSettings;

impl Session {
    /// 接上或断开云端（快捷翻译用）：`None`、或配置起不来（没填密钥）都退回不联网。返回接没接上。
    /// 不记接口用量：用量文件靠文件锁合并，安卓上不支持（`lock() not supported`）。
    pub fn set_cloud(&mut self, settings: Option<&CloudSettings>) -> bool {
        let predictor = settings.map(|settings| CloudPredictor::new(&settings.predict_config()));
        match predictor {
            Some(Ok(predictor)) => {
                self.engine.set_predictor(Box::new(predictor));
                true
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "云端配置起不来，快捷翻译不可用");
                self.engine.set_predictor(Box::new(NoPredictor));
                false
            }
            None => {
                self.engine.set_predictor(Box::new(NoPredictor));
                false
            }
        }
    }

    /// 快捷翻译：汉字为主译成学习语言，外文译成中文（Core 的 `translation_target`）；只留最新一条。
    /// 没接云端或文字为空返回 `false`。
    pub fn translate(&mut self, text: &str) -> bool {
        self.engine.request_translation(text).is_some()
    }

    /// 取回译文：没到返回 `None`，模型没给出内容返回空串（壳停止等待）。
    pub fn poll_translation(&mut self) -> Option<String> {
        self.engine
            .poll_prediction()
            .map(|prediction| prediction.sentence.unwrap_or_default())
    }

    /// 上屏记不记进输入统计：快捷翻译、常用语编辑这些工具框开着时关掉（字不进应用），这时也不发云联想。
    pub fn set_metered(&mut self, metered: bool) {
        self.engine.set_metered(metered);
        self.tools_open = !metered;
    }

    /// 翻译框清空或关掉：作废正在等的那条。
    pub fn cancel_translation(&mut self) {
        self.engine.cancel_sentence_translation();
    }

    /// 释义兜底：开着时上屏了随包释义表查不到译词的词，后台问云端写进个人释义表，下次候选旁就有译词。
    /// 只发那个词本身；PC 的「生词本整句翻译」不跟着开。`None` 或起不来（没填密钥）都关掉，返回开没开上。
    pub fn set_gloss_fill(&mut self, settings: Option<&CloudSettings>) -> bool {
        let filler = settings.map(|settings| CloudGlossFiller::new(&settings.predict_config()));
        match filler {
            Some(Ok(filler)) => {
                self.engine.set_gloss_filler(Box::new(filler));
                true
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "云端配置起不来，释义兜底不可用");
                self.engine.set_gloss_filler(Box::new(NoGlossFiller));
                false
            }
            None => {
                self.engine.set_gloss_filler(Box::new(NoGlossFiller));
                false
            }
        }
    }

    /// 取回释义兜底写好的释义记进个人释义表；只碰通道，每次操作前顺手调。
    pub fn poll_glosses(&mut self) {
        self.engine.poll_glosses();
    }
}
