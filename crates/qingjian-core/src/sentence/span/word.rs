/// 词图一个格子里的一个词：从词库命中里拷出来、不再借用词库的形式，能放进 [`super::SpanCache`]。
#[derive(Debug, Clone, PartialEq)]
pub struct SpanWord {
    /// 词。
    pub text: String,

    /// 词的音节。
    pub syllables: Vec<String>,

    /// 词库静态词频。
    pub frequency: u32,

    /// 来自补充词库，兜底打分多扣一点（[`super::super::SECONDARY_PENALTY`]）。
    pub secondary: bool,

    /// 命中的音节不是敲的原样（模糊音 / 敲错变体）时的代价之和，进路径得分时扣掉；原样命中是 0。
    pub penalty: f64,

    /// 注音敲了声调时对不上的代价（[`super::super::TONE_PENALTY`] 乘声调代价），进路径得分时扣掉；与 `penalty` 分开记，
    /// 声调不对不算「不是敲的原样」。没有声调时是 0。
    pub tone: f64,

    /// 读音少见的代价（[`super::super::reading_cost`]），进路径得分时扣掉；不是多音字时是 0。
    pub reading: f64,
}
