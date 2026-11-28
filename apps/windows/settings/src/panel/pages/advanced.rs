//! 「高级」页：打开配置文件 / 数据目录 / 日志目录、详细日志、学习开关、输入日志、闲置卸载。

use qingjian_platform::LogLevel;
use windows_reactor::*;

use crate::panel::controls::{field, note, page};
use crate::panel::{Message, Settings};

/// 闲置卸载的时长选项（秒）；0 = 不释放。
const IDLE_UNLOAD_CHOICES: [(&str, u64); 4] = [
    ("不释放", 0),
    ("5 分钟", 300),
    ("15 分钟", 900),
    ("1 小时", 3600),
];

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let idle_seconds = settings.config.model.idle_unload_seconds;
    let idle_index = IDLE_UNLOAD_CHOICES
        .iter()
        .position(|(_, seconds)| *seconds == idle_seconds)
        .unwrap_or(1);
    let rows = [
        note("设置改完会自动生效（Server 每秒看一次配置文件）。只有换学习语言要重启 Server。"),
        field(
            "释放闲置模型",
            "多少时间没有按键就把本地模型卸掉，内存回到只有词库的水平；下次按键在后台重新加载。没有加载模型时不起作用。",
            ComboBox::new()
                .items_source(IDLE_UNLOAD_CHOICES.iter().map(|(label, _)| *label))
                .selected_index(idle_index)
                .on_selection_changed(context.callback(Message::IdleUnload)),
        ),
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
            "输入法、引擎与设置程序的日志都在这一个目录，按天分文件，保留 7 天。",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Button::new()
                        .on_click(context.message(Message::OpenLogDir))
                        .content("打开日志目录"),
                    Button::new()
                        .on_click(context.message(Message::ExportLogs))
                        .content("打包日志到桌面"),
                )),
        ),
        field(
            "详细日志",
            "排查问题时临时打开，会记下敲的拼音与上屏文字。",
            ToggleSwitch::new()
                .is_on(g.log_level == LogLevel::Debug)
                .on_toggled(context.callback(Message::VerboseLog)),
        ),
        field(
            "学习输入习惯",
            "按你的选择调整候选顺序、记新词与敲错纠正。关掉后不再学，已学的仍参与排序；学习数据在数据目录里。",
            ToggleSwitch::new()
                .is_on(g.learning)
                .on_toggled(context.callback(Message::Learning)),
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
