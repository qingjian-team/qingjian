//! 设备页：这台的名字、已配对的设备、附近的设备和配对。整个面板换成这一页，搜索框不在树里。

use bamboo_slip::taffy::{AlignItems, FlexDirection, JustifyContent, NodeId, Style};
use bamboo_slip::{Button, RenderError, ScrollArea, TextField, Ui, Visual};
use qingjian_clipboard_sync::PeerStatus;
use qingjian_clipboard_sync::default_name::short_id;

use super::{Context, gap, text_node};
use crate::app::{Message, now_millis};
use crate::i18n::tr;
use crate::link::Pairing;

fn heading(ui: &mut Ui<Message>, context: &Context, text: &str) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    text_node(
        ui,
        text.to_owned(),
        theme.text(theme.font.size, 600, theme.colors.text),
        Style::default(),
    )
}

fn note(ui: &mut Ui<Message>, context: &Context, text: &str) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    text_node(
        ui,
        text.to_owned(),
        theme.text(theme.small, 400, theme.colors.secondary),
        Style::default(),
    )
}

fn row(
    ui: &mut Ui<Message>,
    context: &Context,
    children: &[NodeId],
    spread: bool,
) -> Result<NodeId, RenderError> {
    let theme = &context.theme;
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            align_items: AlignItems::CENTER,
            justify_content: if spread {
                JustifyContent::SPACE_BETWEEN
            } else {
                JustifyContent::START
            },
            gap: gap(theme, 8.0),
            ..Style::default()
        },
        Visual::Group,
        children,
    )
}

/// 一台设备：名字、小字说明、右边的按钮。
fn device_row(
    ui: &mut Ui<Message>,
    context: &Context,
    title: String,
    detail: String,
    button: NodeId,
) -> Result<NodeId, RenderError> {
    let name = heading(ui, context, &title)?;
    let detail = note(ui, context, &detail)?;
    let labels = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..Style::default()
        },
        Visual::Group,
        &[name, detail],
    )?;
    row(ui, context, &[labels, button], true)
}

/// 设备的状态；连不上时带上上次同步的时间，看得出是刚断的还是早就没连上。
fn status_text(status: PeerStatus, last_seen: Option<i64>) -> String {
    match status {
        PeerStatus::Connected => tr("devices.connected").to_owned(),
        PeerStatus::Connecting => tr("devices.connecting").to_owned(),
        PeerStatus::Unreachable => match last_seen {
            Some(seen) => tr("devices.unreachable_since").replace("{time}", &ago(seen)),
            None => tr("devices.unreachable").to_owned(),
        },
    }
}

fn ago(seen: i64) -> String {
    let minutes = (now_millis() - seen).max(0) / 60_000;
    match minutes {
        0 => tr("panel.just_now").to_owned(),
        1..=59 => tr("panel.minutes_ago").replace("{count}", &minutes.to_string()),
        _ => tr("panel.hours_ago").replace("{count}", &(minutes / 60).to_string()),
    }
}

/// 名字一样的设备补上 id 尾巴才分得清。
fn display_name(name: &str, device_id: &str, all: &[&str]) -> String {
    if all.iter().filter(|other| **other == name).count() > 1 {
        format!("{name} #{}", short_id(device_id))
    } else {
        name.to_owned()
    }
}

fn pairing(ui: &mut Ui<Message>, context: &Context) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let mut out = Vec::new();
    match &context.app.pairing {
        Pairing::Idle => {
            let show = Button::new(tr("devices.show_code")).build(
                ui,
                theme,
                "pair-show",
                Message::PairShowCode,
            )?;
            let enter = Button::new(tr("devices.enter_address")).build(
                ui,
                theme,
                "pair-enter",
                Message::PairEnter(String::new()),
            )?;
            out.push(row(ui, context, &[show, enter], false)?);
        }
        Pairing::Showing(code) => {
            out.push(note(ui, context, tr("devices.code_hint"))?);
            let digits = text_node(
                ui,
                code.clone(),
                theme.text(theme.font.size * 2.0, 600, theme.colors.accent),
                Style::default(),
            )?;
            let cancel = Button::new(tr("devices.cancel")).build(
                ui,
                theme,
                "pair-cancel",
                Message::PairCancel,
            )?;
            out.push(row(ui, context, &[digits, cancel], true)?);
        }
        Pairing::Entering { address, code } => {
            out.push(note(ui, context, tr("devices.address"))?);
            out.push(TextField::new(address).build(
                ui,
                theme,
                "pair-address",
                Message::PairAddress,
                None,
                Some(Message::PairCancel),
            )?);
            out.push(note(ui, context, tr("devices.code"))?);
            out.push(TextField::new(code).autofocus().build(
                ui,
                theme,
                "pair-code",
                Message::PairCode,
                Some(Message::PairSubmit),
                Some(Message::PairCancel),
            )?);
            let cancel = Button::new(tr("devices.cancel")).build(
                ui,
                theme,
                "pair-cancel",
                Message::PairCancel,
            )?;
            let connect = Button::new(tr("devices.connect")).primary().build(
                ui,
                theme,
                "pair-connect",
                Message::PairSubmit,
            )?;
            out.push(row(ui, context, &[cancel, connect], false)?);
        }
    }
    if !context.app.pair_status.is_empty() {
        out.push(note(ui, context, &context.app.pair_status)?);
    }
    Ok(out)
}

