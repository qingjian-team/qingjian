//! 「词库」页：随包领域词库开关，用户导入词库的开关 / 移除 / 导入。
//! 随包开关写 `[dictionaries] domains`（列打开的），用户词库写 `disabled`（列关掉的）。

use std::path::{Path, PathBuf};

use qingjian_dictionary::Dictionary;
use qingjian_platform::extra_dictionaries;
use windows_reactor::*;

use crate::panel::controls::{check_row, entry_title, note, page, repo_resource};
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
        let label = entry_title(&name, entries, &license, true, broken);
        let for_msg = stem.clone();
        rows.push(check_row(
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
        let label = entry_title(&name, entries, &license, false, broken);
        let for_msg = stem.clone();
        let remove = Message::RemoveUserDict(stem.clone());
        rows.push(check_row(
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
                note("接受青简 TSV、Rime .dict.yaml 与现成的 .qj；导入即转换进上面的目录，同名覆盖。"),
            )),
        note(&settings.dictionary_status),
    ]);
    page("词库", body)
}

/// 挪进 `dicts\removed`，不真删（与 macOS 一致）。
pub(crate) fn remove_user_dict(settings: &mut Settings, stem: &str) {
    let dir = user_dir(settings);
    let Some((_, path)) = extra_dictionaries::list(&dir)
        .into_iter()
        .find(|(name, _)| name == stem)
    else {
        return;
    };
    let removed = dir.join("removed");
    if let Err(error) = std::fs::create_dir_all(&removed) {
        settings.dictionary_status = format!("移除失败：{error}");
        return;
    }
    if let Some(file_name) = path.file_name() {
        settings.dictionary_status = match std::fs::rename(&path, removed.join(file_name)) {
            Ok(()) => format!("已移除「{stem}」，输入法将自动更新。"),
            Err(error) => format!("移除失败：{error}"),
        };
    }
}

/// 多选词库，逐个转换并汇总结果；成功项的开关一次写回。
pub(crate) fn import(settings: &mut Settings) {
    let Some(sources) = rfd::FileDialog::new()
        .add_filter("词库文件", &["tsv", "yaml", "yml", "qj"])
        .add_filter("所有文件", &["*"])
        .set_title("导入词库")
        .pick_files()
    else {
        return;
    };
    let dir = user_dir(settings);
    let mut disabled = settings.config.dictionaries.disabled.clone();
    let mut results = Vec::new();
    let mut succeeded = 0;
    for source in &sources {
        match qingjian_core::dictionary::import::import(source, &dir) {
            Ok(imported) => {
                if let Some(stem) = imported.path.file_stem().and_then(|s| s.to_str()) {
                    disabled.retain(|name| name != stem);
                }
                succeeded += 1;
                results.push(format!(
                    "已导入「{}」，共 {} 条。",
                    imported.name, imported.entries
                ));
            }
            Err(error) => {
                let message = format!("{} 导入失败：{error}", source.display());
                crate::log::warn(&message);
                results.push(message);
            }
        }
    }
    let mut summary = format!(
        "导入完成：成功 {} 个，失败 {} 个。",
        succeeded,
        sources.len() - succeeded
    );
    if disabled != settings.config.dictionaries.disabled
        && let Err(error) = qingjian_platform::Config::set_array(
            &settings.path,
            "dictionaries",
            "disabled",
            &disabled,
        )
    {
        results.push(format!(
            "自动启用失败：{error}。此前关闭的词库需手动勾选启用。"
        ));
    } else if succeeded > 0 {
        summary.push_str("输入法将自动加载。");
    }
    settings.dictionary_status = format!("{summary}\n{}", results.join("\n"));
}
