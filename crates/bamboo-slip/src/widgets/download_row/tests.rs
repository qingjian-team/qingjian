use std::cell::Cell;

use taffy::NodeId;

use super::*;
use crate::fonts::FontLibrary;
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key};
use crate::widgets::{FieldValue, WidgetColors, fields};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Download,
    Cancel,
    Remove,
}

const LABELS: DownloadLabels<'static> = DownloadLabels {
    download: "Download",
    cancel: "Cancel",
    installing: "Installing",
    retry: "Retry",
    delete: "Delete",
    delete_confirm: "Click again",
    update: "Update",
    blocked: "Update the app first",
};

/// 建一行、布局、交给交互层；返回说明下面那层有没有被建。
fn frame(interaction: &mut Interaction<Message>, state: DownloadState, busy: bool) -> bool {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let built = Cell::new(false);
    let mut ui = Ui::new(interaction);
    let root: NodeId = DownloadRow::new("Medicine", state, LABELS)
        .caption(|ui| {
            built.set(true);
            fields(ui, &theme, &[("Size", FieldValue::Text("0.3 MB"))])
        })
        .busy(busy)
        .build(
            &mut ui,
            &theme,
            "medicine",
            (Message::Download, Message::Cancel, Message::Remove),
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    scene.layout(root, &mut text).unwrap();
    interaction.attach(&scene, root, elements).unwrap();
    built.get()
}

fn click(interaction: &mut Interaction<Message>, name: &str) -> Vec<Message> {
    let (x, y, width, height) = interaction
        .rect(Key::ROOT.child((name, "medicine")))
        .unwrap_or_else(|| panic!("no {name} button"));
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp).messages
}

#[test]
fn installed_with_update_offers_delete_and_update() {
    let mut interaction = Interaction::new();
    assert!(frame(
        &mut interaction,
        DownloadState::Installed { outdated: true },
        false
    ));
    assert_eq!(click(&mut interaction, "remove"), vec![Message::Remove]);
    assert_eq!(click(&mut interaction, "start"), vec![Message::Download]);
}

#[test]
fn downloading_keeps_caption_and_offers_cancel() {
    let mut interaction = Interaction::new();
    let state = DownloadState::Downloading {
        fraction: 0.4,
        text: "3 / 8 MB",
    };
    assert!(frame(&mut interaction, state, true));
    assert_eq!(click(&mut interaction, "cancel"), vec![Message::Cancel]);
    assert!(
        interaction
            .rect(Key::ROOT.child(("start", "medicine")))
            .is_none()
    );
}

#[test]
fn another_download_running_disables_start() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, DownloadState::Available, true);
    assert!(click(&mut interaction, "start").is_empty());
    frame(&mut interaction, DownloadState::Available, false);
    assert_eq!(click(&mut interaction, "start"), vec![Message::Download]);
}
