//! [`ClipFormat`]：系统剪贴板的一种原始格式。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipFormat {
    pub name: String,

    pub data: Vec<u8>,
}

impl ClipFormat {
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_owned(),
            data,
        }
    }
}
