//! 文件里的固定布局：文件头、分节表条目、数据种类编号、元数据分节的内容。

mod header;
mod kind;
mod metadata;
mod section_entry;

pub use header::{FORMAT_VERSION, Header, MAGIC};
pub use kind::Kind;
pub use metadata::Metadata;
pub use section_entry::SectionEntry;
