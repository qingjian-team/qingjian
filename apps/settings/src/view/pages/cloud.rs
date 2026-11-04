//! 「云服务」页：云联想（开关、云端词格数、整句补全、生词本整句翻译），服务（接口地址、模型、密钥、测试连接）。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::{Button, RenderError, SettingRow, Stepper, TextField, Ui, group, section_title};

use crate::i18n::{tr, tr_args};
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
        tr("cloud.slots_none").to_owned()
    } else {
        tr_args("cloud.slots", &[("n", &slots)])
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
        tr("cloud.enabled"),
        Some(tr("cloud.enabled_info")),
        ("predict", "enabled"),
        predict.enabled,
        false,
    )?];
    if Feature::CloudRequestControls.here() {
        rows.extend([
            toggle_info(
                ui,
                page,
                tr("cloud.auto"),
                Some(tr("cloud.auto_info")),
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
        SettingRow::new(tr("cloud.cloud_words"))
            .info(if Feature::CloudRequestControls.here() {
                tr("cloud.cloud_words_info")
            } else {
                tr("cloud.cloud_words_info_slots")
            })
            .control(stepper)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            tr("cloud.sentence"),
            Some(tr("cloud.sentence_info")),
            ("predict", "sentence"),
            predict.sentence,
            off,
        )?,
    ]);
    if Feature::VocabularyTranslation.here() {
        rows.push(toggle_info(
            ui,
            page,
            tr("cloud.vocabulary_translation"),
            Some(tr("cloud.vocabulary_translation_info")),
            ("predict", "vocabulary_translation"),
            predict.vocabulary_translation,
            off,
        )?);
    }
    out.push(section_title(ui, theme, tr("cloud.section_prediction"))?);
    out.push(group(ui, theme, &rows)?);

    let key_set = predict.resolve_api_key().is_some();
    let rows = vec![
        field(
            ui,
            page,
            FieldId::BaseUrl,
            tr("cloud.base_url"),
            None,
            "https://api.deepseek.com",
        )?,
        field(
            ui,
            page,
            FieldId::Model,
            tr("cloud.model"),
            None,
            "deepseek-v4-flash",
        )?,
        field(
            ui,
            page,
            FieldId::ApiKey,
            tr("cloud.api_key"),
            Some(tr("cloud.api_key_info")),
            if key_set {
                tr("cloud.api_key_set")
            } else {
                tr("cloud.api_key_unset")
            },
        )?,
        {
            let running = page.settings.cloud_test.running();
            let button = Button::new(if running {
                tr("cloud.testing")
            } else {
                tr("cloud.test")
            })
            .disabled(running)
            .build(ui, theme, "test-cloud", Message::TestCloud)?;
            with_notice(
                ui,
                page,
                SettingRow::new(tr("cloud.connection"))
                    .info(tr("cloud.connection_info"))
                    .control(button),
                "cloud",
            )?
        },
    ];
    out.push(section_title(ui, theme, tr("cloud.section_service"))?);
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
            tr("cloud.debounce"),
            tr("cloud.debounce_info"),
            100,
            10000,
            100,
        ),
        "min_interval_ms" => (
            tr("cloud.min_interval"),
            tr("cloud.min_interval_info"),
            1000,
            60000,
            1000,
        ),
        _ => unreachable!("unknown prediction timing"),
    };
    let label = tr_args("cloud.milliseconds", &[("n", &value)]);
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
