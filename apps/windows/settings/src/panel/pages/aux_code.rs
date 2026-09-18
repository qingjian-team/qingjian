//! 「辅码」页：触发键录制、候选上显示码开关、码表列表（随包 + 导入）与导入码表。
//! 触发键与显示码写 `[general]`，码表开关写 `[aux_code] disabled`（列关掉的，与 `[dictionaries] disabled` 同形）。

use std::path::PathBuf;

use qingjian_core::is_valid_aux_code_key;
use qingjian_dictionary::{DictionaryError, aux_code_table_info, import_aux_code_table};
use qingjian_platform::code_tables;
use windows_reactor::*;

use crate::panel::controls::{check_row, entry_title, feedback, field, note, page, repo_resource};
use crate::panel::recorder::Recorder;
use crate::panel::{Message, Settings};

/// 随包码表目录：随包根下的 `codes/`，与 Server 装配同款。
const BUNDLED_DIR: &str = "codes";

/// 用户导入的码表目录 `%APPDATA%\Qingjian\codes`。
fn user_dir(settings: &Settings) -> PathBuf {
    settings.data_dir().join("codes")
}

/// 触发键控件：平时是显示当前键的按钮，点下去换成等按键的录制框。
fn record_control(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    match settings.recorder {
        Recorder::Idle => Button::new()
            .on_click(context.message(Message::AuxRecordStart))
            .content(settings.config.general.aux_code_key().to_string()),
        Recorder::Waiting { attempt } => {
            // 密码框承接按键：不走输入法，按 A–Z 不弹候选窗，字符直接进来（Shift 与键盘布局由系统处理）。
            // 一轮一代：敲过一次键就重建，框里不留上一轮的字符
            let record_box = settings.record_box.clone();
            context.use_effect("aux-code-record", attempt, move || {
                let _ = record_box.request_focus();
                None
            });
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .keyed_children([
                    KeyedView::new(
                        format!("record-{attempt}"),
                        PasswordBox::new()
                            .element_ref(&settings.record_box)
                            .placeholder_text("请按一个键")
                            .password_reveal_mode(PasswordRevealMode::Hidden)
                            .width(160.0)
                            .on_password_changed(context.callback(Message::AuxRecorded)),
                    ),
                    KeyedView::new(
                        "record-cancel",
                        Button::new()
                            .on_click(context.message(Message::AuxRecordCancel))
                            .content("取消"),
                    ),
                ])
        }
    }
}

