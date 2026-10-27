//! 「中英文」页：英文模式（开关、切换键、默认英文与按应用记住、英文候选、Shift + 字母），标点与中西文空格，悬浮状态条。

use bamboo_slip::taffy::{FlexDirection, NodeId, Style};
use bamboo_slip::{Checkbox, RenderError, SettingRow, Ui, Visual, group, section_title};
use qingjian_platform::{DEFAULT_ENGLISH_CANDIDATES_ON, ShiftLetter, SwitchKey};

use crate::message::Message;
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::{choice, percent_slider, toggle, toggle_info};

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
        "英文模式",
        Some("关掉后只打中文"),
        ("general", "english_mode"),
        general.english_mode,
        false,
    )?];
    if Feature::SwitchKeys.here() {
        rows.push(switch_keys(ui, page)?);
    }
    if Feature::DefaultEnglish.here() {
        rows.push(toggle_info(
            ui,
            page,
            "默认英文模式",
            Some(if Feature::RememberModePerApp.here() {
                "打开时先用英文"
            } else {
                "新打开的应用和窗口从英文开始"
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
            "按应用记住中英模式",
            Some("切回应用时恢复它上次的中英状态"),
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
    let only = Checkbox::new(only_in_apps, "只在写字的应用里给")
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
        SettingRow::new("英文模式也给候选")
            .info("Tab 或方向键选英文词")
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
        SettingRow::new("Shift + 字母")
            .info("进组句才能打出「C盘」")
            .control(shift)
            .build(ui, theme)?,
    );
    out.push(section_title(ui, theme, "英文")?);
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
        SettingRow::new("中文标点")
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
            SettingRow::new("英文标点")
                .control(english)
                .build(ui, theme)?,
        );
    }
    rows.push(toggle_info(
        ui,
        page,
        "标点先上屏候选",
        Some("nihao, 一气打完「你好，」"),
        ("general", "punct_commits"),
        general.punct_commits,
        false,
    )?);
    if Feature::AutoSpace.here() {
        rows.push(toggle_info(
            ui,
            page,
            "中文与英文、数字之间自动加空格",
            Some("「用Go写」上屏成「用 Go 写」"),
            ("general", "auto_space"),
            general.auto_space,
            false,
        )?);
    }
    out.push(section_title(ui, theme, "标点")?);
    out.push(group(ui, theme, &rows)?);

    if Feature::StatusBar.here() {
        let status = toggle(
            ui,
            page,
            "悬浮状态条",
            ("status_bar", "enabled"),
            config.status_bar.enabled,
        )?;
        let size = percent_slider(
            ui,
            page,
            ("状态条大小", ("status_bar", "size")),
            config.status_bar.size(),
            (60.0, 150.0),
            !config.status_bar.enabled,
        )?;
        out.push(section_title(ui, theme, "状态条")?);
        out.push(group(ui, theme, &[status, size])?);
    }
    Ok(out)
}

fn width_label(full: bool) -> &'static str {
    if full { "全角" } else { "半角" }
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
        boxes.push(Checkbox::new(system, "Ctrl + Space（系统）").build(
            ui,
            &page.theme,
            "switch-key-system",
            Message::SystemCtrlSpace(!system),
        )?);
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
        SettingRow::new("切换键")
            .info(if combos {
                "可以多选"
            } else {
                "勾上后 Caps Lock 只管大小写"
            })
            .control(column),
        "switch-keys",
    )
}
