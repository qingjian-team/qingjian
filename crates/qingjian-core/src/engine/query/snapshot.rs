/// 上一次查询留给输入日志的摘要：上屏时才知道选了什么，查询时才知道看到了什么，两头在这里接上。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuerySnapshot {
    /// 查询时的作用域（原始键）。
    pub scope: String,

    /// 整段的切分（`'` 连接），纠错生效时是纠正后的。
    pub pinyin: String,

    /// 拼写纠错是否生效。
    pub corrected: bool,

    /// 候选文本按顺序（只留前面一段，够定位选了第几个）。
    pub candidates: Vec<String>,

    /// 候选顺序经过了神经重排。
    pub rescored: bool,
}

impl QuerySnapshot {
    /// 最多留多少条候选文本来定位「选了第几个」。
    pub const MAX_CANDIDATES: usize = 32;
}
