//! 「词库」页：随包领域词库开关，用户导入词库的开关 / 移除 / 导入。
//! 随包开关写 `[dictionaries] domains`（列打开的），用户词库写 `disabled`（列关掉的）。

use std::path::{Path, PathBuf};

use qingjian_core::dictionary::Dictionary;
use qingjian_platform::extra_dictionaries;
use windows_reactor::*;

use crate::panel::controls::{note, page, repo_resource};
use crate::panel::{Message, Settings};

/// 用户词库目录 `%APPDATA%\Qingjian\dicts`。
fn user_dir(settings: &Settings) -> PathBuf {
    settings.data_dir().join("dicts")
}

/// 打开词库读显示信息：(显示名, 词条数, 许可证, 是否坏文件)。
fn read_info(path: &Path, stem: &str) -> (String, usize, String, bool) {
    match Dictionary::from_path(path) {
        Ok(dict) => (
            dict.metadata()
                .map_or_else(|| stem.to_owned(), |m| m.name.clone()),
            dict.len(),
            dict.metadata()
                .map_or_else(String::new, |m| m.license.clone()),
            false,
        ),
        Err(_) => (stem.to_owned(), 0, String::new(), true),
    }
}

/// 「名称 · N 条 · 随包 / 许可证」，坏文件标出来。
fn title(name: &str, entries: usize, license: &str, builtin: bool, broken: bool) -> String {
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

/// 一本词库一行：复选框 + 可选的「移除」。
fn dict_row(
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

fn bundled_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let Some(dir) = repo_resource("data/generated/dicts") else {
        return note("没找到随包领域词库目录（安装布局待定）。");
    };
    let dicts = extra_dictionaries::list(&dir);
    if dicts.is_empty() {
        return note("随包领域词库目录是空的。");
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(dicts.len());
    for (stem, path) in dicts {
        let (name, entries, license, broken) = read_info(&path, &stem);
        let enabled = settings.config.dictionaries.is_domain_enabled(&stem);
        let label = title(&name, entries, &license, true, broken);
        let for_msg = stem.clone();
        rows.push(dict_row(
            &stem,
            label,
            enabled,
            broken,
            move |on| Message::ToggleDomain(for_msg.clone(), on),
            None,
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

fn user_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let dicts = extra_dictionaries::list(&user_dir(settings));
    if dicts.is_empty() {
        return note(
            "还没有导入词库。点下面「导入词库」加一本，或把文件放进 %APPDATA%\\Qingjian\\dicts。",
        );
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(dicts.len());
    for (stem, path) in dicts {
        let (name, entries, license, broken) = read_info(&path, &stem);
        let enabled = settings.config.dictionaries.is_enabled(&stem);
        let label = title(&name, entries, &license, false, broken);
        let for_msg = stem.clone();
        let remove = Message::RemoveUserDict(stem.clone());
        rows.push(dict_row(
            &stem,
            label,
            enabled,
            broken,
            move |on| Message::ToggleUserDict(for_msg.clone(), on),
            Some(remove),
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let body = StackPanel::new().spacing(12.0).children([
        note("随包的基础词库始终启用，不在这里。这里管随包领域词库的开关与导入词库的开关 / 移除。改完自动生效。"),
        TextBlock::new()
            .text("随包领域词库")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        bundled_list(settings, context),
        TextBlock::new()
            .text("导入的词库")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        user_list(settings, context),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::ImportDictionary))
                    .content("导入词库…"),
                note("接受青简 TSV、Rime .dict.yaml、.qj；导入即复制进上面的目录。"),
            )),
    ]);
    page("词库", body)
}

/// 挪进 `dicts\removed`，不真删（与 macOS 一致）。
pub(crate) fn remove_user_dict(settings: &Settings, stem: &str) {
    let dir = user_dir(settings);
    let Some((_, path)) = extra_dictionaries::list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
    else {
        return;
    };
    let removed = dir.join("removed");
    if let Err(error) = std::fs::create_dir_all(&removed) {
        eprintln!("建 removed 目录失败: {error}");
        return;
    }
    if let Some(file_name) = path.file_name()
        && let Err(error) = std::fs::rename(&path, removed.join(file_name))
    {
        eprintln!("移除词库 {stem} 失败: {error}");
    }
}

/// 文件选择器选一本，复制进用户词库目录。
pub(crate) fn import(settings: &Settings) {
    let Some(source) = rfd::FileDialog::new()
        .add_filter("词库文件", &["tsv", "yaml", "yml", "qj"])
        .add_filter("所有文件", &["*"])
        .set_title("导入词库")
        .pick_file()
    else {
        return;
    };
    let dir = user_dir(settings);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("建用户词库目录失败: {error}");
        return;
    }
    let Some(file_name) = source.file_name() else {
        return;
    };
    if let Err(error) = std::fs::copy(&source, dir.join(file_name)) {
        eprintln!("导入词库失败: {error}");
    }
}
