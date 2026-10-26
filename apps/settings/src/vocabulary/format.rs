//! 导出的格式。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 表格软件打开。
    Csv,
    /// Anki 的「导入文件」：正面译词，背面中文与例句。
    Anki,
    /// 一个离线网页，浏览器打开、打印。
    Html,
}

impl Format {
    pub const ALL: [Self; 3] = [Self::Csv, Self::Anki, Self::Html];

    pub fn label(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Anki => "Anki",
            Self::Html => "网页",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Anki => "txt",
            Self::Html => "html",
        }
    }
}
