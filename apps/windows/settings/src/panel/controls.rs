//! 各页共用的表单零件（标签行、说明小字、整项、页外壳）与打开文件 / 目录、打包日志的小工具。

use std::path::{Path, PathBuf};

use windows_reactor::{
    AutomationExt, Border, Button, CheckBox, ChildrenControl, ContentControl, FontWeight, Grid,
    GridChildExt, GridLength, HorizontalAlignment, KeyedView, LayoutControl, Orientation,
    ScrollViewer, StackPanel, TextBlock, TextWrapping, ThemeBrush, VerticalAlignment, View,
    ViewContext,
};

use super::notice::Notice;
use super::{Message, Settings};
use crate::log;

/// 随包资源（相对随包根，如 `data/generated/dicts`），定位逻辑与 Server 共用。
pub(super) fn repo_resource(rel: &str) -> Option<PathBuf> {
    qingjian_platform::resources::bundled_resource(rel)
}

pub(super) fn open_in_editor(path: &Path) {
    if let Err(error) = std::process::Command::new("notepad").arg(path).spawn() {
        log::warn(format!("打开 {} 失败: {error}", path.display()));
    }
}

/// 资源管理器打开目录或网址。
pub(super) fn open_with_explorer(target: &str) {
    if let Err(error) = std::process::Command::new("explorer").arg(target).spawn() {
        log::warn(format!("打开 {target} 失败: {error}"));
    }
}

/// 三个进程共用的日志目录，没有就建出来（Server 没跑过时它还不存在）。
pub(super) fn log_dir() -> Option<PathBuf> {
    let dir = qingjian_platform::dirs::log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// 把整个日志目录加 `config.toml` 打成 `qingjian-logs-<日期>.zip` 放到桌面，再在资源管理器里选中它——
/// 用户反馈问题时一个附件搞定。压缩交给 PowerShell 的 Compress-Archive，不为此拉一个压缩库；
/// 桌面路径也让 PowerShell 取（OneDrive 会把桌面挪到别处）。脚本先写成临时 .ps1 再跑，免得命令行引号转义。
pub(super) fn export_logs() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Some(logs) = log_dir() else {
        return;
    };
    log::warn("用户导出日志");
    let mut sources = vec![format!("'{}\\*'", logs.display())];
    if let Some(config) = qingjian_platform::dirs::config_path().filter(|path| path.is_file()) {
        sources.push(format!("'{}'", config.display()));
    }
    let zip_name = format!(
        "qingjian-logs-{}.zip",
        jiff::Zoned::now().strftime("%Y-%m-%d")
    );
    let script = format!(
        "$zip = Join-Path ([Environment]::GetFolderPath('Desktop')) '{zip_name}'\n\
         Compress-Archive -Path {} -DestinationPath $zip -Force\n\
         explorer.exe \"/select,`\"$zip`\"\"\n",
        sources.join(",")
    );
    let script_path = std::env::temp_dir().join("qingjian-export-logs.ps1");
    if let Err(error) = std::fs::write(&script_path, script) {
        log::warn(format!("写导出脚本失败: {error}"));
        return;
    }
    let spawned = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
    if let Err(error) = spawned {
        log::warn(format!("导出日志失败: {error}"));
    }
}

/// 一行设置：标签自动换行，控件按内容占宽。
pub(super) fn labeled(label: &str, control: impl Into<View>) -> View {
    Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .column_spacing(24.0)
        .children([
            TextBlock::new()
                .text(label)
                .text_wrapping(TextWrapping::Wrap)
                .vertical_alignment(VerticalAlignment::Center)
                .into(),
            Border::new()
                .grid_column(1)
                .vertical_alignment(VerticalAlignment::Center)
                .content(control),
        ])
}

/// 灰色小字说明，可换行。
pub(super) fn note(text: &str) -> View {
    TextBlock::new()
        .text(text)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(13.0)
        .opacity(0.7)
        .into()
}

