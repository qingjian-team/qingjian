//! 「快捷键」页：翻页与模式（翻页键，Windows 的额外翻页键、高亮移动键、标点切换键，表达式、问字、? 进问字），候选上的组合键（译词上屏、删除、翻译选中文字），
//! 恢复默认，页底列组句中固定的键。

use bamboo_slip::taffy::{FlexDirection, LengthPercentage, NodeId, Rect, Style};
use bamboo_slip::{
    Button, Checkbox, KeyRecorder, RenderError, Segmented, SettingRow, Ui, Visual, group,
    paragraph, section_title,
};
use qingjian_core::ModeKeys;
use qingjian_platform::{HighlightKeys, PAGE_KEY_OPTIONS};

use crate::message::Message;
use crate::shortcut::Shortcut;
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::{choice, toggle_info};

/// 组句中固定、不能改的键。
const FIXED: &str = if cfg!(target_os = "macos") {
    "组句中固定的键：空格上屏首选，1–9 选词，回车原样上屏，Esc 清空；⌥⌫ 删一个音节，⌘⌫ 删到开头；\
     ⌥← / ⌥→ 按音节跳，⌘← / ⌘→ 到开头 / 末尾；↑ / ↓ 移动高亮，PageUp / PageDown 与 ⇧Tab 翻页；Tab 采用整句补全。"
} else {
    "组句中固定的键：空格上屏首选，1–9 选词，回车原样上屏，Esc 清空；← / → 移动拼音光标，Home / End 到开头 / 末尾；\
     ↑ / ↓ 移动高亮，PageUp / PageDown 与 Shift + Tab 翻页；Tab 采用整句补全。"
};

pub fn shortcuts(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let mut out = Vec::new();

    let (previous, next) = config.general.page_keys();
    let pair = format!("{previous}{next}");
    let labels: Vec<String> = PAGE_KEY_OPTIONS
        .iter()
        .map(|keys| keys.chars().map(String::from).collect::<Vec<_>>().join(" "))
        .collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let selected = PAGE_KEY_OPTIONS
        .iter()
        .position(|keys| *keys == pair)
        .unwrap_or(0);
    let page_keys = Segmented::new(&labels, selected).build(ui, theme, "page-keys", |index| {
        Message::set("general", "page_keys", PAGE_KEY_OPTIONS[index])
    })?;
    let mode = config.shortcut.mode.sanitized();
    let mut rows = vec![
        SettingRow::new("翻页键")
            .info("选逗号句号或减号等号时，组句中敲它们是翻页，不再上屏加标点")
            .control(page_keys)
            .build(ui, theme)?,
    ];
    rows.extend(extra_keys(ui, page, &labels)?);
    rows.extend([
        SettingRow::new("表达式")
            .info("这个字母开头进表达式：v1+2 出 3")
            .control(mode_key(ui, page, "expression", mode.expression)?)
            .build(ui, theme)?,
        SettingRow::new("问字")
            .info("这个字母开头问字：usangemu 问「三个木」（要开云服务），u4e00 出对应的字")
            .control(mode_key(ui, page, "question", mode.question)?)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            "? 也进问字",
            Some("没在打拼音时敲 ? 先进问字，后面跟字母才是问题，跟别的键还原成问号"),
            ("shortcut", "question_mark"),
            mode.question_mark,
            false,
        )?,
    ]);
    out.push(section_title(ui, theme, "翻页与模式")?);
    out.push(group(ui, theme, &rows)?);

    let mut rows = vec![
        recorder(
            ui,
            page,
            Shortcut::Translation,
            "按住它再按候选序号，上屏候选右侧的译词而不是中文",
            false,
        )?,
        recorder(
            ui,
            page,
            Shortcut::TranslationSecond,
            "候选有两个译词时上屏后一个",
            false,
        )?,
        recorder(
            ui,
            page,
            Shortcut::DeleteCandidate,
            "按住它再按候选序号：自己造的词整个删掉，词库里的词清掉学习记录",
            false,
        )?,
        recorder(
            ui,
            page,
            Shortcut::TranslateSelection,
            "在应用里选中文字再按它，译文出现在候选窗口：回车替换，Esc 保留原文。要开云服务",
            false,
        )?,
    ];
    if Feature::CloudRequestControls.here() {
        rows.push(recorder(
            ui,
            page,
            Shortcut::Predict,
            "组句中手动请求 AI 候选，默认 Ctrl + Alt + J；关闭自动联想后仍可使用",
            false,
        )?);
    }
    out.push(section_title(ui, theme, "候选")?);
    out.push(group(ui, theme, &rows)?);

    let reset =
        Button::new("恢复默认").build(ui, theme, "reset-shortcuts", Message::ResetShortcuts)?;
    let reset = SettingRow::new("快捷键").control(reset).build(ui, theme)?;
    let fixed = paragraph(ui, theme, FIXED, None)?;
    // 与分组标题一样往里缩一点
    let inset = LengthPercentage::length(page.px(4.0));
    let fixed = ui.scene().node(
        Style {
            padding: Rect {
                left: inset,
                right: inset,
                top: LengthPercentage::length(0.0),
                bottom: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[fixed],
    )?;
    out.push(group(ui, theme, &[reset])?);
    out.push(fixed);
    Ok(out)
}

/// 额外翻页键、高亮移动键、切换全半角标点，平台的输入法接了哪个显示哪个。
fn extra_keys(
    ui: &mut Ui<Message>,
    page: &Page,
    labels: &[&str],
) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let shortcut = &page.settings.config.shortcut;
    let mut rows = Vec::new();
    if Feature::ExtraPageKeys.here() {
        rows.push(
            SettingRow::new("额外翻页键")
                .info("跟上面的翻页键一起用；表达式、英文直输段、辅码触发键照旧")
                .control(page_key_boxes(ui, page, labels)?)
                .build(ui, theme)?,
        );
    }
    if Feature::HighlightKeys.here() {
        let highlight = choice(
            ui,
            page,
            "highlight-keys",
            &HighlightKeys::ALL,
            shortcut.highlight_keys,
            HighlightKeys::label,
            |value| Message::set("shortcut", "highlight_keys", value.key()),
        )?;
        rows.push(
            SettingRow::new("高亮移动键")
                .info("选 ← / → 时连着拼音行一起走：拼音光标到末尾按 → 才到下一个候选，候选在首位按 ← 退回拼音行；↑ / ↓ 不做事")
                .control(highlight)
                .build(ui, theme)?,
        );
    }
    if Feature::TogglePunctuation.here() {
        let punctuation = shortcut.toggle_punctuation.is_some();
        let switch = bamboo_slip::Switch::new(punctuation).build(
            ui,
            theme,
            "toggle-punctuation",
            Message::set(
                "shortcut",
                "toggle_punctuation",
                if punctuation { "" } else { "ctrl+." },
            ),
        )?;
        rows.push(
            SettingRow::new("Ctrl + . 切换全半角标点")
                .info("切的是当前中英模式的标点；配置文件里 [shortcut] toggle_punctuation 可以换别的组合键")
                .control(switch)
                .build(ui, theme)?,
        );
    }
    Ok(rows)
}

