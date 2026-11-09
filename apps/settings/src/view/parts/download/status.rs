//! 后台下载任务里某一项的状态，翻成 bamboo-slip 下载项要的 [`DownloadState`]（进度那行字、失败原因按界面语言写好）。

use bamboo_slip::DownloadState;

use super::megabytes;
use crate::i18n::{tr, tr_args};
use crate::jobs::download::{Item, Stage};
use crate::view::Page;

pub struct DownloadStatus {
    stage: Option<Stage>,

    /// 下着时是进度那行字，失败时是原因；别的时候空着。
    text: String,
}

impl DownloadStatus {
    pub fn of(page: &Page, item: &Item) -> Self {
        let download = &page.settings.download;
        let stage = download.stage_of(item);
        let text = match &stage {
            Some(Stage::Downloading { done, total }) if *total > 0 => tr_args(
                "experimental.downloading",
                &[("done", &megabytes(*done)), ("total", &megabytes(*total))],
            ),
            Some(Stage::Downloading { .. }) => tr("experimental.connecting").to_owned(),
            Some(Stage::Queued) => tr("download.queued").to_owned(),
            Some(Stage::Failed(error)) => {
                tr_args("experimental.download_failed", &[("error", error)])
            }
            _ => String::new(),
        };
        Self { stage, text }
    }

    /// `installed` 是装着时有没有新版（`Some(true)` 可更新），没装为 `None`。
    pub fn state(&self, installed: Option<bool>) -> DownloadState<'_> {
        match &self.stage {
            Some(Stage::Downloading { done, total }) => DownloadState::Downloading {
                fraction: if *total > 0 {
                    *done as f32 / *total as f32
                } else {
                    0.0
                },
                text: &self.text,
            },
            // 排队的也走「下着」那一档：有取消按钮，进度条空着
            Some(Stage::Queued) => DownloadState::Downloading {
                fraction: 0.0,
                text: &self.text,
            },
            Some(Stage::Installing) => DownloadState::Installing,
            _ if installed.is_some() => DownloadState::Installed {
                outdated: installed == Some(true),
            },
            Some(Stage::Failed(_)) => DownloadState::Failed { reason: &self.text },
            Some(Stage::Done) | None => DownloadState::Available,
        }
    }
}
