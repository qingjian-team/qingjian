//! 整段翻译（生词本的时间线）的提示词与回复解析：一批上屏的中文，各翻成学习语言的自然说法。

use qingjian_core::{Encounter, Language};
use serde::{Deserialize, Serialize};

use super::prompt::is_kana;

/// 一段译文最长几个字符：再长多半是模型在解释。
const MAX_CHARS: usize = 200;

/// 学习语言的名字，写进提示词。
fn language_name(language: Language) -> &'static str {
    match language {
        Language::Japanese => "日文",
        Language::Spanish => "西班牙文",
        Language::Vietnamese => "越南文",
        Language::German => "德文",
        Language::English | Language::Chinese => "英文",
    }
}

pub fn system_prompt(language: Language) -> String {
    let name = language_name(language);
    let reading = if language == Language::Japanese {
        "每条另给 r：t 的完整读音，只用平假名，外来语用片假名。\n"
    } else {
        ""
    };
    format!(
        "你是翻译。下面是用户用输入法打出的一段段中文，供语言学习者对照复习。把每段翻成最自然、地道的{name}：\n\
         - 短语就译成短语，句子就译成句子，不要扩写，不要解释，不要加引号；\n\
         - 网络用语、口语照口语译；专名照惯用写法；\n\
         {reading}\
         输出严格的 JSON：{{\"items\":[{{\"w\":\"收购以后\",\"t\":\"after the acquisition\"}}]}}。\n\
         items 与输入一一对应、顺序一致，w 必须原样照抄输入的中文。"
    )
}

#[derive(Serialize)]
struct UserMessage<'a> {
    texts: &'a [String],
}

pub fn user_prompt(texts: &[String]) -> String {
    serde_json::to_string(&UserMessage { texts }).unwrap_or_default()
}

#[derive(Deserialize)]
struct RawReply {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Deserialize)]
struct RawItem {
    w: String,

    t: String,

    #[serde(default)]
    r: Option<String>,
}

/// 只收请求过的中文，每段一条；译文去掉首尾引号与空白，空的、过长的、跟原文一样的丢掉。解析不了返回空。
pub fn parse_reply(content: &str, language: Language, texts: &[String]) -> Vec<Encounter> {
    let content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```");
    let Ok(reply) = serde_json::from_str::<RawReply>(content.trim()) else {
        return Vec::new();
    };
    let mut out: Vec<Encounter> = Vec::with_capacity(texts.len());
    for item in reply.items {
        let text = item.w.trim();
        if !texts.iter().any(|t| t == text) || out.iter().any(|e| e.text == text) {
            continue;
        }
        let translation = item
            .t
            .trim()
            .trim_matches(|c| matches!(c, '"' | '“' | '”' | '「' | '」'))
            .trim();
        if translation.is_empty()
            || translation == text
            || translation.chars().count() > MAX_CHARS
            || translation.contains('\n')
        {
            continue;
        }
        let reading = (language == Language::Japanese)
            .then(|| item.r.as_deref().map(str::trim).filter(|r| is_kana(r)))
            .flatten()
            .map(str::to_owned);
        out.push(Encounter {
            language,
            text: text.to_owned(),
            translation: translation.to_owned(),
            reading,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_requested_texts_and_cleans_quotes() {
        let texts = vec!["收购以后".to_owned(), "目前功能".to_owned()];
        let reply = r#"```json
{"items":[{"w":"收购以后","t":"\"After the acquisition\""},{"w":"别的","t":"other"},{"w":"目前功能","t":"目前功能"}]}
```"#;
        let encounters = parse_reply(reply, Language::English, &texts);
        assert_eq!(encounters.len(), 1);
        assert_eq!(encounters[0].text, "收购以后");
        assert_eq!(encounters[0].translation, "After the acquisition");
        assert_eq!(encounters[0].reading, None);
    }

    #[test]
    fn japanese_keeps_a_kana_reading() {
        let texts = vec!["生词本".to_owned()];
        let reply = r#"{"items":[{"w":"生词本","t":"単語帳","r":"たんごちょう"}]}"#;
        let encounters = parse_reply(reply, Language::Japanese, &texts);
        assert_eq!(encounters[0].reading.as_deref(), Some("たんごちょう"));
        assert!(system_prompt(Language::Japanese).contains("平假名"));
    }
}
