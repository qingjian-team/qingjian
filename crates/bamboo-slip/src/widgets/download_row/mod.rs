//! 一个能下载的东西占的一行：按 [`DownloadState`] 挑右边的按钮；下着的时候这一行下面横跨整行接一块进度（写下了多少、一条进度条），
//! 与这一行是一体、中间不分隔，一张卡片里有好几项时也分得清是谁的。
//!
//! | 状态 | 右边 | 说明下面那层 |
//! | --- | --- | --- |
//! | 没装 | 「下载」，别的正在下（[`DownloadRow::busy`]）时灰掉 | 使用方给的（作者、大小…） |
//! | 下载中 | 「取消」；行下面横跨整行一块进度 | 使用方给的 |
//! | 正在安装 | 灰的「正在安装」 | 使用方给的 |
//! | 下载失败 | 「重试」，下面写原因 | 使用方给的 |
//! | 装好了 | 「删除」（点两下，第一下由使用方记成 [`DownloadRow::armed`]）、有新版再加「更新」，再加使用方的控件（开关） | 使用方给的 |
//! | 用不了（[`DownloadRow::blocked`]） | 灰着的按钮写为什么 | 使用方给的 |

mod labels;
mod state;
#[cfg(test)]
mod tests;

pub use labels::DownloadLabels;
pub use state::DownloadState;

use std::hash::Hash;

use taffy::{NodeId, Style};

use super::parts::{column, row};
use super::{Button, ProgressBar, SettingRow, WidgetTheme, paragraph};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

/// 说明下面那层，由使用方建。
type Caption<'a, M> = Box<dyn FnOnce(&mut Ui<M>) -> Result<NodeId, RenderError> + 'a>;

pub struct DownloadRow<'a, M> {
    title: &'a str,

    state: DownloadState<'a>,

    labels: DownloadLabels<'a>,

    detail: Option<&'a str>,

    badge: Option<&'a str>,

    caption: Option<Caption<'a, M>>,

    /// 别的正在下：「下载」「更新」「重试」「删除」都灰掉（一次只下一个）。
    busy: bool,

    blocked: bool,

    armed: bool,

    /// 装好后排在按钮最右边的控件。
    extra: Option<NodeId>,
}

impl<'a, M: Clone + 'static> DownloadRow<'a, M> {
    pub fn new(title: &'a str, state: DownloadState<'a>, labels: DownloadLabels<'a>) -> Self {
        Self {
            title,
            state,
            labels,
            detail: None,
            badge: None,
            caption: None,
            busy: false,
            blocked: false,
            armed: false,
            extra: None,
        }
    }

    /// 标题下的一行说明；空串当没有。
    pub fn detail(self, detail: &'a str) -> Self {
        Self {
            detail: (!detail.is_empty()).then_some(detail),
            ..self
        }
    }

    /// 标题旁的小标签（「第三方」）。
    pub fn badge(self, badge: Option<&'a str>) -> Self {
        Self { badge, ..self }
    }

    /// 说明下面那层（一般是 [`super::fields`]）。
    pub fn caption(
        self,
        build: impl FnOnce(&mut Ui<M>) -> Result<NodeId, RenderError> + 'a,
    ) -> Self {
        Self {
            caption: Some(Box::new(build)),
            ..self
        }
    }

    pub fn busy(self, busy: bool) -> Self {
        Self { busy, ..self }
    }

    /// 这一版程序用不了它（格式更新）：不给下载与更新，按钮灰着写 [`DownloadLabels::blocked`]。
    pub fn blocked(self, blocked: bool) -> Self {
        Self { blocked, ..self }
    }

    /// 已经点过一次「删除」，再点才真删。
    pub fn armed(self, armed: bool) -> Self {
        Self { armed, ..self }
    }

    pub fn extra(self, extra: Option<NodeId>) -> Self {
        Self { extra, ..self }
    }

    /// `on_download` 也是「更新」「重试」发的；`on_remove` 每次点「删除」都发，第一下由使用方记成待确认。
    pub fn build(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash + Copy,
        (on_download, on_cancel, on_remove): (M, M, M),
    ) -> Result<NodeId, RenderError> {
        let labels = self.labels;
        let mut line = SettingRow::new(self.title);
        if let Some(detail) = self.detail {
            line = line.detail(detail);
        }
        if let Some(badge) = self.badge {
            line = line.badge(badge);
        }
        if let Some(caption) = self.caption {
            line = line.caption(caption(ui)?);
        }
        let busy = self.busy;
        let start = |label| {
            if self.blocked {
                Button::new(labels.blocked).disabled(true)
            } else {
                Button::new(label).disabled(busy)
            }
        };
        let control = match self.state {
            DownloadState::Downloading { fraction, text } => {
                line = line.footer(progress(ui, theme, fraction, text)?);
                Button::new(labels.cancel).build(ui, theme, ("cancel", local), on_cancel)?
            }
            DownloadState::Installing => Button::new(labels.installing).disabled(true).build(
                ui,
                theme,
                ("installing", local),
                on_cancel,
            )?,
            DownloadState::Installed { outdated } => {
                let mut remove = Button::new(if self.armed {
                    labels.delete_confirm
                } else {
                    labels.delete
                });
                if self.armed {
                    remove = remove.primary();
                }
                let mut controls =
                    vec![
                        remove
                            .disabled(busy)
                            .build(ui, theme, ("remove", local), on_remove)?,
                    ];
                if outdated {
                    controls.push(start(labels.update).build(
                        ui,
                        theme,
                        ("start", local),
                        on_download,
                    )?);
                }
                controls.extend(self.extra);
                ui.scene()
                    .node(row(theme, 12.0), Visual::Group, &controls)?
            }
            DownloadState::Failed { reason } => {
                line = line.below(paragraph(ui, theme, reason, None)?);
                start(labels.retry).build(ui, theme, ("start", local), on_download)?
            }
            DownloadState::Available => {
                start(labels.download).build(ui, theme, ("start", local), on_download)?
            }
        };
        line.control(control).build(ui, theme)
    }
}

/// 行下面横跨整行的进度：上面一行写下了多少，下面一条进度条。
fn progress<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    fraction: f32,
    text: &str,
) -> Result<NodeId, RenderError> {
    let text = ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: theme.text(theme.font.size, 500, theme.colors.text),
        },
        &[],
    )?;
    let bar = ProgressBar::new(fraction).build(ui, theme)?;
    ui.scene()
        .node(column(theme, 10.0), Visual::Group, &[text, bar])
}
