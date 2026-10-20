//! 独立翻译条的纯展示换行；保留英文词界并避免中文闭合标点落在行首。

/// 按近似半角列宽分行，超出可见行时加省略号；不改变翻译原文。
pub fn translation_lines(text: &str, columns: usize, max_lines: usize) -> Vec<String> {
    let columns = columns.max(2);
    let max_lines = max_lines.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut width = 0;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        let mut token = character.to_string();
        if word_character(character) {
            while characters.peek().is_some_and(|next| word_character(*next)) {
                token.push(characters.next().expect("已确认下个字符存在"));
            }
        }
        let token_width: usize = token.chars().map(char_width).sum();
        let break_before = character == '\n'
            || (!line.is_empty()
                && width + token_width > columns
                && !character.is_whitespace()
                && !closing_punctuation(character));
        if break_before {
            lines.push(std::mem::take(&mut line));
            width = 0;
            if lines.len() == max_lines {
                lines.last_mut().expect("至少一行").push('…');
                return lines;
            }
        }
        if character == '\n' {
            continue;
        }
        for character in token.chars() {
            let advance = char_width(character);
            // 超长标识符或 URL 才按字符拆分，普通英文单词整体移到下一行。
            if token_width > columns && !line.is_empty() && width + advance > columns {
                lines.push(std::mem::take(&mut line));
                width = 0;
                if lines.len() == max_lines {
                    lines.last_mut().expect("至少一行").push('…');
                    return lines;
                }
            }
            line.push(character);
            width += advance;
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn char_width(character: char) -> usize {
    if character.is_ascii() { 1 } else { 2 }
}

fn word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '\'' | '-' | '_')
}

fn closing_punctuation(character: char) -> bool {
    matches!(
        character,
        '，' | '。'
            | '！'
            | '？'
            | '；'
            | '：'
            | '、'
            | '”'
            | '’'
            | '）'
            | '】'
            | '》'
            | '」'
            | '』'
            | ','
            | '.'
            | '!'
            | '?'
            | ';'
            | ':'
            | ')'
            | ']'
            | '}'
    )
}

#[cfg(test)]
mod tests {
    use super::translation_lines;

    #[test]
    fn wraps_chinese_and_english_without_losing_text() {
        let text = "我正在学习英语。I am learning English.";
        let lines = translation_lines(text, 12, 10);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), text);
    }

    #[test]
    fn keeps_english_words_together_and_closing_punctuation_off_line_start() {
        let lines = translation_lines("An example floating translation strip.", 14, 10);
        assert!(lines.iter().any(|line| line.contains("example")));
        assert!(lines.iter().any(|line| line.contains("floating")));
        let lines = translation_lines("你好世界，再见世界。", 8, 10);
        assert!(lines.iter().all(|line| !line.starts_with(['，', '。'])));
        assert_eq!(lines.concat(), "你好世界，再见世界。");
    }

    #[test]
    fn bounds_long_output_and_preserves_explicit_newline() {
        assert_eq!(translation_lines("ab\ncd", 12, 10), ["ab", "cd"]);
        let lines = translation_lines(&"长".repeat(500), 48, 8);
        assert_eq!(lines.len(), 8);
        assert!(lines[7].ends_with('…'));
    }
}
