/// 日文译词的一段：一段汉字带它的假名，或一段原样的假名 / 符号。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuriganaSegment {
    /// 这一段的写法。
    pub text: String,

    /// 这一段是汉字时的平假名读音；假名段没有。
    pub reading: Option<String>,
}
