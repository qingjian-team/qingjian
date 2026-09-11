//! 各页共用的表单零件（标签行、说明小字、整项、页外壳）与打开文件 / 目录的小工具。

use std::path::{Path, PathBuf};

use windows_reactor::*;

use super::LABEL_WIDTH;

/// 随包资源（相对随包根，如 `data/generated/dicts`），定位逻辑与 Server 共用。
pub(super) fn repo_resource(rel: &str) -> Option<PathBuf> {
    qingjian_platform::resources::bundled_resource(rel)
}

pub(super) fn open_in_editor(path: &Path) {
    if let Err(error) = std::process::Command::new("notepad").arg(path).spawn() {
        eprintln!("打开 {} 失败: {error}", path.display());
    }
}

/// 资源管理器打开目录或网址。
pub(super) fn open_with_explorer(target: &str) {
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        eprintln!("打开 {target} 失败: {error}");
    }
}

/// 一行设置：固定宽标签 + 控件。
pub(super) fn labeled(label: &str, control: impl Into<View>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children([
            TextBlock::new().text(label).width(LABEL_WIDTH).into(),
            control.into(),
        ])
}

/// 灰色小字说明，可换行。
pub(super) fn note(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(12.0)
        .opacity(0.6)
        .into()
}

/// 一整项：「标签 + 控件」一行，下接说明（`hint` 为空则不加）。
pub(super) fn field(label: &str, hint: &str, control: impl Into<View>) -> View {
    let row = labeled(label, control);
    if hint.is_empty() {
        row
    } else {
        StackPanel::new().spacing(4.0).children([row, note(hint)])
    }
}

/// 在 `(界面名, 配置写法)` 列表里找 `value` 的下标，找不到取 0。
pub(super) fn index_of(options: &[(&str, &str)], value: &str) -> usize {
    options.iter().position(|(_, v)| *v == value).unwrap_or(0)
}

/// 一页外壳：可滚动 + 大标题 + 内容。
pub(super) fn page(title: &str, body: impl Into<View>) -> View {
    ScrollViewer::new().content(
        StackPanel::new().spacing(16.0).margin(24.0).children([
            TextBlock::new()
                .text(title)
                .font_size(24.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .into(),
            body.into(),
        ]),
    )
}
