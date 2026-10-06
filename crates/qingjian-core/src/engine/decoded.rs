//! 双拼、注音与全拼容错的解码结果，通过统一接口进入原生查询与上屏流程。

pub(super) enum EngineDecoded {
    Shuangpin(crate::shuangpin::Decoded),
    Zhuyin(crate::zhuyin::Decoded),
    Ice(super::ice::IceDecoded),
}

impl EngineDecoded {
    pub fn pinyin(&self) -> &str {
        match self {
            Self::Shuangpin(d) => d.pinyin(),
            Self::Zhuyin(d) => d.pinyin(),
            Self::Ice(d) => d.pinyin(),
        }
    }

    pub fn tail(&self) -> &str {
        match self {
            Self::Shuangpin(d) => d.tail(),
            Self::Zhuyin(d) => d.tail(),
            Self::Ice(d) => d.tail(),
        }
    }

    pub fn segmentation(&self) -> Option<crate::parser::Segmentation> {
        match self {
            Self::Shuangpin(d) => d.segmentation(),
            Self::Zhuyin(d) => d.segmentation(),
            Self::Ice(d) => d.segmentation(),
        }
    }

    pub fn marked(&self) -> String {
        match self {
            Self::Shuangpin(d) => d.marked(),
            Self::Zhuyin(d) => d.marked(),
            Self::Ice(d) => d.marked(),
        }
    }

    pub fn keys_for(&self, pinyin_len: usize) -> usize {
        match self {
            Self::Shuangpin(d) => d.keys_for(pinyin_len),
            Self::Zhuyin(d) => d.keys_for(pinyin_len),
            Self::Ice(d) => d.keys_for(pinyin_len),
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            Self::Shuangpin(d) => d.is_complete(),
            Self::Zhuyin(d) => d.is_complete(),
            Self::Ice(d) => d.is_complete(),
        }
    }
}
