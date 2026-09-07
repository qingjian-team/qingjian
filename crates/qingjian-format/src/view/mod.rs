//! 分节正文的两种视图：定长结构体数组 [`Table`] 与 UTF-8 文本 [`Text`]。
//! 自己解析出来的数据和映射文件里的数据都用它们装，查询代码只见 `&[T]` / `&str`。

mod table;
mod text;

pub use table::Table;
pub use text::Text;
