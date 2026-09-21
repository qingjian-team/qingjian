//! 注音解码的音节或显式分隔单元。
/// 解碼的單一單元，對應一個注音音節或分隔符
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// 轉換為拼音後的字串
    pub pinyin: String,

    /// 顯示用的注音符號字串（包含聲調）
    pub display: String,

    /// 原始按鍵字串
    pub keys: String,

    /// 是否為完整音節（有聲調、或可以作為結尾）
    pub complete: bool,
}
