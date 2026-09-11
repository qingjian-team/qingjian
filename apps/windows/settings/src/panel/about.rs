//! 「关于」页：版本、许可证、随包数据的来源与署名、隐私说明与反馈方式（对齐 macOS「关于」页）。
//! 文案集中在常量里；第三方数据的许可证要求署名在分发物里可见，这一页就是放它的地方。

use windows_reactor::*;

use super::{Message, Settings, note, page};

/// 官网。
pub(super) const WEBSITE_URL: &str = "https://qingjian.app";

/// 源码与问题反馈。
pub(super) const REPOSITORY_URL: &str = "https://github.com/qingjian-team";

/// 许可说明，与仓库根目录 `LICENSE` 一致。
const LICENSE_NOTE: &str = "自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 随包数据的来源与许可证（与 macOS「关于」页一致）。
const ATTRIBUTIONS: &[(&str, &str)] = &[
    (
        "词库",
        "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音取自 Unihan（Unicode License v3）。",
    ),
    (
        "语言模型",
        "中文维基百科（CC BY-SA 4.0）与 LCCC（清华大学 CoAI，MIT）语料统计。",
    ),
    ("释义表", "由大语言模型（DeepSeek）生成，青简自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文词表",
        "ESDB / SCOWL（© Kevin Atkinson，按其许可保留版权声明）；CSpell 词典（MIT）。",
    ),
    (
        "词汇等级",
        "CEFR-J Wordlist v1.5（Yukio Tono，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY）。",
    ),
];

/// 隐私说明。
const PRIVACY_NOTE: &str = "青简不上传任何数据。开着云联想时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商（缺省 DeepSeek）的服务器，不经过作者。「高级」页的输入日志只写在本机，可以关掉或清空。";

/// 反馈方式。
const FEEDBACK_NOTE: &str = "遇到问题请把日志文件发给作者。缺省日志不含你敲的内容；排查排序问题时作者可能请你在「高级」页临时打开详细日志。";

pub(super) fn view(_settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let mut attributions: Vec<View> = Vec::with_capacity(ATTRIBUTIONS.len());
    for (name, text) in ATTRIBUTIONS {
        attributions.push(note(&format!("{name}：{text}")));
    }
    let body = StackPanel::new().spacing(12.0).children([
        TextBlock::new()
            .text(concat!("青简 Windows ", env!("CARGO_PKG_VERSION")))
            .font_size(16.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        // 构建标识：build.rs 在编译时塞进 git 分支 @ 短哈希与日期；拿不到 git 就是「本地构建」。
        note(&format!(
            "构建 {}",
            option_env!("QINGJIAN_BUILD").unwrap_or("本地构建")
        )),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::OpenWebsite))
                    .content("官网"),
                Button::new()
                    .on_click(context.message(Message::OpenRepository))
                    .content("GitHub"),
                Button::new()
                    .on_click(context.message(Message::OpenDataDir))
                    .content("打开数据目录"),
                Button::new()
                    .on_click(context.message(Message::OpenLogDir))
                    .content("打开日志目录"),
            )),
        note(LICENSE_NOTE),
        TextBlock::new()
            .text("数据来源与署名")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        StackPanel::new().spacing(6.0).keyed_children(
            attributions
                .into_iter()
                .enumerate()
                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
        ),
        note(PRIVACY_NOTE),
        note(FEEDBACK_NOTE),
    ]);
    page("关于", body)
}
