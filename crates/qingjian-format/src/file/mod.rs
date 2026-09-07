//! 读与写：[`Container`] 映射并校验一个 `.qj` 文件，[`Writer`] 攒分节写出去。

mod container;
mod writer;

pub use container::Container;
pub use writer::Writer;
