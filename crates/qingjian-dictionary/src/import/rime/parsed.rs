//! 单文件转换的公开返回类型，供用户导入和形码转换工具共用。

/// 解析结果：青简 TSV 文本与 YAML 头里的名字。
pub struct Parsed {
    pub tsv: String,

    pub name: Option<String>,
}
