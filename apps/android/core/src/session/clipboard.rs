//! 剪贴板与常用语用到的会话能力：拆词、常用语重挂。

use std::path::Path;

use super::Session;

impl Session {
    /// 剪贴板「拆词」：按引擎的语言模型把一段文字切成可点选的小块（[`qingjian_core::sentence::text_pieces`]）。
    pub fn pieces(&self, text: &str) -> Vec<String> {
        qingjian_core::sentence::text_pieces(text, self.engine.language_model())
    }

    /// 常用语改过：重新挂带输入码的那些。
    pub fn reload_phrases(&mut self, user_dir: &Path) {
        crate::assembly::load_phrases(&mut self.engine, user_dir);
    }
}
