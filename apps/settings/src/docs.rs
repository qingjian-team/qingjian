//! ⓘ 的文档链接：哪一页哪一行的 ⓘ 点了打开官网文档的哪一段。文档源是仓库的 `docs/user/`，官网按目录渲染，
//! 锚点照官网的 `headingId`（`web/src/lib/docs/render.ts`）生成；测试核对每一段在文档里真有这个标题，文档改标题时这里跟着改。

use crate::page_id::PageId;

const DOCS_URL: &str = "https://qingjian.app/docs";

/// （页、行标题、文档（`docs/user/` 下的路径，不带 `.md`）、标题（空着是整页））。
const LINKS: &[(PageId, &str, &str, &str)] = &[
    (
        PageId::Input,
        "拼音",
        "input/fuzzy-and-shuangpin",
        "选择方案",
    ),
    (
        PageId::Input,
        "输入框显示按键",
        "input/fuzzy-and-shuangpin",
        "双拼",
    ),
    (PageId::Input, "形码", "input/wubi", "开启"),
    (
        PageId::Input,
        "键位提示",
        "input/fuzzy-and-shuangpin",
        "键位提示",
    ),
    (PageId::Input, "启用", "input/aux-code", "开启"),
    (PageId::Input, "触发键", "input/aux-code", "触发键"),
    (PageId::Input, "笔画", "input/aux-code", "码表"),
    (PageId::Input, "导入码表", "input/aux-code", "码表"),
    (PageId::Candidates, "译文", "learning/translation", "译词"),
    (
        PageId::Candidates,
        "小键盘数字",
        "getting-started/keys",
        "打拼音时",
    ),
    (PageId::Candidates, "输出字形", "input/traditional", "开启"),
    (
        PageId::Candidates,
        "emoji",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "颜文字",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "符号",
        "input/punctuation",
        "候选后面的 emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "学习输入习惯",
        "personalize/habits",
        "关闭学习与从头开始",
    ),
    (
        PageId::Cloud,
        "生词本整句翻译",
        "cloud/sentence-translation",
        "生词本整句翻译",
    ),
    (
        PageId::Vocabulary,
        "导出列出的词",
        "learning/vocabulary",
        "两个视图共用",
    ),
    (
        PageId::Vocabulary,
        "清空生词本",
        "learning/vocabulary",
        "两个视图共用",
    ),
    (
        PageId::Candidates,
        "本地整句模型",
        "input/sentence",
        "本地整句模型",
    ),
    (
        PageId::Candidates,
        "打拼音时也给英文词",
        "input/english-mode",
        "在中文模式里打英文",
    ),
    (
        PageId::Language,
        "英文模式",
        "input/english-mode",
        "进入英文模式",
    ),
    (
        PageId::Language,
        "英文模式也给候选",
        "input/english-mode",
        "哪些应用里给候选",
    ),
    (
        PageId::Language,
        "Shift + 字母",
        "input/english-mode",
        "在中文模式里打英文",
    ),
    (
        PageId::Language,
        "切换键",
        "input/english-mode",
        "改掉或关掉中 / 英切换",
    ),
    (
        PageId::Language,
        "默认英文模式",
        "input/english-mode",
        "默认模式与按应用记忆",
    ),
    (
        PageId::Language,
        "按应用记住中英模式",
        "input/english-mode",
        "默认模式与按应用记忆",
    ),
    (
        PageId::Language,
        "中文与英文、数字之间自动加空格",
        "input/english-mode",
        "中西文自动空格",
    ),
    (
        PageId::Language,
        "标点先上屏候选",
        "input/punctuation",
        "打拼音时敲标点",
    ),
    (
        PageId::Appearance,
        "拼音显示位置",
        "personalize/appearance",
        "排布",
    ),
    (
        PageId::Shortcuts,
        "翻页键",
        "personalize/keys-settings",
        "翻页键",
    ),
    (
        PageId::Shortcuts,
        "额外翻页键",
        "personalize/keys-settings",
        "翻页键",
    ),
    (
        PageId::Shortcuts,
        "高亮移动键",
        "personalize/keys-settings",
        "高亮移动键（macOS、Windows）",
    ),
    (
        PageId::Shortcuts,
        "Ctrl + . 切换全半角标点",
        "personalize/keys-settings",
        "全半角标点快捷键（macOS、Windows）",
    ),
    (
        PageId::Shortcuts,
        "Ctrl + Shift + F 切换简繁输出",
        "personalize/keys-settings",
        "简繁切换快捷键（Windows）",
    ),
    (PageId::Shortcuts, "表达式", "input/shortcuts", "换前缀字母"),
    (
        PageId::Shortcuts,
        "问字",
        "cloud/question-and-translate",
        "问字",
    ),
    (
        PageId::Shortcuts,
        "? 也进问字",
        "cloud/question-and-translate",
        "用问号进入问字",
    ),
    (
        PageId::Shortcuts,
        "上屏第一个译词",
        "learning/translation",
        "直接上屏译词",
    ),
    (
        PageId::Shortcuts,
        "上屏第二个译词",
        "learning/translation",
        "直接上屏译词",
    ),
    (
        PageId::Shortcuts,
        "翻译选中文字",
        "cloud/question-and-translate",
        "翻译选中的文字",
    ),
    (
        PageId::Phrases,
        "新增短语",
        "personalize/phrases",
        "新增与编辑",
    ),
    (PageId::Phrases, "权重", "personalize/phrases", "排列规则"),
    (
        PageId::Phrases,
        "也用系统的文本替换",
        "personalize/phrases",
        "系统的文本替换（macOS）",
    ),
    (
        PageId::Dictionaries,
        "导入词库",
        "personalize/dictionaries",
        "导入自己的词库",
    ),
    (PageId::Cloud, "启用", "cloud/index", "开启"),
    (PageId::Cloud, "云端词", "cloud/predict", "AI 候选词"),
    (PageId::Cloud, "整句补全", "cloud/predict", "整句补全"),
    (
        PageId::Cloud,
        "整句候选翻译",
        "cloud/sentence-translation",
        "整句候选翻译",
    ),
    (PageId::Cloud, "API 密钥", "cloud/index", "会发送什么"),
    (PageId::Cloud, "连接", "cloud/index", "开启"),
    (PageId::Advanced, "详细日志", "help/privacy", "运行日志"),
    (PageId::Advanced, "记录输入日志", "help/privacy", "输入日志"),
    (PageId::About, "自动检查更新", "help/privacy", "检查更新"),
    (PageId::About, "渠道", "help/privacy", "检查更新"),
];

/// `page` 页上标题为 `title` 那一行的 ⓘ 链接。
pub fn link(page: PageId, title: &str) -> Option<String> {
    LINKS
        .iter()
        .find(|(on, row, ..)| *on == page && *row == title)
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
        // 选一种双拼：「输入框显示按键」只在双拼时建出来
        let shuangpin = qingjian_platform::Scheme::ALL[1];
        qingjian_platform::Config::set_value(&path, "general", "scheme", shuangpin.key()).unwrap();
        let settings = crate::settings::Settings::load(path).unwrap();
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
                || (*page == PageId::Phrases && *row == "权重")
                || (*row == "笔画" && !stroke)
            {
                continue;
            }
            assert!(
                index.iter().any(|entry| entry.page == *page
                    && entry.title == *row
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
