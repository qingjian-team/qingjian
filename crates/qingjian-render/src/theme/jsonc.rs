//! JSONC：主题文件可以写 `//` 与 `/* */` 注释、对象与数组末尾多一个逗号（同 VS Code 的 `settings.json`）。
//! 解析前把注释与多余的逗号换成空格（换行照留），再交给 serde_json；位置不变，报错的行号、列号仍对得上原文件。

/// 去掉注释与尾逗号。字符串里的 `//`、`,` 原样保留；没收尾的块注释吞到文件末尾，交给 JSON 解析报错。
pub(super) fn strip(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = skip_string(bytes, i),
            b'/' if matches!(bytes.get(i + 1), Some(b'/' | b'*')) => {
                let end = comment_end(bytes, i);
                blank(&mut out[i..end]);
                i = end;
            }
            b',' => {
                if matches!(next_significant(bytes, i + 1), Some(b'}' | b']')) {
                    out[i] = b' ';
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    // 换掉的是整个注释（字符边界上起止）或单个逗号，结果仍是合法 UTF-8
    String::from_utf8(out).unwrap_or_else(|_| source.to_owned())
}

/// `start` 是开引号，返回收引号之后的位置。
fn skip_string(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `start` 是注释的 `/`，返回注释之后的位置（行注释停在换行前）。
fn comment_end(bytes: &[u8], start: usize) -> usize {
    if bytes[start + 1] == b'/' {
        bytes[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |n| start + n)
    } else {
        bytes[start + 2..]
            .windows(2)
            .position(|pair| pair == b"*/")
            .map_or(bytes.len(), |n| start + 2 + n + 2)
    }
}

/// 从 `i` 起跳过空白与注释后的第一个字节。
fn next_significant(bytes: &[u8], mut i: usize) -> Option<u8> {
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if matches!(bytes.get(i + 1), Some(b'/' | b'*')) => i = comment_end(bytes, i),
            other => return Some(other),
        }
    }
    None
}

/// 换成空格，换行照留；多字节字符的每个字节都换，结果仍是合法 UTF-8，字节位置不变。
fn blank(bytes: &mut [u8]) {
    for byte in bytes {
        if *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
}

#[cfg(test)]
mod tests {
    use super::strip;

    fn parse(source: &str) -> serde_json::Value {
        serde_json::from_str(&strip(source)).unwrap()
    }

    #[test]
    fn comments_and_trailing_commas_are_dropped() {
        let value = parse(
            r#"{
                // 行注释
                "a": [1, 2, /* 块注释 */ 3,],
                /* 跨行
                   注释，里面有 "引号" 和 , */
                "b": { "c": "d", // 逗号后面是注释
                },
            }"#,
        );
        assert_eq!(
            value,
            serde_json::json!({ "a": [1, 2, 3], "b": { "c": "d" } })
        );
    }

    #[test]
    fn strings_are_left_alone() {
        let value = parse(r#"{ "url": "https://qingjian.app/a,]", "q": "say \"//hi\",}" }"#);
        assert_eq!(value["url"], "https://qingjian.app/a,]");
        assert_eq!(value["q"], "say \"//hi\",}");
    }

    #[test]
    fn errors_keep_original_positions() {
        let source = "{\n  // 注释\n  \"a\": 1,\n  \"b\": oops\n}";
        let error = serde_json::from_str::<serde_json::Value>(&strip(source)).unwrap_err();
        assert_eq!(error.line(), 4);
    }

    #[test]
    fn comment_markers_inside_comments_and_non_ascii_survive() {
        let value = parse("{ \"名字\": \"樱花\" /* 中文注释 // 套着 */ }");
        assert_eq!(value["名字"], "樱花");
    }
}