/// 额外翻页键的九个勾选排成一行。
fn page_key_boxes(
    ui: &mut Ui<Message>,
    page: &Page,
    labels: &[&str],
) -> Result<NodeId, RenderError> {
    let extra = page.settings.config.shortcut.extra_page_keys;
    let mut boxes = Vec::new();
    for (pair, label) in PAGE_KEY_OPTIONS.into_iter().zip(labels) {
        let on = extra.contains(pair);
        boxes.push(Checkbox::new(on, label).build(
            ui,
            &page.theme,
            ("extra-page-keys", pair),
            Message::SetList("shortcut", "extra_page_keys", extra.with(pair, !on).into()),
        )?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(12.0),
            ..Style::default()
        },
        Visual::Group,
        &boxes,
    )
}

fn mode_key(
    ui: &mut Ui<Message>,
    page: &Page,
    key: &'static str,
    current: char,
) -> Result<NodeId, RenderError> {
    let labels: Vec<String> = ModeKeys::CANDIDATES.iter().map(char::to_string).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let selected = ModeKeys::CANDIDATES
        .iter()
        .position(|letter| *letter == current)
        .unwrap_or(0);
    Segmented::new(&labels, selected).build(ui, &page.theme, ("mode-key", key), move |index| {
        Message::ModeKey(key, ModeKeys::CANDIDATES[index])
    })
}

/// 一行录键：标题、ⓘ、录键按钮，录得不对时下面一行说原因。
pub fn recorder(
    ui: &mut Ui<Message>,
    page: &Page,
    shortcut: Shortcut,
    info: &str,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    let current = shortcut.current(&page.settings.config);
    let control = KeyRecorder::new(&current)
        .prompt(shortcut.prompt())
        .disabled(disabled)
        .recording(page.settings.recording == Some(shortcut))
        .build(
            ui,
            &page.theme,
            ("recorder", shortcut),
            Message::Record(shortcut),
            move |press| Message::Recorded(shortcut, press.clone()),
            Message::StopRecording,
        )?;
    with_notice(
        ui,
        page,
        SettingRow::new(shortcut.title())
            .info(info)
            .control(control),
        shortcut.title(),
    )
}
