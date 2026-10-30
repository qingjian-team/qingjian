//! 「中英文」页：英文模式（开关、切换键、默认英文与按应用记住、英文候选、Shift + 字母），标点与中西文空格，悬浮状态条。

use bamboo_slip::taffy::{FlexDirection, NodeId, Style};
use bamboo_slip::{Checkbox, RenderError, SettingRow, Ui, Visual, group, section_title};
use qingjian_platform::{DEFAULT_ENGLISH_CANDIDATES_ON, ShiftLetter, SwitchKey, ThirdInputMethod};

use crate::i18n::tr;
use crate::message::{Message, Popup};
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::{choice, percent_slider, select, toggle, toggle_info};
use crate::view::parts::menus::third_input_label;

/// 全角 / 半角两档，配置里是布尔。
const WIDTHS: [bool; 2] = [true, false];

pub fn language(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();

    let mut rows = vec![toggle_info(
        ui,
        page,
        tr("language.english_mode"),
        Some(tr("language.english_mode_info")),
        ("general", "english_mode"),
        general.english_mode,
        false,
    )?];
    if Feature::SwitchKeys.here() {
        rows.push(switch_keys(ui, page)?);
    }
    let third = general.double_shift_input;
    let third_names: Vec<&str> = ThirdInputMethod::ALL
        .iter()
        .map(|method| third_input_label(*method))
        .collect();
    let third_input = select(
        ui,
        page,
        Popup::ThirdInput,
        (third_input_label(third), &third_names),
        140.0,
        false,
    )?;
    rows.push(
        SettingRow::new(tr("language.double_shift_input"))
            .info(tr("language.double_shift_input_info"))
            .control(third_input)
            .build(ui, theme)?,
    );
    if Feature::DefaultEnglish.here() {
        rows.push(toggle_info(
            ui,
            page,
            tr("language.default_english"),
            Some(if Feature::RememberModePerApp.here() {
                tr("language.default_english_info")
            } else {
                tr("language.default_english_info_new_window")
            }),
            ("general", "default_english"),
            general.default_english,
            !general.english_mode,
        )?);
    }
    if Feature::RememberModePerApp.here() {
        rows.push(toggle_info(
            ui,
            page,
            tr("language.remember_per_app"),
            Some(tr("language.remember_per_app_info")),
            ("general", "remember_mode_per_app"),
            general.remember_mode_per_app,
            !general.english_mode,
        )?);
    }
    let only_in_apps = !config.apps.english_candidates_everywhere();
    let apps = if only_in_apps {
        vec!["*".to_owned()]
    } else {
        DEFAULT_ENGLISH_CANDIDATES_ON
            .iter()
            .map(|&app| app.to_owned())
            .collect()
    };
    let only = Checkbox::new(only_in_apps, tr("language.english_only_in_apps"))
        .disabled(!general.english_candidates)
        .build(
            ui,
            theme,
            "english-only-in-apps",
            Message::SetList("apps", "english_candidates_on", apps),
        )?;
    let switch = bamboo_slip::Switch::new(general.english_candidates).build(
        ui,
        theme,
        "english-candidates",
        Message::set("general", "english_candidates", !general.english_candidates),
    )?;
    rows.push(
        SettingRow::new(tr("language.english_candidates"))
            .info(tr("language.english_candidates_info"))
            .control(switch)
            .below(only)
            .build(ui, theme)?,
    );
    let shift = choice(
        ui,
        page,
        "shift-letter",
        &ShiftLetter::ALL,
        general.shift_letter,
        ShiftLetter::label,
        |value| Message::set("general", "shift_letter", value.key()),
    )?;
    rows.push(
        SettingRow::new(tr("language.shift_letter"))
            .info(tr("language.shift_letter_info"))
            .control(shift)
            .build(ui, theme)?,
    );
    out.push(section_title(ui, theme, tr("language.section_english"))?);
    out.push(group(ui, theme, &rows)?);

    let chinese = choice(
        ui,
        page,
        "full-width",
        &WIDTHS,
        general.full_width_punctuation,
        width_label,
        |value| Message::set("general", "full_width_punctuation", value),
    )?;
    let mut rows = vec![
        SettingRow::new(tr("language.chinese_punctuation"))
            .control(chinese)
            .build(ui, theme)?,
    ];
    if Feature::EnglishPunctuation.here() {
        let english = choice(
            ui,
            page,
            "english-full-width",
            &[false, true],
            general.english_full_width_punctuation,
            width_label,
            |value| Message::set("general", "english_full_width_punctuation", value),
        )?;
        rows.push(
            SettingRow::new(tr("language.english_punctuation"))
                .control(english)
                .build(ui, theme)?,
        );
    }
    rows.push(toggle_info(
        ui,
        page,
        tr("language.punct_commits"),
        Some(tr("language.punct_commits_info")),
        ("general", "punct_commits"),
        general.punct_commits,
        false,
    )?);
    if Feature::AutoSpace.here() {
        rows.push(toggle_info(
            ui,
            page,
            tr("language.auto_space"),
            Some(tr("language.auto_space_info")),
            ("general", "auto_space"),
            general.auto_space,
            false,
        )?);
    }
    out.push(section_title(
        ui,
        theme,
        tr("language.section_punctuation"),
    )?);
    out.push(group(ui, theme, &rows)?);

    if Feature::StatusBar.here() {
        let status = toggle(
            ui,
            page,
            tr("language.status_bar"),
            ("status_bar", "enabled"),
            config.status_bar.enabled,
        )?;
        let size = percent_slider(
            ui,
            page,
            (tr("language.status_bar_size"), ("status_bar", "size")),
            config.status_bar.size(),
            (60.0, 150.0),
            !config.status_bar.enabled,
        )?;
        out.push(section_title(ui, theme, tr("language.section_status_bar"))?);
        out.push(group(ui, theme, &[status, size])?);
    }
    Ok(out)
}

