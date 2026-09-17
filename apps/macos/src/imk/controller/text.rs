//! 可打印字符的处理：中英文模式、直输段、表达式与问字模式的分流。

use super::*;

impl QingjianInputController {
    pub(super) fn handle_text(&self, text: &str, client: TextClient<'_>) -> bool {
        tracing::debug!(%text, "inputText");
        self.note_application(&client);
        let mut composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let english = modifiers::caps_lock_on();
        // 终端、编辑器这类应用（`[apps] english_candidates_off`）里英文模式是纯直通
        let english_candidates = english
            && host::with(|h| h.english_candidates_in(client.bundle_identifier().as_deref()))
                .unwrap_or(false);
        // 英文模式组词中 Caps Lock 灭了（或开关关了）：敲的字母先原样上屏，别把它们当拼音
        if composing
            && !english_candidates
            && host::with(|h| h.engine.english_mode()).unwrap_or(false)
        {
            self.commit_raw(client);
            composing = false;
        }
        let [byte] = text.as_bytes() else {
            // 多字符文本（如输入法联动、粘贴）：先把当前候选（英文模式下是敲的字母）上屏，再交给应用
            if composing {
                if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                    self.commit_raw(client);
                } else {
                    self.commit_highlighted(client);
                }
            }
            return false;
        };
        let c = char::from(*byte);
        host::with(|h| h.indicator.update());
        // 缓冲区为空时敲 ? 先进问字模式（配置 `[shortcut] question_mark`，缺省关），中英文模式都行：
        // 后面跟字母就是在问字，跟别的键就还原成问号
        if !composing
            && c == QUESTION_PREFIX
            && host::with(|h| h.engine.takes_question_mark()).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 双拼下 Shift+V / Shift+U 进表达式 / 问字模式（全拼下的 v / u 被音节占了）
        if !composing && !english && host::with(|h| h.engine.takes_mode_letter(c)).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        let question = composing && host::with(|h| h.engine.question_mode()).unwrap_or(false);
        // 英文模式下问字：Caps Lock 让字母以大写送来，按小写收进问题
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        host::with(|h| h.engine.set_english_mode(english_candidates && !question));
        let (page_previous, page_next) =
            host::with(|h| h.page_keys).unwrap_or(qingjian_platform::DEFAULT_PAGE_KEYS);
        // Caps Lock 亮着 = 英文模式：不组句、不转标点，字母默认小写、按住 Shift 才大写
        if english && !question {
            // Caps Lock 亮着时 macOS 不管按没按 Shift 送来的都是大写，只能读 Shift 状态：按着才大写
            let letter = if modifiers::shift_down() {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            if !english_candidates {
                if composing {
                    self.commit_raw(client);
                }
                if c.is_ascii_alphabetic() {
                    client.insert_text(&letter.to_string());
                    host::with(|h| h.engine.note_passthrough(letter));
                    return true;
                }
                host::with(|h| h.engine.note_passthrough(c));
                return false;
            }
            // 英文候选：字母（以及组词中的 _ ' -）进缓冲区，候选来自英文词表。选词与中文模式一样：
            // 空格选高亮（词上屏后空格照样交给应用，接着打下一个词）、数字选当前页第 N 个、翻页键翻页；
            // 数字对应的格子没有候选（kubectl 这类词表没有的词、候选不足 N 个）时是标识符的一部分（foo1）。
            // 回车、标点先把敲的字母原样上屏再交给应用
            if composing
                && let Some(offset) = c.to_digit(10).filter(|d| *d > 0)
                && let Some(index) =
                    host::with(|h| h.session.index_on_page(offset as usize - 1)).flatten()
            {
                return self.commit_index(index, client);
            }
            if c.is_ascii_alphabetic()
                || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
            {
                host::with(|h| h.engine.push(letter));
                self.refresh(client);
                return true;
            }
            if composing && c == page_previous {
                return self.turn_page(-1, client);
            }
            if composing && c == page_next {
                return self.turn_page(1, client);
            }
            if composing {
                if c == ' ' {
                    self.commit_highlighted(client);
                } else {
                    self.commit_raw(client);
                }
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        // 表达式模式（v 开头）：数字与运算符进缓冲区，不当选词 / 翻页键
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        // 英文直输段（缓冲区里已有 `-` 这类字符）：可见字符一律追加，空格 / 回车整段原样上屏
        let raw = composing && host::with(|h| h.engine.raw_mode()).unwrap_or(false);
        // 组句中敲 `-`：进入英文直输段（`no-way`）；配成翻页键（`[general] page_keys` 选 `-=`）时才翻页
        let hyphen = composing && !question && c == '-' && c != page_previous && c != page_next;
        // 问字模式下敲的还可能是码点（`u4e00`、`u+1f600`）：数字与 `+` 进缓冲区而不是选词
        let unicode = question && host::with(|h| h.engine.unicode_entry()).unwrap_or(false);
        // 微软 / 搜狗双拼的 `;` 是 ing 键：末尾有落单声母时进缓冲区，其他时候还是标点
        let semicolon =
            composing && c == ';' && host::with(|h| h.engine.takes_semicolon()).unwrap_or(false);
        // 组句中敲半角标点：进缓冲区，整段成为英文直输段（`hello,` `dui'ma?`），中文模式下也能打带标点的英文；
        // 翻页键除外；⇧+数字（! @ # …）在前面已被删候选 / 译词键截走
        let punctuation = composing
            && !question
            && !expression
            && c.is_ascii_punctuation()
            && c != page_previous
            && c != page_next;
        if c.is_ascii_lowercase()
            || (composing && c == '\'')
            || semicolon
            || (expression && qingjian_core::shortcut::is_expression_char(c))
            || (raw && c.is_ascii_graphic())
            || (unicode && (c.is_ascii_digit() || c == '+'))
            || hyphen
            || punctuation
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 直输段里的空格：整段原样上屏，空格本身也交给应用（`hello, world` 里的空格要在）
        if raw && c == ' ' {
            self.commit_highlighted(client);
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing && self.restore_bare_question(client) {
            // 空格只是「把这个 ? 上屏」，不再多打一个空格；其他键按非组句状态继续处理
            if c == ' ' {
                return true;
            }
            return self.handle_text(text, client);
        }
        // 按住 Shift 打的大写字母：临时打英文，先把拼音原样上屏，再把字母交给应用
        if c.is_ascii_uppercase() {
            if composing {
                self.commit_raw(client);
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing {
            match c {
                ' ' => return self.commit_highlighted(client),
                '1'..='9' => {
                    let offset = usize::from(*byte - b'1');
                    if let Some(index) = host::with(|h| h.session.index_on_page(offset)).flatten() {
                        return self.commit_index(index, client);
                    }
                    // 这一页没有这一格（`gpt6` 只有三个候选）：数字当内容进缓冲区，成为英文直输段；
                    // 问字模式里数字不是问题的一部分，不算
                    if !question {
                        host::with(|h| h.engine.push(c));
                        self.refresh(client);
                    }
                    return true;
                }
                c if c == page_previous => return self.turn_page(-1, client),
                c if c == page_next => return self.turn_page(1, client),
                // 其他字符：把当前高亮候选上屏，再按非组句状态处理这个字符
                _ => {
                    self.commit_highlighted(client);
                }
            }
        }
        // 中文模式下的全角标点；转不了的（数字、字母以外的其他键）原样交给应用
        match host::with(|h| h.engine.punctuate(c)).flatten() {
            Some(full_width) => {
                client.insert_text(full_width);
                true
            }
            None => {
                host::with(|h| h.engine.note_passthrough(c));
                false
            }
        }
    }
}
