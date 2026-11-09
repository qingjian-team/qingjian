/// 一条路径的得分拆开看：总分、其中语言模型转移的部分、其中知意出字的部分（其余是词频加分与读音 / 敲错代价）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Parts {
    pub score: f64,

    pub language_model: f64,

    pub emission: f64,
}