/// 随包码表：标「随包」，不给移除（开关还是能关）。
fn bundled_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let Some(dir) = repo_resource(BUNDLED_DIR) else {
        return note("没找到随包码表目录（随包根的 codes/），随包笔画表装好后这里会列出来。");
    };
    let tables = code_tables::list(&dir);
    if tables.is_empty() {
        return note("随包码表目录是空的。");
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(tables.len());
    for (stem, path) in tables {
        let info = aux_code_table_info(&path);
        let enabled = settings.config.aux_code.is_enabled(&stem);
        let label = entry_title(&info.name, info.entries, &info.license, true, info.broken);
        let for_msg = stem.clone();
        rows.push(check_row(
            &stem,
            label,
            enabled,
            info.broken,
            move |on| Message::ToggleAuxTable(for_msg.clone(), on),
            None,
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

/// 自己导入的码表：可开关、可移除（挪进 codes\removed，不真删）。
fn user_list(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let dir = user_dir(settings);
    let tables = code_tables::list(&dir);
    if tables.is_empty() {
        return note(
            "还没有导入码表。点下面「导入码表」加一张，或把 .qj 放进 %APPDATA%\\Qingjian\\codes。",
        );
    }
    let mut rows: Vec<KeyedView> = Vec::with_capacity(tables.len());
    for (stem, path) in tables {
        let info = aux_code_table_info(&path);
        let enabled = settings.config.aux_code.is_enabled(&stem);
        let label = entry_title(&info.name, info.entries, &info.license, false, info.broken);
        let for_msg = stem.clone();
        let remove = Message::RemoveAuxTable(stem.clone());
        rows.push(check_row(
            &stem,
            label,
            enabled,
            info.broken,
            move |on| Message::ToggleAuxTable(for_msg.clone(), on),
            Some(remove),
            context,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(rows)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let body = StackPanel::new().spacing(12.0).children([
        note(
            "拼音打完之后敲触发键进辅码态，再敲码表里的字母把候选缩到这些码上；码表没覆盖的词在辅码态不出现。改完自动生效。",
        ),
        field(
            "启用辅码",
            "开着时拼音打完之后敲触发键进辅码态；关着时触发键完全保持原生行为（英文直输段 / 双拼韵母键）。没有任何可用码表时也不会进辅码态。",
            ToggleSwitch::new()
                .is_on(settings.config.aux_code.enabled)
                .on_toggled(context.callback(Message::AuxCodeEnabled)),
        ),
        field(
            "触发键",
            "点一下再按一个键；字母、数字、单引号与当前翻页键不能当触发键，微软、搜狗双拼里分号先当 ing 的韵母。",
            record_control(settings, context),
        ),
        field(
            "候选上显示码",
            "码与译文拼成一条注记，显示成「鹤 rbm · crane」。开着时不用进辅码：纯拼音打字候选也带词的码（首条），方便边打边记。",
            ToggleSwitch::new()
                .is_on(settings.config.general.aux_code_show)
                .on_toggled(context.callback(Message::AuxCodeShow)),
        ),
        field(
            "码删空后保持辅码",
            "删空码后 ; 仍在、候选全部回来，再按一次退格才退出辅码；关掉则删空即回拼音态。",
            ToggleSwitch::new()
                .is_on(settings.config.general.aux_code_keep_empty)
                .on_toggled(context.callback(Message::AuxCodeKeepEmpty)),
        ),
        TextBlock::new()
            .text("码表")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        bundled_list(settings, context),
        user_list(settings, context),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::ImportCodeTable))
                    .content("导入码表…"),
                note(
                    "接受 Rime 的 .dict.yaml（要有词、码两列）与现成的 .qj。码表由你自己取得，青简不随包分发第三方形码表。",
                ),
            )),
        note(
            "文件示例（词与码之间是制表符，另存为 my.dict.yaml 即可导入）：\n---\nname: 我的码表\nversion: 1\n...\n开发\tkf\n开放\tkf\n鹤\thn",
        ),
        feedback(&settings.notice),
    ]);
    page("辅码", body)
}

/// 录制框里敲进来的文本：合法就落盘，非法就红字提示、保持原值、继续等下一个键。
pub(crate) fn record_key(settings: &mut Settings, text: &str) {
    let Some(key) = text.chars().next() else {
        return;
    };
    let page_keys = settings.config.general.page_keys();
    if !is_valid_aux_code_key(key, page_keys) {
        settings.notice.fail(format!(
            "「{key}」不能当触发键：要单个可见的符号，字母、数字、单引号与当前翻页键（{}{}）不行。",
            page_keys.0, page_keys.1
        ));
        settings.recorder = settings.recorder.waiting();
        return;
    }
    settings.save("general", "aux_code_key", key.to_string());
    settings.notice.clear();
    settings.recorder = Recorder::Idle;
}

/// 挪进 `codes\removed`，不真删（与词库一致）。
pub(crate) fn remove_table(settings: &Settings, stem: &str) {
    let dir = user_dir(settings);
    let Some((_, path)) = code_tables::list(&dir)
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
        eprintln!("移除码表 {stem} 失败: {error}");
    }
}

/// 文件选择器选一张码表，转换后放进用户码表目录；统计进 note，失败进红字。
pub(crate) fn import(settings: &mut Settings) {
    let Some(source) = rfd::FileDialog::new()
        .add_filter("码表文件", &["yaml", "yml", "qj"])
        .add_filter("所有文件", &["*"])
        .set_title("导入码表")
        .pick_file()
    else {
        return;
    };
    match import_aux_code_table(&source, &user_dir(settings)) {
        Ok(imported) => {
            let report = imported.report;
            settings.notice.succeed(format!(
                "已导入码表「{}」：读入 {} · 有码 {} · 无码 {} · 跳过 {}",
                imported.name, report.read, report.with_code, report.no_code, report.skipped
            ));
        }
        Err(DictionaryError::NoCodeEntries) => settings
            .notice
            .fail("这个文件里没有识别到码表：要「词 + 码」两列，码只收 a–z。".to_owned()),
        Err(error) => settings.notice.fail(format!("导入码表失败：{error}")),
    }
}
