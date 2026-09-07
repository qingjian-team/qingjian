//! 「关于」页：版本与构建、许可证、随包数据的来源与署名、隐私说明与反馈方式。
//!
//! 文案集中在这里的常量里，改措辞不用碰布局代码。第三方数据的许可证要求署名在分发物里可见，这一页就是放它的地方。

use objc2::MainThreadMarker;
use objc2_app_kit::NSFont;
use objc2_app_kit::NSTextField;
use objc2_foundation::NSString;

use crate::preferences::controls::{GROUP_GAP, button, note_full, small_label};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 许可说明，与仓库根目录 `LICENSE` 一致。
pub const LICENSE_NOTE: &str = "自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 随包数据的来源与许可证。改数据来源时同步改这里和 `apps/macos/scripts/bundle.sh` 里 `pack` 的署名。
pub const ATTRIBUTIONS: &[(&str, &str)] = &[
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
        "The CEFR-J Wordlist Version 1.5（Yukio Tono，Tokyo University of Foreign Studies，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY；经 elzup/jlpt-word-list 整理，MIT）。",
    ),
];

/// 官网。
pub const WEBSITE_URL: &str = "https://qingjian.app";

/// 源码与问题反馈。
pub const REPOSITORY_URL: &str = "https://github.com/qingjian-team";

/// 隐私说明。
pub const PRIVACY_NOTE: &str = "青简不上传任何数据。开着云联想或翻译时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商（缺省 DeepSeek）的服务器，不经过作者。「高级」页的输入日志只写在这台电脑的数据目录里，可以关掉或清空。";

/// 反馈方式。
pub const FEEDBACK_NOTE: &str = "遇到问题请把当天的日志文件发给作者，再附上「复制诊断信息」的内容。缺省日志不含你敲的内容；排查排序问题时作者可能请你在「高级」页临时打开详细日志。";

/// 把「关于」页的控件摆进 `layout`。
pub fn build(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    target: &PreferencesTarget,
    version: &str,
    build: &str,
) {
    let title = NSTextField::labelWithString(&NSString::from_str(&format!("青简 {version}")), mtm);
    title.setFont(Some(&NSFont::boldSystemFontOfSize(15.0)));
    layout.place(&title, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
    let build_label = small_label(mtm, &format!("构建 {build}"));
    layout.place(
        &build_label,
        PAGE_PADDING,
        layout.inner_width(),
        ROW_HEIGHT * 0.7,
    );
    layout.next_row(ROW_HEIGHT * 0.7);
    let website = button(mtm, "官网", Setting::OpenWebsite, target);
    let repository = button(mtm, "GitHub", Setting::OpenRepository, target);
    layout.place(&website, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
    layout.place(&repository, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
    layout.next_row(ROW_HEIGHT + 4.0);
    note_full(layout, mtm, LICENSE_NOTE);
    layout.space(GROUP_GAP);

    let heading = small_label(mtm, "数据来源与署名");
    layout.place(
        &heading,
        PAGE_PADDING,
        layout.inner_width(),
        ROW_HEIGHT * 0.7,
    );
    layout.next_row(ROW_HEIGHT * 0.7);
    for (name, text) in ATTRIBUTIONS {
        note_full(layout, mtm, &format!("{name}：{text}"));
    }
    layout.space(GROUP_GAP);

    note_full(layout, mtm, PRIVACY_NOTE);
    note_full(layout, mtm, FEEDBACK_NOTE);
    let open = button(mtm, "打开日志目录", Setting::OpenLogDirectory, target);
    let copy = button(mtm, "复制诊断信息", Setting::CopyDiagnostics, target);
    layout.place(&open, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
    layout.place(&copy, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
    layout.next_row(ROW_HEIGHT + 4.0);
}
