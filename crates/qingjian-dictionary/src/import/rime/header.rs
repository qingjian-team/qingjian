//! Rime 词库头里用于导入的名称、版本、列定义与分表引用。

#[derive(Debug)]
pub(in crate::import) struct Header {
    pub name: Option<String>,

    pub version: Option<String>,

    pub license: Option<String>,

    pub imports: Vec<String>,

    pub columns: Vec<String>,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            license: None,
            imports: Vec::new(),
            columns: ["text", "code", "weight"].map(str::to_owned).to_vec(),
        }
    }
}

impl Header {
    /// 返回头字段与正文起点；保留旧解析器接受没有 `---` 的词库的行为。
    pub fn parse(text: &str) -> (Self, usize) {
        let mut header = Self::default();
        let mut in_header = false;
        let mut list = None;
        for (index, raw) in text.lines().enumerate() {
            let line = uncomment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if line == "---" {
                in_header = true;
                continue;
            }
            if line == "..." {
                return (header, index + 1);
            }
            if !in_header && raw.contains('\t') {
                return (header, index);
            }
            if let Some(item) = line.strip_prefix("- ") {
                match list {
                    Some("columns") => header.columns.push(unquote(item)),
                    Some("import_tables") => header.imports.push(unquote(item)),
                    _ => {}
                }
                continue;
            }
            list = None;
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "name" => header.name = Some(unquote(value)),
                "version" => header.version = Some(unquote(value)),
                "license" => header.license = Some(unquote(value)),
                "columns" | "import_tables" => {
                    let items = if let Some(body) =
                        value.strip_prefix('[').and_then(|v| v.strip_suffix(']'))
                    {
                        body.split(',')
                            .map(unquote)
                            .filter(|v| !v.is_empty())
                            .collect()
                    } else {
                        list = Some(if key.trim() == "columns" {
                            "columns"
                        } else {
                            "import_tables"
                        });
                        Vec::new()
                    };
                    if key.trim() == "columns" {
                        header.columns = items;
                    } else {
                        header.imports = items;
                    }
                }
                _ => {}
            }
        }
        (header, text.lines().count())
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).to_owned()
}

/// 行尾注释只在引号外且 `#` 前为空白时起效，避免截断词库名称。
fn uncomment(line: &str) -> &str {
    let mut quote = None;
    let mut previous = None;
    for (index, ch) in line.char_indices() {
        if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            }
        }
        if ch == '#' && quote.is_none() && previous.is_none_or(char::is_whitespace) {
            return &line[..index];
        }
        previous = Some(ch);
    }
    line
}
