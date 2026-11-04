//! 「快捷键」页：翻页与模式（翻页键，平台接了的额外翻页键、高亮移动键、标点切换键，表达式、问字、? 进问字），候选上的键（平台接了的上屏键、译词上屏、删除、翻译选中文字），
//! 恢复默认，页底列组句中固定的键。

use bamboo_slip::taffy::{FlexDirection, LengthPercentage, NodeId, Rect, Style};
use bamboo_slip::{
    Button, Checkbox, KeyRecorder, RenderError, Segmented, SettingRow, Ui, Visual, group,
    paragraph, section_title,
};
use qingjian_core::ModeKeys;
use qingjian_platform::{HighlightKeys, PAGE_KEY_OPTIONS};

use crate::i18n::tr;
use crate::message::Message;
use crate::shortcut::Shortcut;
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::{choice, toggle_info};

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
        SettingRow::new(tr("shortcuts.page_keys"))
            .info(tr("shortcuts.page_keys_info"))
            .control(page_keys)
            .build(ui, theme)?,
    ];
    rows.extend(extra_keys(ui, page, &labels)?);
    rows.extend([
        SettingRow::new(tr("shortcuts.expression"))
            .info(tr("shortcuts.expression_info"))
            .control(mode_key(ui, page, "expression", mode.expression)?)
            .build(ui, theme)?,
        SettingRow::new(tr("shortcuts.lookup"))
            .info(tr("shortcuts.lookup_info"))
            .control(mode_key(ui, page, "question", mode.question)?)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            tr("shortcuts.question_lookup"),
            Some(tr("shortcuts.question_lookup_info")),
            ("shortcut", "question_mark"),
            mode.question_mark,
            false,
        )?,
    ]);
    out.push(section_title(ui, theme, tr("shortcuts.section_paging"))?);
    out.push(group(ui, theme, &rows)?);

    let mut rows = Vec::new();
    if Feature::CommitKeys.here() {
        rows.push(recorder(
            ui,
            page,
            Shortcut::CommitCandidate,
            tr("shortcuts.commit_candidate_info"),
            false,
        )?);
        rows.push(recorder(
            ui,
            page,
            Shortcut::CommitRaw,
            tr("shortcuts.commit_raw_info"),
            false,
        )?);
    }
    rows.extend([
        recorder(
            ui,
            page,
            Shortcut::Translation,
            tr("shortcuts.translation_info"),
            false,
        )?,
        recorder(
            ui,
            page,
            Shortcut::TranslationSecond,
            tr("shortcuts.translation_second_info"),
            false,
        )?,
        recorder(
            ui,
            page,
            Shortcut::TranslateSelection,
            tr("shortcuts.translate_selection_info"),
            false,
        )?,
    ]);
    if Feature::CloudRequestControls.here() {
        rows.push(recorder(
            ui,
            page,
            Shortcut::Predict,
            tr("shortcuts.predict_info"),
            false,
        )?);
    }
    if Feature::TranslatePanel.here() {
        rows.push(recorder(
            ui,
            page,
            Shortcut::TranslatePanel,
            tr("shortcuts.translate_panel_info"),
            false,
        )?);
    }
    out.push(section_title(
        ui,
        theme,
        tr("shortcuts.section_candidates"),
    )?);
    out.push(group(ui, theme, &rows)?);

    let reset = Button::new(tr("shortcuts.reset")).build(
        ui,
        theme,
        "reset-shortcuts",
        Message::ResetShortcuts,
    )?;
    let reset = SettingRow::new(tr("shortcuts.shortcuts"))
        .control(reset)
        .build(ui, theme)?;
    let fixed = if Feature::CommitKeys.here() {
        tr("shortcuts.fixed_without_commit")
    } else {
        tr("shortcuts.fixed")
    };
    let fixed = paragraph(ui, theme, fixed, None)?;
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

/// 额外翻页键、高亮移动键、切换全半角标点与简繁，平台的输入法接了哪个显示哪个。
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
            SettingRow::new(tr("shortcuts.extra_page_keys"))
                .info(tr("shortcuts.extra_page_keys_info"))
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
            SettingRow::new(tr("shortcuts.highlight_keys"))
                .info(tr("shortcuts.highlight_keys_info"))
                .control(highlight)
                .build(ui, theme)?,
        );
    }
    if Feature::ExtraCandidateNavigation.here() {
        let enabled = shortcut.extra_candidate_navigation;
        let checkbox = Checkbox::new(enabled, "Ctrl + F/B/N/P/A/E").build(
            ui,
            theme,
            "extra-candidate-navigation",
            Message::set("shortcut", "extra_candidate_navigation", !enabled),
        )?;
        rows.push(
            SettingRow::new(tr("shortcuts.extra_candidate_navigation"))
                .info(tr("shortcuts.extra_candidate_navigation_info"))
                .control(checkbox)
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
            SettingRow::new(tr("shortcuts.punctuation_width"))
                .info(tr("shortcuts.punctuation_width_info"))
                .control(switch)
                .build(ui, theme)?,
        );
    }
    if Feature::ToggleTraditional.here() {
        let traditional = shortcut.toggle_traditional.is_some();
        let switch = bamboo_slip::Switch::new(traditional).build(
            ui,
            theme,
            "toggle-traditional",
            Message::set(
                "shortcut",
                "toggle_traditional",
                if traditional { "" } else { "ctrl+shift+f" },
            ),
        )?;
        rows.push(
            SettingRow::new(tr("shortcuts.traditional"))
                .info(tr("shortcuts.traditional_info"))
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
