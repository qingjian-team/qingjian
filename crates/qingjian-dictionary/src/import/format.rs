//! 文本词库的第二列是拼音还是形码编码：导入只有一个入口，按内容分流。

/// 第二列看着是形码编码（五笔这类）：多字词的那一列大多不带空格。拼音的多字词音节之间总有空格，
/// 编码是连着写的一串字母。只有单字的文件分不出来，按拼音算。
pub fn is_code_table(tsv: &str) -> bool {
    let mut spaced = 0usize;
    let mut joined = 0usize;
    for line in tsv.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(word), Some(code)) = (fields.next(), fields.next()) else {
            continue;
        };
        if word.chars().count() < 2 {
            continue;
        }
        let code = code.trim();
        if code.contains(char::is_whitespace) {
            spaced += 1;
        } else if !code.is_empty() && code.chars().all(|c| c.is_ascii_alphabetic()) {
            joined += 1;
        }
    }
    joined > spaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_shape_codes_from_pinyin() {
        assert!(is_code_table(
            "# 形码码表\r\n拉混\truji\t1\r\n形表\tgage\t1\r\n青简\tgetu\t1\r\n"
        ));
        assert!(!is_code_table(
            "账套\tzhang tao\t500\n民法典\tmin fa dian\n"
        ));
        // 单字在两边都长得一样，看多字词
        assert!(is_code_table("工\ta\t1\n式\taa\t1\n开发\tgant\t9\n"));
        assert!(!is_code_table("的\tde\t1\n"));
    }
}
