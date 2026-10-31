//! 「实验功能」页：可以下载的本地模型，一个一张卡片。没下载给「下载」，下着给进度条与「取消」，下好了才有开关与「删除」；
//! 知意、通变两个只开一个，开了这个就关掉另一个，都关着就是不用模型（只按词库）。知意开着时下面出三档。

use bamboo_slip::taffy::{AlignItems, FlexDirection, NodeId, Style};
use bamboo_slip::{
    Button, ProgressBar, RenderError, SettingRow, Switch, Ui, Visual, group, paragraph,
    section_title,
};
use qingjian_platform::{LocalModel, ZhiyiTier};

use crate::i18n::{tr, tr_args};
use crate::jobs::model_download::{Pack, Stage};
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::controls::choice;

pub fn experimental(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let mut out = Vec::new();
    if let Some(("download", error)) = &page.settings.notice {
        out.push(paragraph(ui, theme, error, None)?);
    }
    for pack in Pack::ALL {
        out.push(section_title(ui, theme, title(pack.model))?);
        let rows = card(ui, page, pack)?;
        out.push(group(ui, theme, &rows)?);
    }
    if crate::support::Feature::OfflineTranslation.here() {
        out.extend(super::offline_translation::card(ui, page)?);
    }
    Ok(out)
}

fn title(model: LocalModel) -> &'static str {
    match model {
        LocalModel::Zhiyi => tr("experimental.zhiyi_recommended"),
        _ => model.label(),
    }
}

fn description(model: LocalModel) -> &'static str {
    match model {
        LocalModel::Zhiyi => tr("experimental.zhiyi_description"),
        LocalModel::Tongbian => tr("experimental.tongbian_description"),
    }
}

fn card(ui: &mut Ui<Message>, page: &Page, pack: Pack) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let settings = page.settings;
    let model = &settings.config.model;
    let detail = format!("{} · {}", pack.size, description(pack.model));
    let stage = settings
        .download
        .stage()
        .filter(|(downloading, _)| *downloading == pack.model)
        .map(|(_, stage)| stage);
    let busy = settings.download.busy();
    let mut rows = Vec::new();
    let mut row = SettingRow::new(pack.model.label()).detail(&detail);
    match &stage {
        Some(Stage::Downloading { done, total }) => {
            let cancel = Button::new(tr("experimental.cancel")).build(
                ui,
                theme,
                ("cancel", pack.tag),
                Message::CancelDownload,
            )?;
            rows.push(row.control(cancel).build(ui, theme)?);
            rows.push(progress(ui, page, *done, *total)?);
        }
        Some(Stage::Installing) => {
            let waiting = Button::new(tr("experimental.installing"))
                .disabled(true)
                .build(ui, theme, ("installing", pack.tag), Message::CancelDownload)?;
            rows.push(row.control(waiting).build(ui, theme)?);
        }
        _ if pack.installed() => {
            let on = model.enabled && model.name == pack.model;
            let switch = Switch::new(on).build(
                ui,
                theme,
                ("use", pack.tag),
                Message::UseModel((!on).then_some(pack.model)),
            )?;
            let armed = settings.remove_armed == Some(pack.model);
            let mut remove = Button::new(if armed {
                tr("experimental.delete_confirm")
            } else {
                tr("experimental.delete")
            });
            if armed {
                remove = remove.primary();
            }
            let remove = remove.build(
                ui,
                theme,
                ("remove", pack.tag),
                Message::RemoveModel(pack.model),
            )?;
            let controls = ui.scene().node(
                Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::CENTER,
                    gap: page.gap(12.0),
                    ..Style::default()
                },
                Visual::Group,
                &[remove, switch],
            )?;
            rows.push(row.control(controls).build(ui, theme)?);
            if on && pack.model == LocalModel::Zhiyi {
                let tiers = choice(
                    ui,
                    page,
                    "zhiyi-tier",
                    &ZhiyiTier::ALL,
                    model.zhiyi_tier,
                    ZhiyiTier::label,
                    |tier| Message::set("model", "zhiyi_tier", tier.key()),
                )?;
                rows.push(
                    SettingRow::new(tr("experimental.tier"))
                        .control(tiers)
                        .build(ui, theme)?,
                );
            }
        }
        _ => {
            let failed = match &stage {
                Some(Stage::Failed(error)) => Some(error.as_str()),
                _ => None,
            };
            let label = if failed.is_some() {
                tr("experimental.retry")
            } else {
                tr("experimental.download")
            };
            let download = Button::new(label).disabled(busy).build(
                ui,
                theme,
                ("download", pack.tag),
                Message::DownloadModel(pack.model),
            )?;
            if let Some(error) = failed {
                row = row.below(paragraph(
                    ui,
                    theme,
                    &tr_args("experimental.download_failed", &[("error", &error)]),
                    None,
                )?);
            }
            rows.push(row.control(download).build(ui, theme)?);
        }
    }
    Ok(rows)
}

/// 进度条一行：上面写下了多少，下面一条进度条。
fn progress(
    ui: &mut Ui<Message>,
    page: &Page,
    done: u64,
    total: u64,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let fraction = if total > 0 {
        done as f32 / total as f32
    } else {
        0.0
    };
    let text = if total > 0 {
        tr_args(
            "experimental.downloading",
            &[("done", &megabytes(done)), ("total", &megabytes(total))],
        )
    } else {
        tr("experimental.connecting").to_owned()
    };
    let bar = ProgressBar::new(fraction).build(ui, theme)?;
    SettingRow::new(&text)
        .control(bar)
        .stacked()
        .build(ui, theme)
}

fn megabytes(bytes: u64) -> String {
    format!("{:.0} MB", bytes as f64 / 1_000_000.0)
}
