//! 「高级」页：配置文件与数据目录、设置的导入导出，日志，输入日志。

use bamboo_slip::taffy::{FlexDirection, NodeId, Style};
use bamboo_slip::{Button, RenderError, SettingRow, Switch, Ui, Visual, group, section_title};
use qingjian_platform::LogLevel;

use crate::i18n::tr;
use crate::message::Message;
use crate::system::action::Action;
use crate::view::Page;
use crate::view::parts::actions::{actions, with_notice};
use crate::view::parts::controls::toggle_info;

pub fn advanced(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let general = &page.settings.config.general;
    let mut out = Vec::new();

    let buttons = actions(
        ui,
        page,
        &[
            (tr("advanced.open_in_editor"), Action::OpenConfig),
            (tr("advanced.open_data_dir"), Action::OpenDataDir),
        ],
    )?;
    let row = SettingRow::new(tr("advanced.config_file"))
        .control(buttons)
        .build(ui, theme)?;
    let import = Button::new(tr("advanced.import")).build(
        ui,
        theme,
        "import-config",
        Message::ImportConfig,
    )?;
    let export = Button::new(tr("advanced.export")).build(
        ui,
        theme,
        "export-config",
        Message::ExportConfig,
    )?;
    let transfer = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(8.0),
            ..Style::default()
        },
        Visual::Group,
        &[import, export],
    )?;
    let transfer = with_notice(
        ui,
        page,
        SettingRow::new(tr("advanced.transfer"))
            .info(tr("advanced.transfer_info"))
            .control(transfer),
        "config-transfer",
    )?;
    out.push(section_title(ui, theme, tr("advanced.section_config"))?);
    out.push(group(ui, theme, &[row, transfer])?);

    let verbose = general.log_level == LogLevel::Debug;
    let next = if verbose {
        LogLevel::Info
    } else {
        LogLevel::Debug
    };
    let switch = Switch::new(verbose).build(
        ui,
        theme,
        "verbose-log",
        Message::set("general", "log_level", next.key()),
    )?;
    let buttons = actions(
        ui,
        page,
        &[
            (tr("advanced.open_log_dir"), Action::OpenLogDir),
            (tr("advanced.export_logs"), Action::ExportLogs),
        ],
    )?;
    let rows = vec![
        SettingRow::new(tr("advanced.verbose_log"))
            .info(tr("advanced.verbose_log_info"))
            .control(switch)
            .build(ui, theme)?,
        with_notice(
            ui,
            page,
            SettingRow::new(tr("advanced.log_files")).control(buttons),
            "logs",
        )?,
    ];
    out.push(section_title(ui, theme, tr("advanced.section_logs"))?);
    out.push(group(ui, theme, &rows)?);

    let armed = page.settings.armed == Some(Action::ClearInputLog);
    let mut clear = Button::new(if armed {
        tr("advanced.clear_confirm")
    } else {
        tr("advanced.clear")
    });
    if armed {
        clear = clear.primary();
    }
    let clear = clear.build(
        ui,
        theme,
        "clear-input-log",
        Message::Run(Action::ClearInputLog),
    )?;
    let rows = vec![
        toggle_info(
            ui,
            page,
            tr("advanced.input_log"),
            Some(tr("advanced.input_log_info")),
            ("general", "input_log"),
            general.input_log,
            false,
        )?,
        with_notice(
            ui,
            page,
            SettingRow::new(tr("advanced.clear_input_log")).control(clear),
            "input-log",
        )?,
    ];
    out.push(section_title(ui, theme, tr("advanced.section_input_log"))?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}
