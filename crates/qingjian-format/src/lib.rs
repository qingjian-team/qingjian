//! `.qj` 数据容器：词库、语言模型这些常驻数据的二进制文件格式，打开就是 mmap，不反序列化。
//!
//! 文件 = [`Header`]（魔数、格式版本、数据种类、分节数）+ 分节表（4 字节标签 + 偏移 + 长度）+ 各分节正文，
//! 分节正文按 8 字节对齐。第一节固定是 `META`：TOML 文本的 [`Metadata`]（名称、许可证、署名、来源、条数），
//! 其余分节由各数据种类自己定义（词库的 arena 与索引、语言模型的 CSR 数组），内存里是什么布局文件里就是什么布局，
//! 所以加载只是映射文件再校验一遍头与分节边界。
//!
//! 数值一律小端、原生对齐；本 crate 在大端机器上拒绝编译。字符串分节打开时校验一次 UTF-8。
//! 数据文件只整体替换（写临时文件再改名），从不就地修改。

#[cfg(target_endian = "big")]
compile_error!("qingjian-format 的 .qj 文件是小端布局零拷贝读取，不支持大端机器");

mod error;
mod file;
pub mod hash;
mod layout;
mod view;

pub use error::FormatError;
pub use file::{Container, Writer};
pub use layout::{FORMAT_VERSION, Header, Kind, MAGIC, Metadata, SectionEntry};
pub use view::{Table, Text};

/// 元数据分节的标签，每个文件的第一节。
pub const META_TAG: [u8; 4] = *b"META";

/// 分节正文的对齐：足够让 `u64` / `f64` 字段的结构体零拷贝读取。
pub const SECTION_ALIGN: usize = 8;

/// 文件扩展名。
pub const EXTENSION: &str = "qj";

/// 分节标签的可读形式（日志与错误用）。
pub fn tag_str(tag: &[u8; 4]) -> String {
    tag.iter()
        .map(|&b| if b.is_ascii_graphic() { b as char } else { '?' })
        .collect()
}