pub(super) fn devices(ui: &mut Ui<Message>, context: &Context) -> Result<Vec<NodeId>, RenderError> {
    let theme = &context.theme;
    let app = context.app;
    let mut body = vec![heading(ui, context, tr("devices.this_device"))?];
    let name = TextField::new(&app.name_edit).build(
        ui,
        theme,
        "device-name",
        Message::NameChange,
        Some(Message::NameSave),
        None,
    )?;
    let save = Button::new(tr("devices.save_name")).build(
        ui,
        theme,
        "device-name-save",
        Message::NameSave,
    )?;
    body.push(row(ui, context, &[name, save], false)?);

    let service = app.link.service();
    let peers = service
        .map(|service| service.peer_states())
        .unwrap_or_default();
    let nearby = service.map(|service| service.nearby()).unwrap_or_default();
    let names: Vec<&str> = peers
        .iter()
        .map(|state| state.peer.name.as_str())
        .chain(nearby.iter().map(|device| device.name.as_str()))
        .collect();

    if !peers.is_empty() {
        body.push(heading(ui, context, tr("devices.paired"))?);
    }
    for state in &peers {
        let peer = &state.peer;
        let status = status_text(state.status, peer.last_seen);
        let button = Button::new(tr("devices.unpair")).build(
            ui,
            theme,
            ("unpair", peer.device_id.as_str()),
            Message::Unpair(peer.device_id.clone()),
        )?;
        body.push(device_row(
            ui,
            context,
            display_name(&peer.name, &peer.device_id, &names),
            status,
            button,
        )?);
    }

    body.push(heading(ui, context, tr("devices.nearby"))?);
    let strangers: Vec<_> = nearby
        .iter()
        .filter(|device| {
            !peers
                .iter()
                .any(|state| state.peer.device_id == device.device_id)
        })
        .collect();
    if strangers.is_empty() {
        body.push(note(ui, context, tr("devices.none_nearby"))?);
    }
    for device in strangers {
        let button = Button::new(tr("devices.pair")).build(
            ui,
            theme,
            ("pair", device.device_id.as_str()),
            Message::PairEnter(device.address.to_string()),
        )?;
        body.push(device_row(
            ui,
            context,
            display_name(&device.name, &device.device_id, &names),
            device.model.clone(),
            button,
        )?);
    }
    body.extend(pairing(ui, context)?);

    let content = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: gap(theme, 8.0),
            ..Style::default()
        },
        Visual::Group,
        &body,
    )?;
    let area = ScrollArea::new(Style {
        flex_grow: 1.0,
        flex_basis: bamboo_slip::taffy::Dimension::length(0.0),
        ..Style::default()
    })
    .build(ui, theme, "devices", content)?;
    let title = heading(ui, context, tr("devices.title"))?;
    let back =
        Button::new(tr("devices.back")).build(ui, theme, "devices-back", Message::CloseDevices)?;
    Ok(vec![row(ui, context, &[title, back], true)?, area])
}

#[cfg(test)]
mod tests {
    use super::display_name;

    #[test]
    fn only_duplicates_get_a_suffix() {
        assert_eq!(display_name("灯", "abcd1234", &["灯", "爱音"]), "灯");
        assert_eq!(display_name("灯", "abcd1234", &["灯", "灯"]), "灯 #1234");
    }
}
