//! 设置里影响候选的几项（含学习语言），与中文键盘的标点键。

use qingjian_core::{Language, NoTranslator, TraditionalVariant};

use super::Session;
use crate::assembly::load_translator;
use crate::frame::Frame;
use crate::options::InputOptions;

impl Session {
    /// 换模糊音、标点全半角、中英混输、学习语言、学习开关、繁体字形、附加词库、分次选词这几项；组句中的候选跟着重算。
    pub fn set_options(&mut self, options: &InputOptions) -> Frame {
        self.engine.set_fuzzy(options.fuzzy_rules());
        self.engine
            .set_full_width_punctuation(options.full_width_punctuation);
        self.engine
            .set_english_in_chinese(options.english_in_chinese);
        self.engine.set_chinese_first(options.chinese_first);
        self.set_learning_language(options.learning_language());
        self.engine.set_learning(options.learning);
        self.engine.set_hold_selections(options.hold_selections);
        self.traditional_variant = options.traditional_variant();
        if self.engine.traditional() != TraditionalVariant::Off {
            self.engine.set_traditional(self.traditional_variant);
        }
        self.sync_dictionaries(options);
        self.refresh(String::new())
    }

    /// 学习语言变了才换释义表（每次交设置都走这里）：关是不翻译；没有这门语言的表就保持原样。
    /// 换之前先落盘，旧语言的个人释义表与生词记录不丢。
    fn set_learning_language(&mut self, language: Option<Language>) {
        // 不翻译时 Core 报的学习语言是中文
        let current = Some(self.engine.learning_language()).filter(|l| *l != Language::Chinese);
        if current == language {
            return;
        }
        let translator = match language {
            Some(language) => match load_translator(&self.data_dir, &self.user_dir, language) {
                Some(translator) => translator,
                None => return,
            },
            None => Box::new(NoTranslator),
        };
        self.engine.flush_learning();
        self.engine.set_translator(translator);
        tracing::info!(
            language = language.map_or("off", Language::code),
            "学习语言已切换"
        );
    }

    /// 私密输入（应用声明不要学习的输入框，如浏览器的无痕标签页）：不学习、不记生词与统计、不发云端，同 PC。
    /// 壳每次换输入框时按输入框交一次。
    pub fn set_private(&mut self, private: bool) {
        self.engine.set_private(private);
    }

    /// 中文键盘上的标点键（交进来的是它对应的半角字符）：全角开着按 Core 转成中文标点（引号成对、数字后的点保持半角），
    /// 否则原样；返回要上屏的文字。组句中的输入壳已经先顶了首选。
    pub fn punctuate(&mut self, c: char) -> String {
        match self.engine.punctuate(c) {
            Some(text) => text.to_owned(),
            None => {
                self.engine.note_passthrough(c);
                c.to_string()
            }
        }
    }
}
