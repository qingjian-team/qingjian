//! 「输入」页：方案（拼音、双拼显示按键、五笔），模糊音，辅码。

use bamboo_slip::taffy::LengthPercentage;
use bamboo_slip::taffy::{Dimension, FlexDirection, FlexWrap, NodeId, Rect, Size, Style};
use bamboo_slip::{Checkbox, RenderError, SettingRow, Switch, Ui, Visual, group, section_title};
use qingjian_platform::Scheme;

use super::shortcuts;
use crate::library::Kind;
use crate::message::{Message, Popup};
use crate::shortcut::Shortcut;
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::controls::{select, toggle_info, toggle_with};
use crate::view::parts::lists::{entry_row, import_row, toggle_disabled};

/// 模糊音的九条：配置键、界面名。
const FUZZY: [(&str, &str); 9] = [
    ("z_zh", "z = zh"),
    ("c_ch", "c = ch"),
    ("s_sh", "s = sh"),
    ("n_l", "n = l"),
    ("f_h", "f = h"),
    ("l_r", "l = r"),
    ("an_ang", "an = ang"),
    ("en_eng", "en = eng"),
    ("in_ing", "in = ing"),
];

pub fn input(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();

    let scheme = general.scheme();
    let names: Vec<&str> = Scheme::ALL.iter().map(|scheme| scheme.label()).collect();
    let control = select(
        ui,
        page,
        Popup::Scheme,
        (scheme.label(), &names),
        160.0,
        false,
    )?;
    let mut rows = vec![
        SettingRow::new("拼音")
            .info("双拼与注音下 v、u、i 是按键，表达式和问字改用 Shift + V、Shift + U 进")
            .control(control)
            .build(ui, theme)?,
    ];
    if scheme.is_shuangpin() {
        rows.push(toggle_info(
            ui,
            page,
            "输入框显示按键",
            Some("输入框里显示敲的字母，候选窗口里照旧是全拼"),
            ("general", "shuangpin_raw_preedit"),
            general.shuangpin_raw_preedit,
            false,
        )?);
    }
    let wubi = general.wubi();
    let switch = Switch::new(wubi).build(
        ui,
        theme,
        "wubi",
        Message::set("general", "wubi", if wubi { "" } else { "wubi86" }),
    )?;
    rows.push(
        SettingRow::new("五笔（86 版）")
            .info("跟拼音同时开就是混输，五笔打全的词排在前面")
            .control(switch)
            .build(ui, theme)?,
    );
    out.push(section_title(ui, theme, "方案")?);
    out.push(group(ui, theme, &rows)?);

    // 拼音关着时模糊音没有意义
    let pinyin_off = scheme == Scheme::Off;
    let fuzzy = fuzzy(ui, page, pinyin_off)?;
    out.push(section_title(ui, theme, "模糊音")?);
    out.push(group(ui, theme, &[fuzzy])?);

    if Feature::AuxCode.here() {
        out.extend(aux_code(ui, page)?);
    }
    Ok(out)
}

/// 辅码：开关、显示、触发键，再一组码表（随包的、导入的、导入按钮）。
fn aux_code(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();
    let aux = &config.aux_code;
    let rows = vec![
        toggle_info(
            ui,
            page,
            "启用",
            Some("拼音打完敲触发键（缺省是分号），再敲码筛选候选"),
            ("aux_code", "enabled"),
            aux.enabled,
            false,
        )?,
        toggle_with(
            ui,
            page,
            "候选上显示码",
            ("general", "aux_code_show"),
            general.aux_code_show,
            !aux.enabled,
        )?,
        toggle_with(
            ui,
            page,
            "码删空后留在辅码",
            ("general", "aux_code_keep_empty"),
            general.aux_code_keep_empty,
            !aux.enabled,
        )?,
        shortcuts::recorder(
            ui,
            page,
            Shortcut::AuxCodeKey,
            "拼音打完敲它进辅码；微软、搜狗双拼里分号先当韵母",
            !aux.enabled,
        )?,
    ];
    out.push(section_title(ui, theme, "辅码")?);
    out.push(group(ui, theme, &rows)?);

    let library = &page.settings.library;
    let mut rows = Vec::new();
    for (entry, removable) in library
        .bundled_codes
        .iter()
        .map(|entry| (entry, false))
        .chain(library.codes.iter().map(|entry| (entry, true)))
    {
        let toggle = toggle_disabled(Kind::CodeTable, &aux.disabled, &entry.stem);
        rows.push(entry_row(
            ui,
            page,
            Kind::CodeTable,
            entry,
            aux.is_enabled(&entry.stem),
            toggle,
            removable,
        )?);
    }
    rows.push(import_row(
        ui,
        page,
        Kind::CodeTable,
        "导入码表",
        "接受 Rime 的 .dict.yaml（词、码两列）与 .qj；码表要自己取得，青简不随包分发第三方形码表",
    )?);
    out.push(section_title(ui, theme, "码表")?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 九个勾选排成三列。
fn fuzzy(ui: &mut Ui<Message>, page: &Page, disabled: bool) -> Result<NodeId, RenderError> {
    let rules = &page.settings.config.fuzzy;
    let on = [
        rules.z_zh,
        rules.c_ch,
        rules.s_sh,
        rules.n_l,
        rules.f_h,
        rules.l_r,
        rules.an_ang,
        rules.en_eng,
        rules.in_ing,
    ];
    let mut cells = Vec::new();
    for ((key, label), on) in FUZZY.into_iter().zip(on) {
        let checkbox = Checkbox::new(on, label).disabled(disabled).build(
            ui,
            &page.theme,
            ("fuzzy", key),
            Message::set("fuzzy", key, !on),
        )?;
        cells.push(ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::percent(1.0 / 3.0),
                    height: Dimension::auto(),
                },
                ..Style::default()
            },
            Visual::Group,
            &[checkbox],
        )?);
    }
    let gap = LengthPercentage::length(page.px(14.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: Size {
                width: LengthPercentage::length(0.0),
                height: gap,
            },
            padding: Rect {
                top: gap,
                bottom: gap,
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &cells,
    )
}
