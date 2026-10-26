//! 已冻结拼音与来源的字词样本；答案只用于结果归档，不参与候选构造。

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Entry {
    pub id: String,

    pub text: String,

    pub keys: String,

    pub source: String,

    pub category: String,
}
