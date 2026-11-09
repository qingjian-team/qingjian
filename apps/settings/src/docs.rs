//! ⓘ 的文档链接：哪一页哪一行的 ⓘ 点了打开官网文档的哪一段。文档源是仓库的 `docs/user/`，官网按目录渲染，
//! 锚点照官网的 `headingId`（`web/src/lib/docs/render.ts`）生成；测试核对每一段在文档里真有这个标题，文档改标题时这里跟着改。

use crate::i18n::tr;
use crate::page_id::PageId;

const DOCS_URL: &str = "https://qingjian.app/docs";

/// （页、行标题的文案键（随包数据的行写名字本身）、文档（`docs/user/` 下的路径，不带 `.md`）、标题（空着是整页））。
const LINKS: &[(PageId, &str, &str, &str)] = &[
    (
        PageId::Input,
        "input.pinyin",
        "input/fuzzy-and-shuangpin",
        "选择方案",
    ),
    (
        PageId::Input,
        "input.raw_preedit",
        "input/fuzzy-and-shuangpin",
        "双拼",
    ),
    (
        PageId::Input,
        "input.raw_window",
        "input/fuzzy-and-shuangpin",
        "双拼",
    ),
    (PageId::Input, "input.code", "input/wubi", "开启"),
    (
        PageId::Input,
        "input.key_hint",
        "input/fuzzy-and-shuangpin",
        "键位提示",
    ),
    (PageId::Input, "input.aux_enabled", "input/aux-code", "开启"),
    (
        PageId::Input,
        "shortcut.aux_code_key",
        "input/aux-code",
        "触发键",
    ),
    (PageId::Input, "笔画", "input/aux-code", "码表"),
    (
        PageId::Input,
        "input.import_code_table",
        "input/aux-code",
        "码表",
    ),
    (
        PageId::Candidates,
        "candidates.translation",
        "learning/translation",
        "译词",
    ),
    (
        PageId::Candidates,
        "candidates.numpad_digit",
        "getting-started/keys",
        "打拼音时",
    ),
    (
        PageId::Candidates,
        "candidates.traditional",
        "input/traditional",
        "开启",
    ),
    (
        PageId::Candidates,
        "candidates.emoji",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "candidates.kaomoji",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "candidates.symbols",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "candidates.learning",
        "personalize/habits",
        "关闭学习与从头开始",
    ),
    (
        PageId::Cloud,
        "cloud.vocabulary_translation",
        "cloud/sentence-translation",
        "生词本整句翻译",
    ),
    (
        PageId::Vocabulary,
        "vocabulary.export",
        "learning/vocabulary",
        "两个视图共用",
    ),
    (
        PageId::Vocabulary,
        "vocabulary.clear_book",
        "learning/vocabulary",
        "两个视图共用",
    ),
    (
        PageId::Candidates,
        "candidates.local_model",
        "input/sentence",
        "本地整句模型",
    ),
    (
        PageId::Candidates,
        "candidates.english_in_chinese",
        "input/english-mode",
        "在中文模式里打英文",
    ),
    (
        PageId::Language,
        "language.english_mode",
        "input/english-mode",
        "进入英文模式",
    ),
    (
        PageId::Language,
        "language.english_candidates",
        "input/english-mode",
        "哪些应用里给候选",
    ),
    (
        PageId::Language,
        "language.shift_letter",
        "input/english-mode",
        "在中文模式里打英文",
    ),
    (
        PageId::Language,
        "language.switch_keys",
        "input/english-mode",
        "改掉或关掉中 / 英切换",
    ),
    (
        PageId::Language,
        "language.default_english",
        "input/english-mode",
        "默认模式与按应用记忆",
    ),
    (
        PageId::Language,
        "language.remember_per_app",
        "input/english-mode",
        "默认模式与按应用记忆",
    ),
    (
        PageId::Language,
        "language.auto_space",
        "input/english-mode",
        "中西文自动空格",
    ),
    (
        PageId::Language,
        "language.punct_commits",
        "input/punctuation",
        "打拼音时敲标点",
    ),
    (
        PageId::Appearance,
        "appearance.preedit",
        "personalize/appearance",
        "排布",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.page_keys",
        "personalize/keys-settings",
        "翻页键",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.extra_page_keys",
        "personalize/keys-settings",
        "翻页键",
    ),
    (
        PageId::Shortcuts,
        "shortcut.commit_candidate",
        "personalize/keys-settings",
        "上屏键（Windows、Linux）",
    ),
    (
        PageId::Shortcuts,
        "shortcut.commit_raw",
        "personalize/keys-settings",
        "上屏键（Windows、Linux）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.enter_translation",
        "personalize/keys-settings",
        "回车优先上屏译词（macOS）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.highlight_keys",
        "personalize/keys-settings",
        "高亮移动键（macOS、Windows）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.extra_candidate_navigation",
        "personalize/keys-settings",
        "Emacs 风格候选键（macOS）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.punctuation_width",
        "personalize/keys-settings",
        "全半角标点快捷键（macOS、Windows）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.traditional",
        "personalize/keys-settings",
        "简繁切换快捷键（Windows）",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.expression",
        "input/shortcuts",
        "换前缀字母",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.lookup",
        "cloud/question-and-translate",
        "问字",
    ),
    (
        PageId::Shortcuts,
        "shortcuts.question_lookup",
        "cloud/question-and-translate",
        "用问号进入问字",
    ),
    (
        PageId::Shortcuts,
        "shortcut.translation",
        "learning/translation",
        "直接上屏译词",
    ),
    (
        PageId::Shortcuts,
        "shortcut.translation_second",
        "learning/translation",
        "直接上屏译词",
    ),
    (
        PageId::Shortcuts,
        "shortcut.translate_selection",
        "cloud/question-and-translate",
        "翻译选中的文字",
    ),
    (
        PageId::Phrases,
        "phrases.new_phrase",
        "personalize/phrases",
        "新增与编辑",
    ),
    (
        PageId::Phrases,
        "phrases.transfer",
        "personalize/phrases",
        "导入与导出",
    ),
    (
        PageId::Phrases,
        "phrases.system_replacements",
        "personalize/phrases",
        "系统的文本替换（macOS）",
    ),
    (
        PageId::Dictionaries,
        "dictionaries.import",
        "personalize/dictionaries",
        "导入自己的词库",
    ),
    (PageId::Cloud, "cloud.enabled", "cloud/index", "开启"),
    (
        PageId::Cloud,
        "cloud.cloud_words",
        "cloud/predict",
        "AI 候选词",
    ),
    (PageId::Cloud, "cloud.sentence", "cloud/predict", "整句补全"),
    (PageId::Cloud, "cloud.api_key", "cloud/index", "会发送什么"),
    (PageId::Cloud, "cloud.connection", "cloud/index", "开启"),
    (
        PageId::Advanced,
        "advanced.transfer",
        "advanced/data-files",
        "搬到另一台电脑",
    ),
    (
        PageId::Advanced,
        "advanced.verbose_log",
        "help/privacy",
        "运行日志",
    ),
    (
        PageId::Advanced,
        "advanced.input_log",
        "help/privacy",
        "输入日志",
    ),
    (
        PageId::About,
        "about.auto_check",
        "help/privacy",
        "检查更新",
    ),
    (PageId::About, "about.channel", "help/privacy", "检查更新"),
];

