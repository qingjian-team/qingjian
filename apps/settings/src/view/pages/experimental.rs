//! 「实验功能」页：可以下载的本地模型，一个一张卡片。没下载给「下载」，下着给进度条与「取消」，下好了才有开关与「删除」；
//! 通变（随包）、知微、知意三个只开一个，开了这个就关掉另一个，都关着就是通变。知意开着时下面出三档。

use bamboo_slip::taffy::{AlignItems, FlexDirection, NodeId, Style};
use bamboo_slip::{
    Button, ProgressBar, RenderError, SettingRow, Switch, Ui, Visual, group, paragraph,
    section_title,
};
use qingjian_platform::{LocalModel, ZhiyiTier};

use crate::jobs::model_download::{Pack, Stage};
use crate::message::Message;
use crate::view::Page;
use crate::view::parts::controls::choice;

pub fn experimental(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let mut out = Vec::new();
    if !page.settings.config.model.enabled {
        out.push(paragraph(
            ui,
            theme,
            "先到「候选」页打开「本地整句模型」",
            None,
        )?);
    }
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
        LocalModel::Zhiyi => "知意（推荐）",
        _ => model.label(),
    }
}

fn description(model: LocalModel) -> &'static str {
    match model {
        LocalModel::Zhiyi => "按前文挑首选",
        LocalModel::Zhiwei => "按前文排整句",
        LocalModel::Tongbian => "随安装包",
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
            let cancel = Button::new("取消").build(
                ui,
                theme,
                ("cancel", pack.tag),
                Message::CancelDownload,
            )?;
            rows.push(row.control(cancel).build(ui, theme)?);
            rows.push(progress(ui, page, *done, *total)?);
        }
        Some(Stage::Installing) => {
            let waiting = Button::new("正在安装…").disabled(true).build(
                ui,
                theme,
                ("installing", pack.tag),
                Message::CancelDownload,
            )?;
            rows.push(row.control(waiting).build(ui, theme)?);
        }
        _ if pack.installed() => {
            let on = model.name == pack.model;
            let next = if on { LocalModel::Tongbian } else { pack.model };
            let switch = Switch::new(on).build(
                ui,
                theme,
                ("use", pack.tag),
                Message::set("model", "name", next.key()),
            )?;
            let armed = settings.remove_armed == Some(pack.model);
            let mut remove = Button::new(if armed {
                "再点一次删除"
            } else {
                "删除"
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
                    align_items: Some(AlignItems::CENTER),
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
                rows.push(SettingRow::new("档位").control(tiers).build(ui, theme)?);
            }
        }
        _ => {
            let failed = match &stage {
                Some(Stage::Failed(error)) => Some(error.as_str()),
                _ => None,
            };
            let label = if failed.is_some() { "重试" } else { "下载" };
            let download = Button::new(label).disabled(busy).build(
                ui,
                theme,
                ("download", pack.tag),
                Message::DownloadModel(pack.model),
            )?;
            if let Some(error) = failed {
                row = row.below(paragraph(ui, theme, &format!("下载失败：{error}"), None)?);
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
        format!("正在下载 {} / {}", megabytes(done), megabytes(total))
    } else {
        "正在连接…".to_owned()
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
