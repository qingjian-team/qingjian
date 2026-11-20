//! Engine 的词库集合：主词库、附加词库、模型补充词与内置符号词。

use qingjian_dictionary::{Dictionary, WordList};

use crate::{ContactBook, CustomPhrase};

/// Engine 持有的全部词库。
///
/// 主词库由壳装配，附加词库随导入 / 开关整体换掉，模型补充词单独一格，内置符号词是静态表。
pub(crate) struct EngineLexicon {
    /// 英文词表，中英混输用；没有就不出英文候选。
    pub(crate) english: Option<WordList>,

    /// 用户定义的短语，按输入码精确匹配、按权重排在最前。
    pub(crate) custom_phrases: Vec<CustomPhrase>,

    /// 屏蔽词：候选文本里含有其中任何一个就整条丢掉。自定义短语不过这一关。
    pub(crate) blocked: Vec<String>,

    /// 通讯录（壳读系统通讯录交来，用户在设置里开了才有）：名字并进查词，见 `set_contacts`。
    pub(crate) contacts: Option<ContactBook>,

    /// 静态词库。
    pub(crate) dictionary: Dictionary,

    /// 附加词库（领域词库、用户导入的），与主词库一起查词、一起进整句词图；不参与语言模型（它们没有 bigram，
    /// 走词频兜底）。壳按用户目录 `dicts/` 与配置 `[dictionaries]` 装配。
    pub(crate) extra_dictionaries: Vec<Dictionary>,

    /// 从三元模型词表补出的词（补充词库），壳在接上三元模型时装配。单放一格：附加词库随导入 / 开关整个换掉，它不跟着换。
    pub(crate) model_words: Option<Dictionary>,

    /// 内置的符号、emoji 名字（尖括号、笑死），带上 emoji 表时启用：词库里没有它们，不收就打不出这个词、也就配不上符号。
    pub(crate) companion_words: Option<&'static Dictionary>,
}

impl EngineLexicon {
    /// 只装配主词库，其余留空；壳随后按配置补附加词库与模型词。
    pub(crate) fn new(dictionary: Dictionary) -> Self {
        Self {
            dictionary,
            english: None,
            custom_phrases: Vec::new(),
            blocked: Vec::new(),
            contacts: None,
            extra_dictionaries: Vec::new(),
            model_words: None,
            companion_words: None,
        }
    }
}