/// 一整项：左侧名称与说明、右侧控件，使用随系统明暗主题变化的原生卡片。
pub(super) fn field(label: &str, hint: &str, control: impl Into<View>) -> View {
    let mut description = vec![
        TextBlock::new()
            .text(label)
            .text_wrapping(TextWrapping::Wrap)
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
    ];
    if !hint.is_empty() {
        description.push(note(hint));
    }
    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(1.0)
        .corner_radius(8.0)
        .padding(16.0)
        .content(
            Grid::new()
                .columns([GridLength::STAR, GridLength::Auto])
                .column_spacing(24.0)
                .children([
                    StackPanel::new()
                        .spacing(6.0)
                        .vertical_alignment(VerticalAlignment::Center)
                        .keyed_children(
                            description
                                .into_iter()
                                .enumerate()
                                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
                        ),
                    Border::new()
                        .automation_name(label)
                        .max_width(360.0)
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(control),
                ]),
        )
}

/// 一组相关设置，标题与卡片间距在各页保持一致。
pub(super) fn section(title: &str, fields: impl IntoIterator<Item = View>) -> View {
    StackPanel::new().spacing(8.0).children([
        TextBlock::new()
            .text(title)
            .font_size(16.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        StackPanel::new().spacing(8.0).keyed_children(
            fields
                .into_iter()
                .enumerate()
                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
        ),
    ])
}

/// 在 `(界面名, 配置写法)` 列表里找 `value` 的下标，找不到取 0。
pub(super) fn index_of(options: &[(&str, &str)], value: &str) -> usize {
    options.iter().position(|(_, v)| *v == value).unwrap_or(0)
}

/// 一行「名称 · N 条 · 随包 / 许可证」，坏文件标出来：词库页与辅码页共用。
pub(super) fn entry_title(
    name: &str,
    entries: usize,
    license: &str,
    builtin: bool,
    broken: bool,
) -> String {
    if broken {
        return format!("{name}（文件损坏）");
    }
    let mut text = format!("{name} · {entries} 条");
    if builtin {
        text.push_str(" · 随包");
    } else if !license.is_empty() {
        text.push_str(&format!(" · {license}"));
    }
    text
}

/// 一行「复选框 + 可选的移除按钮」：词库页与辅码页共用，坏文件禁掉开关。
pub(super) fn check_row(
    stem: &str,
    label: String,
    enabled: bool,
    broken: bool,
    toggle: impl Fn(bool) -> Message + 'static,
    remove: Option<Message>,
    context: &mut ViewContext<Settings>,
) -> KeyedView {
    let check = CheckBox::new()
        .is_checked(enabled)
        .is_enabled(!broken)
        .on_is_checked_changed(context.callback(toggle))
        .content(label);
    let row = match remove {
        Some(message) => StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                check,
                Button::new()
                    .on_click(context.message(message))
                    .content("移除"),
            )),
        None => check,
    };
    KeyedView::new(stem.to_owned(), row)
}

/// 页面底部的提示：成功统计一行灰字、失败一行红字，都没有就不画。
pub(super) fn feedback(notice: &Notice) -> View {
    let mut lines: Vec<KeyedView> = Vec::new();
    if let Some(text) = &notice.note {
        lines.push(KeyedView::new("notice-note", note(text)));
    }
    if let Some(text) = &notice.error {
        lines.push(KeyedView::new(
            "notice-error",
            TextBlock::new()
                .text(text)
                .text_wrapping(TextWrapping::Wrap)
                .font_size(12.0)
                .foreground(ThemeBrush::SystemCritical),
        ));
    }
    StackPanel::new().spacing(4.0).keyed_children(lines)
}

/// 一页外壳：可滚动 + 大标题 + 内容。
pub(super) fn page(title: &str, body: impl Into<View>) -> View {
    let content = Border::new()
        .background(ThemeBrush::SolidBackground)
        .content(
            ScrollViewer::new().content(
                StackPanel::new()
                    .spacing(24.0)
                    .margin(28.0)
                    .max_width(1000.0)
                    .horizontal_alignment(HorizontalAlignment::Left)
                    .children([
                        TextBlock::new()
                            .text(title)
                            .font_size(28.0)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .into(),
                        body.into(),
                    ]),
            ),
        );
    // 切换页面重建滚动区，避免沿用上一页的滚动位置；同页修改仍保留焦点与位置。
    View::keyed_fragment([KeyedView::new(title.to_owned(), content)])
}
