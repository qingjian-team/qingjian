//! 搜狗细胞词库 `.scel` 的解析。
//!
//! 这是一个**没有官方文档**的二进制格式，结构来自社区逆向（`scel2mmseg` / `scel2txt` 等实现，互相印证一致）：
//!
//! ```text
//! 0x000  128 字节头
//! [4]    掩码：0x44 → 词条区在 0x2628；0x45 → 0x26c4
//! 0x130  标题 / 0x338 分类 / 0x540 说明 / 0xd40 例句（都是 UTF-16LE）
//! 0x1540 拼音表：u16 编号 + u16 字节长 + UTF-16LE 字符串，最后一项固定是 "zuo"
//! 词条区  一组：u16 词数 + u16 拼音字节数 + 若干 u16 音节编号
//!         随后每个词：u16 字节长 + UTF-16LE 词 + 12 字节（权重 / 索引一类，青简用不上）
//! ```
//!
//! 词长与拼音长度都是**字节数**（UTF-16LE，所以是字符数的两倍），这点踩过：按字符数读会立刻错位。
//!
//! 产出是青简词库的 TSV（`词<TAB>音节 音节<TAB>词频`），交给同一条 [`crate::import::import`] 落成 `.qj`。
//! 搜狗不带词频，这里统一给 `DEFAULT_COUNT`；读音来自它的拼音表，连写音节之间本来就有空格。

/// 搜狗词库不带词频，导入时统一给这个数：足够让它参与排序，又不会盖过自带词频的词库。
const DEFAULT_COUNT: u32 = 100;

/// `.scel` 的识别：扩展名，或开头 4 字节的魔数（`40 15 00 00`）。
pub fn is_scel(name: &str, bytes: &[u8]) -> bool {
    name.to_ascii_lowercase().ends_with(".scel") || bytes.starts_with(&[0x40, 0x15, 0x00, 0x00])
}

/// 把 `.scel` 的字节解析成 TSV 文本。解析不出任何词条时返回 `None`。
pub fn to_tsv(bytes: &[u8]) -> Option<String> {
    let offset = entry_offset(bytes)?;
    let pinyin = pinyin_table(bytes, offset)?;
    let mut out = String::with_capacity(bytes.len() / 4);
    let mut cursor = offset;
    while cursor + 4 <= bytes.len() {
        let word_count = read_u16(bytes, cursor)? as usize;
        let pinyin_bytes = read_u16(bytes, cursor + 2)? as usize;
        cursor += 4;
        let syllables = pinyin_bytes / 2;
        // 越界的音节数是脏数据，收尾；空组（词数 0 或拼音 0）是合法占位，跳过接着读
        if syllables > 64 {
            break;
        }
        if word_count == 0 || syllables == 0 {
            continue;
        }
        let mut reading = String::new();
        let mut broken = false;
        for index in 0..syllables {
            let Some(id) = read_u16(bytes, cursor + index * 2) else {
                broken = true;
                break;
            };
            let Some(syllable) = pinyin.get(&id) else {
                broken = true;
                break;
            };
            if index > 0 {
                reading.push(' ');
            }
            reading.push_str(syllable);
        }
        cursor += pinyin_bytes;
        if broken {
            break;
        }
        for _ in 0..word_count {
            let length = read_u16(bytes, cursor)? as usize;
            cursor += 2;
            if !length.is_multiple_of(2) || cursor + length > bytes.len() {
                return finish(out);
            }
            let word = decode_utf16(bytes.get(cursor..cursor + length)?);
            cursor += length + 12;
            if word.is_empty() || word.chars().any(|c| c.is_control()) {
                continue;
            }
            out.push_str(&word);
            out.push('\t');
            out.push_str(&reading);
            out.push('\t');
            out.push_str(&DEFAULT_COUNT.to_string());
            out.push('\n');
        }
    }
    finish(out)
}

/// 一个词都没解析出来时算失败。
fn finish(out: String) -> Option<String> {
    (!out.is_empty()).then_some(out)
}

