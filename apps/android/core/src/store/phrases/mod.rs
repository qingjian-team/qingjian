//! 常用语面板的读写：列表、分组、增删改、移到最前都走 `qingjian_learning::PhraseBook`。
//! 不要句柄；改了之后壳再调会话的 `reload_phrases`，带输入码的常用语才进候选。

mod page;
mod request;
mod rules;

pub use page::PhrasePage;
pub use request::PhraseRequest;
pub use rules::PhraseRules;

use std::path::Path;

use qingjian_learning::{DEFAULT_GROUP, MAX_CODE_CHARS, PhraseBook};

/// 壳的编辑框按这份规则收文字，与落盘时的裁剪一致。
pub fn phrase_rules() -> PhraseRules {
    PhraseRules {
        default_group: DEFAULT_GROUP,
        max_code: MAX_CODE_CHARS,
    }
}

/// 跑一个请求，返回操作之后的整页（分组与全部常用语，靠前的常用）。
pub fn phrases(user_dir: &Path, request: PhraseRequest) -> PhrasePage {
    let mut book = PhraseBook::open(user_dir);
    let result = match request {
        PhraseRequest::List => Ok(()),
        PhraseRequest::Add { text, group, code } => book.add(&text, &group, &code).map(drop),
        PhraseRequest::Update {
            index,
            text,
            group,
            code,
        } => book.update(index, &text, &group, &code).map(drop),
        PhraseRequest::Remove { index } => book.remove(index),
        PhraseRequest::Top { index } => book.move_to_top(index),
    };
    if let Err(error) = result {
        tracing::warn!(%error, "常用语写不进去");
    }
    PhrasePage {
        groups: book.groups(),
        phrases: book.phrases().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_then_move_to_top() {
        let dir = std::env::temp_dir().join(format!("qj-android-phrases-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let add = |text: &str| {
            serde_json::from_str(&format!(
                r#"{{"op":"add","text":"{text}","group":"","code":""}}"#
            ))
            .unwrap()
        };
        phrases(&dir, add("一"));
        phrases(&dir, add("二"));
        let page = phrases(&dir, PhraseRequest::Top { index: 1 });
        assert_eq!(page.phrases[0].text, "一");
        assert_eq!(page.groups, [phrase_rules().default_group]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn code_is_cut_to_the_shell_limit() {
        let dir = std::env::temp_dir().join(format!("qj-android-rules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let code = "a".repeat(phrase_rules().max_code + 10);
        let request = PhraseRequest::Add {
            text: "长".to_owned(),
            group: String::new(),
            code,
        };
        let page = phrases(&dir, request);
        assert_eq!(page.phrases[0].code.len(), phrase_rules().max_code);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
