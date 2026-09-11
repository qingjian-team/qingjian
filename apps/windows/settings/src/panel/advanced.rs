//! 「高级」设置页：打开配置文件 / 数据目录 / 日志目录、详细日志、输入日志（对齐 macOS「高级」页）。

use qingjian_platform::LogLevel;
use windows_reactor::*;

use super::{Message, Settings, field, note, page};

pub(super) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let rows = [
        note("设置改完会自动生效（Server 每秒看一次配置文件）。只有换学习语言要重启 Server。"),
        field(
            "配置文件",
            "",
            Button::new()
                .on_click(context.message(Message::OpenConfigFile))
                .content("在记事本中打开"),
        ),
        field(
            "数据目录",
            "",
            Button::new()
                .on_click(context.message(Message::OpenDataDir))
                .content("打开数据目录"),
        ),
        field(
            "日志目录",
            "",
            Button::new()
                .on_click(context.message(Message::OpenLogDir))
                .content("打开日志目录"),
        ),
        field(
            "详细日志",
            "排查问题时临时打开，会记下敲的拼音与上屏文字。",
            ToggleSwitch::new()
                .is_on(g.log_level == LogLevel::Debug)
                .on_toggled(context.callback(Message::VerboseLog)),
        ),
        field(
            "记录输入日志",
            "每次上屏记一行，只写本机、不上传，用于离线评测与个人模型。",
            ToggleSwitch::new()
                .is_on(g.input_log)
                .on_toggled(context.callback(Message::InputLog)),
        ),
        field(
            "清空输入日志",
            "",
            Button::new()
                .on_click(context.message(Message::ClearInputLog))
                .content("清空输入日志"),
        ),
    ];
    page("高级", StackPanel::new().spacing(16.0).children(rows))
}
