/// 壳从应用读到的光标附近文本。应用不支持时整个为 `None`，Engine 退回本地输入历史。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurroundingText {
    /// 光标（或 marked text 起点）之前的文本，壳可以给得比观察窗口长，Engine 会裁。
    pub before: String,

    /// marked text 之后的文本。
    pub after: String,
}
