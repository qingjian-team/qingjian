/// 候选的音节逐个对到输入上的结果：上屏时按它消耗拼音、记个人敲错表。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Alignment {
    /// 消耗的输入字节数（含跳过的 `'`）。
    pub consumed: usize,

    /// 靠敲错变体对上的音节：(敲的那段字母, 候选的音节)。模糊音对上的不算。
    pub typos: Vec<(String, String)>,
}
