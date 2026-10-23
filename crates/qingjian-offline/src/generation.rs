//! 从模型配置读取特殊 token，不依赖某一对模型写死的词表大小。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Generation {
    pub decoder_start_token_id: i64,

    pub eos_token_id: i64,

    pub pad_token_id: i64,

    pub vocab_size: usize,
}
