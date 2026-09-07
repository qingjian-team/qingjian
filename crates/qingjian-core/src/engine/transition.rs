use crate::sentence::Context;

/// 记进个人 n-gram 的一条转移：上屏时记下来，用户紧接着退格删掉重选时原样退回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// 前一个词之前的那个词；`None` 表示前一个词在句首。
    pub earlier: Option<String>,

    /// 前一个词；`None` 表示这个词在句首。
    pub previous: Option<String>,

    /// 上屏的词。
    pub word: String,

    /// 记了几份（用户点选的双份，整句路径顺带的一份）。
    pub times: u32,
}

impl Transition {
    pub fn new(context: Context<'_>, word: &str, times: u32) -> Self {
        Self {
            earlier: context.earlier.map(str::to_owned),
            previous: context.previous.map(str::to_owned),
            word: word.to_owned(),
            times,
        }
    }

    /// 记录时的上文。
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous.as_deref(),
            earlier: self.earlier.as_deref(),
        }
    }
}
