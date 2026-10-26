//! 「自定义短语」页：每条一行（开关、输入码、权重、内容的开头、编辑），末尾新增；macOS 另有「也用系统的文本替换」。
//! 新增、编辑在对话框里改，保存时整张表校验。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto,
    NodeId, Rect, Size, Style,
};
use bamboo_slip::{
    Button, Checkbox, RenderError, SettingRow, TextField, Ui, Visual, dialog, group, paragraph,
    section_title,
};

use crate::message::{FieldId, Message};
use crate::support::Feature;
use crate::view::Page;
use crate::view::parts::actions::with_notice;
use crate::view::parts::controls::toggle_info;

/// 内容在列表里最多显示几个字。
const PREVIEW: usize = 24;

/// 对话框宽度（点）。
const DIALOG_WIDTH: f32 = 440.0;

pub fn phrases(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let mut out = Vec::new();

    let mut rows = Vec::new();
    for (index, phrase) in config.custom_phrases.iter().enumerate() {
        rows.push(phrase_row(ui, page, index, phrase)?);
    }
    let add = Button::new("新增…").build(ui, theme, "add-phrase", Message::OpenPhrase(None))?;
    rows.push(with_notice(
        ui,
        page,
        SettingRow::new("新增短语")
            .info("输入码敲全后，短语排在普通候选前面，空格或数字键上屏；同一个输入码有几条时权重大的在前")
            .control(add),
        "phrases",
    )?);
    out.push(section_title(ui, theme, "短语")?);
    out.push(group(ui, theme, &rows)?);

    if Feature::SystemTextReplacements.here() {
        let replacements = toggle_info(
            ui,
            page,
            "也用系统的文本替换",
            Some("系统「文本替换」里的条目也能打"),
            ("general", "system_text_replacements"),
            config.general.system_text_replacements,
            false,
        )?;
        out.push(section_title(ui, theme, "系统")?);
        out.push(group(ui, theme, &[replacements])?);
    }

    if page.settings.phrase.is_some() {
        editor(ui, page)?;
    }
    Ok(out)
}

fn phrase_row(
    ui: &mut Ui<Message>,
    page: &Page,
    index: usize,
    phrase: &qingjian_core::CustomPhrase,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let toggle = Checkbox::new(phrase.enabled, &phrase.code).build(
        ui,
        theme,
        ("phrase-on", index),
        Message::TogglePhrase(index),
    )?;
    let weight = small(ui, page, &format!("权重 {}", phrase.weight))?;
    let leading = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            gap: page.gap(12.0),
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Group,
        &[toggle, weight],
    )?;
    let preview = small(ui, page, &preview(&phrase.text))?;
    let edit = Button::new("编辑").build(
        ui,
        theme,
        ("phrase-edit", index),
        Message::OpenPhrase(Some(index)),
    )?;
    let trailing = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            gap: page.gap(12.0),
            ..Style::default()
        },
        Visual::Group,
        &[preview, edit],
    )?;
    let space = LengthPercentage::length(page.px(10.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            gap: page.gap(16.0),
            min_size: Size {
                width: LengthPercentageAuto::auto(),
                height: LengthPercentageAuto::length(page.px(theme.control_height + 20.0)),
            },
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[leading, trailing],
    )
}

/// 新增 / 编辑的对话框。
fn editor(ui: &mut Ui<Message>, page: &Page) -> Result<(), RenderError> {
    let theme = &page.theme;
    let Some(editor) = &page.settings.phrase else {
        return Ok(());
    };
    let fields = &page.settings.fields;
    let (Some(code), Some(weight), Some(text)) = (
        fields.get(&FieldId::PhraseCode),
        fields.get(&FieldId::PhraseWeight),
        fields.get(&FieldId::PhraseText),
    ) else {
        return Ok(());
    };
    let title = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: if editor.index.is_some() {
                "编辑短语"
            } else {
                "新增短语"
            }
            .to_owned(),
            style: theme.text(theme.font.size + 2.0, 600, theme.colors.text),
        },
        &[],
    )?;
    let code = TextField::new(code)
        .placeholder("小写字母，如 dz")
        .width(200.0)
        .build(
            ui,
            theme,
            "phrase-code",
            |edit| Message::Edit(FieldId::PhraseCode, edit),
            None,
            Some(Message::ClosePhrase),
        )?;
    let code = SettingRow::new("输入码").control(code).build(ui, theme)?;
    let weight = TextField::new(weight)
        .placeholder("1–999")
        .width(80.0)
        .build(
            ui,
            theme,
            "phrase-weight",
            |edit| Message::Edit(FieldId::PhraseWeight, edit),
            None,
            Some(Message::ClosePhrase),
        )?;
    let weight = SettingRow::new("权重")
        .info("1–999，同一个输入码下权重大的排在前面，可以相同；相同的按列表里的先后")
        .control(weight)
        .build(ui, theme)?;
    let content = TextField::new(text)
        .placeholder("原样上屏的文字，可以换行")
        .width(DIALOG_WIDTH - 40.0)
        .rows(4)
        .build(
            ui,
            theme,
            "phrase-text",
            |edit| Message::Edit(FieldId::PhraseText, edit),
            None,
            Some(Message::ClosePhrase),
        )?;
    let mut children = vec![title, code, weight, content];
    if let Some(error) = &editor.error {
        children.push(paragraph(ui, theme, error, None)?);
    }
    let mut buttons = Vec::new();
    if editor.index.is_some() {
        let delete = Button::new(if editor.confirm_delete {
            "再点一次删除"
        } else {
            "删除"
        })
        .build(ui, theme, "phrase-delete", Message::DeletePhrase)?;
        buttons.push(delete);
    }
    let spacer = ui.scene().node(
        Style {
            flex_grow: 1.0,
            size: Size {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[],
    )?;
    buttons.push(spacer);
    buttons.push(Button::new("取消").build(ui, theme, "phrase-cancel", Message::ClosePhrase)?);
    buttons.push(Button::new("保存").primary().build(
        ui,
        theme,
        "phrase-save",
        Message::SavePhrase,
    )?);
    children.push(ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            gap: page.gap(8.0),
            margin: Rect {
                top: LengthPercentageAuto::length(page.px(4.0)),
                ..Rect::zero()
            },
            ..Style::default()
        },
        Visual::Group,
        &buttons,
    )?);
    dialog(ui, theme, "phrase-dialog", DIALOG_WIDTH, &children)
}

fn small(ui: &mut Ui<Message>, page: &Page, text: &str) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: theme.text(theme.small, 400, theme.colors.secondary),
        },
        &[],
    )
}

/// 列表里的内容：只取第一行，太长截断。
fn preview(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    let more = text.contains('\n') || first.chars().count() > PREVIEW;
    let mut shown: String = first.chars().take(PREVIEW).collect();
    if more {
        shown.push('…');
    }
    shown
}