/// 词条区起点：认掩码 `0x44` / `0x45`。
fn entry_offset(bytes: &[u8]) -> Option<usize> {
    match *bytes.get(4)? {
        0x44 => Some(0x2628),
        0x45 => Some(0x26c4),
        _ => None,
    }
}

/// 拼音表：`编号 → 音节`。
///
/// 常见版本按字母序排、以 `zuo` 收尾，但**不能拿 `zuo` 当终止条件**：实测《网络流行新词》里
/// 词条引用了 `zuo` 之后的编号（缺表就走不下去了）。所以一直读到词条区起点，长度字段坏了就跳过
/// 那几字节接着扫 —— 多读进来的无用映射不影响结果，少读会让后面的词整片丢掉。
fn pinyin_table(bytes: &[u8], end: usize) -> Option<std::collections::HashMap<u16, String>> {
    let mut table = std::collections::HashMap::new();
    let mut cursor = 0x1540 + 4;
    let limit = end.min(bytes.len());
    while cursor + 4 <= limit {
        let Some(code) = read_u16(bytes, cursor) else {
            break;
        };
        let Some(length) = read_u16(bytes, cursor + 2).map(usize::from) else {
            break;
        };
        cursor += 4;
        if !length.is_multiple_of(2) || cursor + length > limit {
            // 错位：跳过这一项的四字节头部，接着找下一项
            continue;
        }
        let text = decode_utf16(&bytes[cursor..cursor + length]);
        cursor += length;
        if !text.is_empty() {
            table.insert(code, text);
        }
    }
    (!table.is_empty()).then_some(table)
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let pair = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([pair[0], pair[1]]))
}

/// 小端 UTF-16 解码；落单的半个代理项丢掉。
fn decode_utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let mut out = String::with_capacity(units.len());
    for item in char::decode_utf16(units) {
        match item {
            Ok(c) => out.push(c),
            Err(_) => continue,
        }
    }
    out.trim_end_matches('\0').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 拼一个最小的 `.scel`：掩码 0x44、两项拼音、一组词。
    /// 拼音表固定在 `0x1544`，词条区从 `0x2628` 起 —— 与真实文件的布局一致。
    fn sample() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x2628];
        bytes[4] = 0x44;
        let mut cursor = 0x1544;
        for (code, text) in [(1u16, "ni"), (2, "hao"), (3, "zuo")] {
            let encoded: Vec<u8> = text
                .encode_utf16()
                .flat_map(|unit| unit.to_le_bytes())
                .collect();
            bytes[cursor..cursor + 2].copy_from_slice(&code.to_le_bytes());
            bytes[cursor + 2..cursor + 4].copy_from_slice(&(encoded.len() as u16).to_le_bytes());
            bytes[cursor + 4..cursor + 4 + encoded.len()].copy_from_slice(&encoded);
            cursor += 4 + encoded.len();
        }
        // 词条区
        bytes.extend_from_slice(&2u16.to_le_bytes()); // 两个词共用这个读音
        bytes.extend_from_slice(&4u16.to_le_bytes()); // 拼音 4 字节 = 2 个音节
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        for word in ["你好", "拟好"] {
            let encoded: Vec<u8> = word
                .encode_utf16()
                .flat_map(|unit| unit.to_le_bytes())
                .collect();
            bytes.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
            bytes.extend_from_slice(&encoded);
            bytes.extend_from_slice(&[0u8; 12]);
        }
        bytes
    }

    #[test]
    fn parses_two_words_with_shared_reading() {
        let tsv = to_tsv(&sample()).expect("应当解析出词");
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "你好\tni hao\t100");
        assert_eq!(lines[1], "拟好\tni hao\t100");
    }

    #[test]
    fn recognises_by_magic_and_extension() {
        assert!(is_scel("x.scel", &[]));
        assert!(is_scel("x.bin", &[0x40, 0x15, 0x00, 0x00]));
        assert!(!is_scel("x.tsv", b"word\tpin yin"));
    }

    #[test]
    fn rejects_unknown_mask_and_truncated_files() {
        let mut bytes = sample();
        bytes[4] = 0x99;
        assert!(to_tsv(&bytes).is_none());
        let truncated = &sample()[..0x2630];
        assert!(to_tsv(truncated).is_none());
    }
}