fn width_label(full: bool) -> &'static str {
    if full {
        tr("language.full_width")
    } else {
        tr("language.half_width")
    }
}

/// 中英切换键可以多选，一个都不勾就只剩按钮能切；最后一个是 Windows 自己的 Ctrl + Space，改的是系统设置。
/// macOS 只有单击 Shift 一项，勾上后 Caps Lock 只管大小写。
fn switch_keys(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let keys = page.settings.config.shortcut.switch_mode;
    let combos = Feature::SwitchKeyCombos.here();
    let mut boxes = Vec::new();
    for key in SwitchKey::ALL {
        if !combos && key != SwitchKey::Shift {
            continue;
        }
        let on = keys.contains(key);
        let values = keys
            .with(key, !on)
            .config_values()
            .into_iter()
            .map(str::to_owned)
            .collect();
        boxes.push(Checkbox::new(on, key.label()).build(
            ui,
            &page.theme,
            ("switch-key", key.key()),
            Message::SetList("shortcut", "switch_mode", values),
        )?);
    }
    if Feature::SystemCtrlSpace.here() {
        let system = page.settings.ctrl_space;
        boxes.push(
            Checkbox::new(system, tr("language.system_ctrl_space")).build(
                ui,
                &page.theme,
                "switch-key-system",
                Message::SystemCtrlSpace(!system),
            )?,
        );
    }
    let column = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            align_items: bamboo_slip::taffy::AlignItems::FLEX_START,
            gap: page.gap(8.0),
            ..Style::default()
        },
        Visual::Group,
        &boxes,
    )?;
    with_notice(
        ui,
        page,
        SettingRow::new(tr("language.switch_keys"))
            .info(if combos {
                tr("language.switch_keys_multi")
            } else {
                tr("language.switch_keys_caps")
            })
            .control(column),
        "switch-keys",
    )
}
