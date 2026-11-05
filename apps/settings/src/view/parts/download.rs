//! 下载进度那一行：模型卡片与词库行共用。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{ProgressBar, RenderError, SettingRow, Ui};

use crate::i18n::{tr, tr_args};
use crate::message::Message;
use crate::view::Page;

/// 进度条一行：上面写下了多少，下面一条进度条。
pub fn progress(
    ui: &mut Ui<Message>,
    page: &Page,
    done: u64,
    total: u64,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let fraction = if total > 0 {
        done as f32 / total as f32
    } else {
        0.0
    };
    let text = if total > 0 {
        tr_args(
            "experimental.downloading",
            &[("done", &megabytes(done)), ("total", &megabytes(total))],
        )
    } else {
        tr("experimental.connecting").to_owned()
    };
    let bar = ProgressBar::new(fraction).build(ui, theme)?;
    SettingRow::new(&text)
        .control(bar)
        .stacked()
        .build(ui, theme)
}

/// 下载进度与词库大小的写法：10 MB 以下留一位小数（领域词库多半不到 1 MB）。
pub fn megabytes(bytes: u64) -> String {
    let megabytes = bytes as f64 / 1_000_000.0;
    if megabytes < 10.0 {
        format!("{megabytes:.1} MB")
    } else {
        format!("{megabytes:.0} MB")
    }
}
