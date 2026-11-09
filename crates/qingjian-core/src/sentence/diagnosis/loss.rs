use super::Parts;

/// 原句没成为整句首选的原因，按候选生成的先后排：越靠前的越早丢。
#[derive(Debug, Clone, PartialEq)]
pub enum Loss {
    /// 词库里没有能拼出原句的词：原句前 `covered` 个字拼得出，之后断了。
    Unreachable { covered: usize },

    /// 换一种拼音切分就拼得出（`syllables` 是那种切分），但整句用的不是它。
    Segmentation { syllables: Vec<String> },

    /// 词都在词库里，但有一个在格子里排第 `rank`（从 1 起）名、超过每格只留的 `limit` 个，被剪掉了。
    Pruned {
        word: String,
        rank: usize,
        limit: usize,
    },

    /// 原句的路径在词图里，分数也比选出来的高，是束搜索中途把它挤掉了。
    Search { gold: Parts, best: Parts },

    /// 原句的路径在词图里，但分数比选出来的低：语言模型、出字分或词频判错了。
    Scoring { gold: Parts, best: Parts },

    /// 词图最优路径就是原句，候选列表组装时被别的候选排到了前面。
    Assembly,
}

impl Loss {
    /// 统计用的短名。
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unreachable { .. } => "unreachable",
            Self::Segmentation { .. } => "segmentation",
            Self::Pruned { .. } => "pruned",
            Self::Search { .. } => "search",
            Self::Scoring { .. } => "scoring",
            Self::Assembly => "assembly",
        }
    }
}
