//! 双拼 / 注音的解码结果：两种方案解出来的东西在引擎里走同一条路，这里把它们收成一个枚举。

pub(super) enum EngineDecoded {
    Shuangpin(crate::shuangpin::Decoded),
    Zhuyin(crate::zhuyin::Decoded),
}

impl EngineDecoded {
    pub fn pinyin(&self) -> &str {
        match self {
            Self::Shuangpin(d) => d.pinyin(),
            Self::Zhuyin(d) => d.pinyin(),
        }
    }

    pub fn tail(&self) -> &str {
        match self {
            Self::Shuangpin(d) => d.tail(),
            Self::Zhuyin(d) => d.tail(),
        }
    }

    pub fn segmentation(&self) -> Option<crate::parser::Segmentation> {
        match self {
            Self::Shuangpin(d) => d.segmentation(),
            Self::Zhuyin(d) => d.segmentation(),
        }
    }

    /// 每個音節敲的聲調，只有注音有；雙拼給空表（排序不看聲調）。
    pub fn tones(&self) -> Vec<Option<u8>> {
        match self {
            Self::Shuangpin(_) => Vec::new(),
            Self::Zhuyin(d) => d.tones(),
        }
    }

    pub fn marked(&self) -> String {
        match self {
            Self::Shuangpin(d) => d.marked(),
            Self::Zhuyin(d) => d.marked(),
        }
    }

    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        match self {
            Self::Shuangpin(d) => d.keys_for(pinyin_len),
            Self::Zhuyin(d) => d.keys_for(pinyin_len),
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            Self::Shuangpin(d) => d.is_complete(),
            Self::Zhuyin(d) => d.is_complete(),
        }
    }
}
