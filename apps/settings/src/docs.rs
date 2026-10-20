//! ⓘ 的文档链接：哪一页哪一行的 ⓘ 点了打开官网文档的哪一段。文档源是仓库的 `docs/user/`，官网按目录渲染，
//! 锚点照官网的 `headingId`（`web/src/lib/docs/render.ts`）生成；测试核对每一段在文档里真有这个标题，文档改标题时这里跟着改。

use crate::page_id::PageId;

const DOCS_URL: &str = "https://qingjian.app/docs";

/// 设置窗口这一页的说明，没有更专门的文档时 ⓘ 指到这里。
const SETTINGS: &str = "settings/preferences";

/// （页、行标题、文档（`docs/user/` 下的路径，不带 `.md`）、标题（空着是整页））。
const LINKS: &[(PageId, &str, &str, &str)] = &[
    (
        PageId::Input,
        "拼音",
        "input/fuzzy-and-shuangpin",
        "输入方案",
    ),
    (
        PageId::Input,
        "输入框显示按键",
        "input/fuzzy-and-shuangpin",
        "双拼",
    ),
    (
        PageId::Input,
        "五笔（86 版）",
        "input/fuzzy-and-shuangpin",
        "混输（五笔 + 拼音）",
    ),
    (PageId::Input, "启用", "input/aux-code", "用法"),
    (PageId::Input, "触发键", "input/aux-code", "触发键"),
    (PageId::Input, "导入码表", "input/aux-code", "码表"),
    (PageId::Candidates, "译文", "learning/translation", "译词"),
    (PageId::Candidates, "输出字形", "input/traditional", "开启"),
    (
        PageId::Candidates,
        "emoji",
        "input/shortcuts",
        "emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "颜文字",
        "input/shortcuts",
        "emoji、颜文字与符号",
    ),
    (
        PageId::Candidates,
        "符号",
        "input/shortcuts",
        "emoji、颜文字与符号",
    ),
    (PageId::Candidates, "学习输入习惯", SETTINGS, "候选"),
    (
        PageId::Cloud,
        "生词本整句翻译",
        "learning/translation",
        "生词本",
    ),
    (
        PageId::Vocabulary,
        "导出列出的词",
        "learning/translation",
        "生词本",
    ),
    (
        PageId::Vocabulary,
        "清空生词本",
        "learning/translation",
        "生词本",
    ),
    (PageId::Candidates, "本地整句模型", "input/local-model", ""),
    (
        PageId::Candidates,
        "打拼音时也给英文词",
        "input/shortcuts",
        "中英混输",
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
        "行为",
    ),
    (PageId::Language, "Shift + 字母", SETTINGS, "中英文"),
    (
        PageId::Language,
        "实时中英互译",
        "input/live-translation",
        "边输入边翻译",
    ),
    (
        PageId::Language,
        "本机服务地址",
        "input/live-translation",
        "开启",
    ),
    (
        PageId::Language,
        "本地翻译模型",
        "input/live-translation",
        "开启",
    ),
    (
        PageId::Language,
        "切换键",
        "input/english-mode",
        "改掉或关掉中 / 英切换（Windows）",
    ),
    (
        PageId::Language,
        "默认英文模式",
        "input/english-mode",
        "默认模式与按应用记忆（Windows）",
    ),
    (
        PageId::Language,
        "按应用记住中英模式",
        "input/english-mode",
        "默认模式与按应用记忆（Windows）",
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
        "input/shortcuts",
        "中英混输",
    ),
    (
        PageId::Appearance,
        "拼音显示位置",
        SETTINGS,
        "横排与拼音位置",
    ),
    (
        PageId::Shortcuts,
        "翻页键",
        "getting-started/keys",
        "输入组句时（拼音 / 注音 / 五笔）",
    ),
    (
        PageId::Shortcuts,
        "额外翻页键",
        "getting-started/keys",
        "输入组句时（拼音 / 注音 / 五笔）",
    ),
    (
        PageId::Shortcuts,
        "高亮移动键",
        "getting-started/keys",
        "输入组句时（拼音 / 注音 / 五笔）",
    ),
    (
        PageId::Shortcuts,
        "Ctrl + . 切换全半角标点",
        "getting-started/keys",
        "输入组句时（拼音 / 注音 / 五笔）",
    ),
    (
        PageId::Shortcuts,
        "表达式",
        "input/shortcuts",
        "日期、时间、算式",
    ),
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
        "问字",
    ),
    (
        PageId::Shortcuts,
        "上屏第一个译词",
        "learning/translation",
        "直接输出译词",
    ),
    (
        PageId::Shortcuts,
        "上屏第二个译词",
        "learning/translation",
        "直接输出译词",
    ),
    (
        PageId::Shortcuts,
        "删除候选",
        "learning/translation",
        "删除不需要的候选",
    ),
    (
        PageId::Shortcuts,
        "翻译选中文字",
        "learning/translation",
        "翻译选中的文字",
    ),
    (PageId::Phrases, "新增短语", SETTINGS, "自定义短语"),
    (PageId::Phrases, "位置", SETTINGS, "自定义短语"),
    (
        PageId::Phrases,
        "也用系统的文本替换",
        SETTINGS,
        "系统的文本替换",
    ),
    (
        PageId::Dictionaries,
        "导入词库",
        "settings/dictionaries",
        "导入用户词库",
    ),
    (PageId::Cloud, "启用", "cloud/index", "开启"),
    (PageId::Cloud, "云端词", "cloud/index", "云端候选词"),
    (PageId::Cloud, "整句补全", "cloud/index", "整句补全"),
    (
        PageId::Cloud,
        "整句候选翻译",
        "cloud/sentence-translation",
        "",
    ),
    (
        PageId::Cloud,
        "API 密钥",
        "help/data-and-logs",
        "云联想发什么",
    ),
    (PageId::Cloud, "连接", SETTINGS, "云服务"),
    (
        PageId::Advanced,
        "详细日志",
        "help/data-and-logs",
        "运行日志",
    ),
    (
        PageId::Advanced,
        "记录输入日志",
        "help/data-and-logs",
        "输入日志",
    ),
    (
        PageId::About,
        "自动检查更新",
        "help/data-and-logs",
        "检查更新",
    ),
    (PageId::About, "渠道", "help/data-and-logs", "检查更新"),
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
        // 只在别的平台显示的行这个平台上建不出来；短语卡片里的行只在卡片打开时建
        for (page, row, ..) in LINKS {
            if crate::support::elsewhere(*page, row) || (*page == PageId::Phrases && *row == "位置")
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
