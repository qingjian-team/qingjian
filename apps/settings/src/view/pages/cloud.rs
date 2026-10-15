//! 「云服务」页：云联想（开关、云端词格数、整句补全、整句候选翻译），服务（接口地址、模型、密钥、测试连接）。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Button, RenderError, SettingRow, Stepper, TextField, Ui, group, section_title};

use crate::message::{FieldId, Message};
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::toggle_info;

/// 云端词最多补几格。
const MAX_SLOTS: usize = 4;

/// 输入框的宽度（点）。
const FIELD_WIDTH: f32 = 260.0;

pub fn cloud(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let predict = &page.settings.config.predict;
    let off = !predict.enabled;
    let mut out = Vec::new();

    let slots = predict.slots.min(MAX_SLOTS);
    let label = if slots == 0 {
        "不要".to_owned()
    } else {
        format!("{slots} 格")
    };
    let stepper = Stepper::new(&label)
        .limits(slots > 0, slots < MAX_SLOTS)
        .disabled(off)
        .build(
            ui,
            theme,
            "slots",
            Message::set("predict", "slots", slots.saturating_sub(1) as i64),
            Message::set("predict", "slots", (slots + 1) as i64),
        )?;
    let rows = vec![
        toggle_info(
            ui,
            page,
            "启用",
            Some("打字时把光标附近几十个字发给下面的服务，让模型补全整句、联想下文；密码框里不发"),
            ("predict", "enabled"),
            predict.enabled,
            false,
        )?,
        SettingRow::new("云端词")
            .info("云端的词到了补进第一页末尾这几格，前面的本地候选不动")
            .control(stepper)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            "整句补全",
            Some("拼音后面给出整句，按 Tab 采用"),
            ("predict", "sentence"),
            predict.sentence,
            off,
        )?,
        toggle_info(
            ui,
            page,
            "整句候选翻译",
            Some(
                "本地整句候选出来后，单独把这条候选发给服务，译文随后显示在它右侧；不发光标前后的文字",
            ),
            ("predict", "sentence_translation"),
            predict.sentence_translation,
            off,
        )?,
    ];
    out.push(section_title(ui, theme, "云联想")?);
    out.push(group(ui, theme, &rows)?);

    let key_set = predict.resolve_api_key().is_some();
    let rows = vec![
        field(
            ui,
            page,
            FieldId::BaseUrl,
            "接口地址",
            None,
            "https://api.deepseek.com",
        )?,
        field(ui, page, FieldId::Model, "模型", None, "deepseek-v4-flash")?,
        field(
            ui,
            page,
            FieldId::ApiKey,
            "API 密钥",
            Some("只存在这台电脑的配置文件里，打包日志时会抹掉"),
            if key_set {
                "已设置，输入新的可替换"
            } else {
                "未设置"
            },
        )?,
        {
            let running = page.settings.cloud_test.running();
            let button = Button::new(if running {
                "测试中…"
            } else {
                "测试连接"
            })
            .disabled(running)
            .build(ui, theme, "test-cloud", Message::TestCloud)?;
            with_notice(
                ui,
                page,
                SettingRow::new("连接")
                    .info("用上面填的地址、模型、密钥发一条最短的请求")
                    .control(button),
                "cloud",
            )?
        },
    ];
    out.push(section_title(ui, theme, "服务")?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

fn field(
    ui: &mut Ui<Message>,
    page: &Page,
    id: FieldId,
    title: &str,
    info: Option<&str>,
    placeholder: &str,
) -> Result<NodeId, RenderError> {
    let Some(edit) = page.settings.fields.get(&id) else {
        return SettingRow::new(title).build(ui, &page.theme);
    };
    let input = TextField::new(edit)
        .placeholder(placeholder)
        .secret(id == FieldId::ApiKey)
        .width(FIELD_WIDTH)
        .build(
            ui,
            &page.theme,
            ("field", id),
            move |edit| Message::Edit(id, edit),
            Some(Message::Commit(id)),
            Some(Message::Cancel(id)),
        )?;
    let mut row = SettingRow::new(title).control(input);
    if let Some(info) = info {
        row = row.info(info);
    }
    row.build(ui, &page.theme)
}