/// `page` 页上标题为 `title` 那一行的 ⓘ 链接。
pub fn link(page: PageId, title: &str) -> Option<String> {
    LINKS
        .iter()
        .find(|(on, row, ..)| *on == page && tr(row) == title)
        .map(|(_, _, doc, heading)| url(doc, heading))
}

fn url(doc: &str, heading: &str) -> String {
    if heading.is_empty() {
        return format!("{DOCS_URL}/{doc}");
    }
    format!("{DOCS_URL}/{doc}#{}", encode(&heading_id(heading)))
}

/// 同官网：首尾去空白，空白与 URL 保留字符的连续段换成一个连字符，中文原样留着。
fn heading_id(text: &str) -> String {
    let reserved = |c: char| c.is_whitespace() || "/?#%&+;,:@=$".contains(c);
    let mut id = String::new();
    let mut in_run = false;
    for c in text.trim().chars() {
        if reserved(c) {
            if !in_run {
                id.push('-');
            }
            in_run = true;
        } else {
            id.push(c);
            in_run = false;
        }
    }
    id
}

/// 非 ASCII 按 UTF-8 字节转成 `%XX`：交给系统打开时有的平台不认原样的中文网址。
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文档里的各级标题（去掉 `#` 与行内标记），同官网 `plainText` 的做法。
    fn headings(doc: &str) -> Vec<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/user")
            .join(format!("{doc}.md"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("读不了 {}：{error}", path.display()));
        text.lines()
            .filter_map(|line| {
                line.trim_start_matches('#')
                    .strip_prefix(' ')
                    .filter(|_| line.starts_with('#'))
            })
            .map(|heading| heading.replace(['`', '*', '_'], ""))
            .collect()
    }

    #[test]
    fn every_link_points_at_a_heading_that_exists() {
        for (page, row, doc, heading) in LINKS {
            let found = headings(doc);
            assert!(
                heading.is_empty()
                    || found
                        .iter()
                        .any(|text| heading_id(text) == heading_id(heading)),
                "{} 页「{row}」的 ⓘ 指向 {doc}.md 的「{heading}」，文档里没有这个标题",
                page.label()
            );
        }
    }

    #[test]
    fn every_link_belongs_to_a_row_with_an_info_mark() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-settings-docs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        // 选一种双拼：「输入框显示按键」「候选窗显示按键」只在双拼时建出来
        let shuangpin = qingjian_platform::Scheme::ALL[1];
        qingjian_platform::Config::set_value(&path, "general", "scheme", shuangpin.key()).unwrap();
        let settings = crate::settings::Settings::load(path);
        let index = crate::view::index(&settings);
        let _ = std::fs::remove_dir_all(&dir);
        // 只在别的平台显示的行这个平台上建不出来；短语卡片里的行只在卡片打开时建；
        // 「笔画」是随包码表的行，CI 只有样例数据、没有这张表
        let stroke = settings
            .library
            .bundled_codes
            .iter()
            .any(|entry| entry.stem == qingjian_platform::code_tables::STROKE);
        for (page, row, ..) in LINKS {
            if crate::support::elsewhere(*page, row)
                || (*page == PageId::Phrases && *row == "phrases.weight")
                || (*row == "笔画" && !stroke)
            {
                continue;
            }
            assert!(
                index.iter().any(|entry| entry.page == *page
                    && entry.title == tr(row)
                    && entry.detail.is_some()),
                "{} 页没有带 ⓘ 的「{row}」这一行",
                page.label()
            );
        }
    }

    #[test]
    fn anchors_follow_the_website() {
        assert_eq!(heading_id("拼音 / 注音"), "拼音-注音");
        assert_eq!(heading_id("混输（五笔 + 拼音）"), "混输（五笔-拼音）");
        assert_eq!(
            url("cloud/index", "开启"),
            "https://qingjian.app/docs/cloud/index#%E5%BC%80%E5%90%AF"
        );
    }
}
