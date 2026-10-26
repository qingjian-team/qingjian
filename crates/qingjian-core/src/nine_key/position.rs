//! 九宫格形状里的一个位置。

use qingjian_dictionary::SyllablePattern;

/// 形状里的一个位置：输入的一段，以及这段码下的全部写法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// 这段在输入里的起点（字节）。
    pub start: usize,

    /// 这段的长度；每种写法都与它等长。
    pub len: usize,

    /// 同码的写法，至少一种；已选定字母的位置只有一种。
    pub alternatives: Vec<String>,

    /// 写法是完整音节（否则是简拼声母或末尾没打完的前缀）。
    pub complete: bool,

    /// 按前缀查：简拼、或末尾的位置（打完的音节也可能还没打完，`xia` 是 `xiang` 的前缀）。
    pub prefix: bool,
}

impl Position {
    /// 给词库多写法查询与整句词图用的模式。
    pub fn patterns(&self) -> Vec<SyllablePattern<'_>> {
        self.alternatives
            .iter()
            .map(|text| SyllablePattern {
                text,
                complete: !self.prefix,
            })
            .collect()
    }
}
