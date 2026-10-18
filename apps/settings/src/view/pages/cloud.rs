//! 「云服务」页：云联想（开关、云端词格数、整句补全、整句候选翻译、生词本整句翻译），服务（接口地址、模型、密钥、测试连接）。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Button, RenderError, SettingRow, Stepper, TextField, Ui, group, section_title};

use crate::message::{FieldId, Message};
use crate::support::Feature;
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
    let mut rows = vec![toggle_info(
        ui,
        page,
        "启用",
        Some("打字时把光标附近几十个字发给下面的服务，让模型补全整句、联想下文；密码框里不发"),
        ("predict", "enabled"),
        predict.enabled,
        false,
    )?];
    if Feature::CloudRequestControls.here() {
        rows.extend([
            toggle_info(
                ui,
                page,
                "自动联想",
                Some("停键后自动请求；关掉后在组句中按配置的快捷键手动联想，默认 Ctrl + Alt + J"),
                ("predict", "automatic"),
                predict.automatic,
                off,
            )?,
            timing(
                ui,
                page,
                "debounce_ms",
                predict.debounce_ms,
                off || !predict.automatic,
            )?,
            timing(ui, page, "min_interval_ms", predict.min_interval_ms, off)?,
        ]);
    }
    rows.extend([
        SettingRow::new("云端词")
            .info(if Feature::CloudRequestControls.here() {
                "自动结果补在第一页末尾，空格选本地首项；手动联想的结果放到前面的空位"
            } else {
                "云端的词到了补进第一页末尾这几格，前面的本地候选不动"
            })
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
    ]);
    if Feature::VocabularyTranslation.here() {
        rows.push(toggle_info(
            ui,
            page,
            "生词本整句翻译",
            Some("打完一句话后单独把这句发给服务，译文记进生词本的时间线；不发光标前后的文字"),
            ("predict", "vocabulary_translation"),
            predict.vocabulary_translation,
            off,
        )?);
    }
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

/// 请求节奏用步进控件，沿用 Windows 旧设置的范围；显示已保存的值，不在建页时改写配置。
fn timing(
    ui: &mut Ui<Message>,
    page: &Page,
    key: &'static str,
    value: u64,
    disabled: bool,
) -> Result<NodeId, RenderError> {
    let (title, info, minimum, maximum, step) = match key {
        "debounce_ms" => (
            "停键等待",
            "停止敲键多久后发出自动请求；继续输入会重新计时",
            100,
            10000,
            100,
        ),
        "min_interval_ms" => (
            "最短请求间隔",
            "两次请求起始至少相隔多久，手动请求也计入；缓存复用不联网",
            1000,
            60000,
            1000,
        ),
        _ => unreachable!("unknown prediction timing"),
    };
    let label = format!("{value} 毫秒");
    let control = Stepper::new(&label)
        .limits(value > minimum, value < maximum)
        .disabled(disabled)
        .build(
            ui,
            &page.theme,
            ("predict-timing", key),
            Message::set(
                "predict",
                key,
                value.saturating_sub(step).clamp(minimum, maximum) as i64,
            ),
            Message::set(
                "predict",
                key,
                value.saturating_add(step).clamp(minimum, maximum) as i64,
            ),
        )?;
    SettingRow::new(title)
        .info(info)
        .control(control)
        .build(ui, &page.theme)
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
