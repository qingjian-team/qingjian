//! 一行设置：左边标题（可带小标签、一行说明、再一层小字、一个悬停说明的 ⓘ），右边控件，控件下面还能挂附加项（勾选框、链接）。
//! 控件很宽的（主题卡片）用 [`SettingRow::stacked`] 放到标题下面。

use std::time::Duration;

use taffy::{AlignItems, JustifyContent, LengthPercentageAuto, NodeId, Position, Rect, Style};

use super::WidgetTheme;
use super::info::info_with_link;
use super::parts::{column, label, padding, row, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Key, Ui};

/// 醒目标出（[`Ui::spotlighted`]）撤掉后底色淡出的时长。
const SPOTLIGHT_FADE: Duration = Duration::from_millis(700);

pub struct SettingRow<'a> {
    title: &'a str,

    detail: Option<&'a str>,

    caption: Option<NodeId>,

    badge: Option<&'a str>,

    info: Option<&'a str>,

    control: Option<NodeId>,

    below: Vec<NodeId>,

    /// 横跨整行、排在标题与控件下面的一块（下载进度）；与这一行是一体，中间不分隔。
    footer: Option<NodeId>,

    stacked: bool,

    /// 使用方补的搜索同义词。
    keywords: &'a [&'a str],
}

impl<'a> SettingRow<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            detail: None,
            caption: None,
            badge: None,
            info: None,
            control: None,
            below: Vec::new(),
            footer: None,
            stacked: false,
            keywords: &[],
        }
    }

    /// 标题下面一行次要色的说明；能不写就不写。
    pub fn detail(self, detail: &'a str) -> Self {
        Self {
            detail: Some(detail),
            ..self
        }
    }

    /// 说明下面再一层：使用方拼好的一行小字（大小、许可、能点的作者名这类附带信息），与说明分开一层。
    pub fn caption(self, node: NodeId) -> Self {
        Self {
            caption: Some(node),
            ..self
        }
    }

    /// 标题与控件下面横跨整行的一块（下载进度这类）；只用于标题在左、控件在右的行。
    pub fn footer(self, node: NodeId) -> Self {
        Self {
            footer: Some(node),
            ..self
        }
    }

    /// 标题右边的小标签（「第三方」这类），比标题小一号、垫一块底。
    pub fn badge(self, text: &'a str) -> Self {
        Self {
            badge: Some(text),
            ..self
        }
    }

    /// 标题右边的 ⓘ，悬停时浮出这段说明。
    pub fn info(self, text: &'a str) -> Self {
        Self {
            info: Some(text),
            ..self
        }
    }

    pub fn control(self, node: NodeId) -> Self {
        Self {
            control: Some(node),
            ..self
        }
    }

    /// 挂在控件下面、右对齐。
    pub fn below(mut self, node: NodeId) -> Self {
        self.below.push(node);
        self
    }

    /// 搜索时也按这些词找到这一行（标题、ⓘ、控件上的字之外的同义词）。
    pub fn keywords(self, keywords: &'a [&'a str]) -> Self {
        Self { keywords, ..self }
    }

    /// 控件放到标题下面、左对齐占满一行。
    pub fn stacked(self) -> Self {
        Self {
            stacked: true,
            ..self
        }
    }

    /// 行登记成可搜索项（标题与 ⓘ 全文），key 由标题派生；应用醒目标出它时铺一层底色。
    pub fn build<M>(self, ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
        let key = ui.key(("row", self.title));
        let owned: Vec<NodeId> = self
            .control
            .iter()
            .chain(&self.caption)
            .chain(&self.below)
            .chain(&self.footer)
            .copied()
            .collect();
        ui.searchable(key, (self.title, self.info), &owned, self.keywords);
        let spot = spotlight(ui, theme, key, (-2.0, 8.0))?;
        let colors = &theme.colors;
        let mut title = label(ui, theme, self.title, theme.font.size, 500, colors.text)?;
        if let Some(text) = self.info {
            let link = ui.info_link(self.title);
            let mark = info_with_link(ui, theme, ("info", self.title), text, link)?;
            title = ui.scene().node(
                Style {
                    align_items: AlignItems::CENTER,
                    ..row(theme, 6.0)
                },
                Visual::Group,
                &[title, mark],
            )?;
        }
        if let Some(text) = self.badge {
            let badge = badge(ui, theme, text)?;
            title = ui.scene().node(
                Style {
                    align_items: AlignItems::CENTER,
                    ..row(theme, 8.0)
                },
                Visual::Group,
                &[title, badge],
            )?;
        }
        let mut texts = vec![title];
        if let Some(detail) = self.detail {
            // 说明长了折行，不把右边的控件挤出去
            texts.push(ui.scene().node(
                Style::default(),
                Visual::Text {
                    text: detail.to_owned(),
                    style: theme.text(theme.small, 400, colors.secondary).wrapped(),
                },
                &[],
            )?);
        }
        if let Some(caption) = self.caption {
            texts.push(caption);
        }
        let heading = ui.scene().node(
            Style {
                flex_shrink: 1.0,
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..column(theme, 2.0)
            },
            Visual::Group,
            &texts,
        )?;
        let node = if self.stacked {
            let mut children = vec![spot, heading];
            children.extend(self.control);
            children.extend(self.below);
            ui.scene().node(
                Style {
                    padding: padding(theme, 12.0, 0.0),
                    ..column(theme, 10.0)
                },
                Visual::Group,
                &children,
            )?
        } else {
            self.inline(ui, theme, spot, heading)?
        };
        ui.interact(node, key, Interact::new());
        Ok(node)
    }

    /// 标题在左、控件在右的一行。
    fn inline<M>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        spot: NodeId,
        heading: NodeId,
    ) -> Result<NodeId, RenderError> {
        let footer = self.footer;
        let mut trailing: Vec<NodeId> = self.control.into_iter().collect();
        trailing.extend(self.below);
        let trailing = ui.scene().node(
            Style {
                align_items: AlignItems::FLEX_END,
                flex_shrink: 0.0,
                ..column(theme, 8.0)
            },
            Visual::Group,
            &trailing,
        )?;
        let min_height = taffy::Size {
            width: taffy::LengthPercentageAuto::auto(),
            height: taffy::LengthPercentageAuto::length(theme.px(theme.control_height)),
        };
        if let Some(footer) = footer {
            let line = ui.scene().node(
                Style {
                    justify_content: JustifyContent::SPACE_BETWEEN,
                    min_size: min_height,
                    ..row(theme, 16.0)
                },
                Visual::Group,
                &[heading, trailing],
            )?;
            return ui.scene().node(
                Style {
                    padding: padding(theme, 10.0, 0.0),
                    ..column(theme, 10.0)
                },
                Visual::Group,
                &[spot, line, footer],
            );
        }
        ui.scene().node(
            Style {
                padding: padding(theme, 10.0, 0.0),
                justify_content: JustifyContent::SPACE_BETWEEN,
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::auto(),
                    height: taffy::LengthPercentageAuto::length(
                        theme.px(theme.control_height + 20.0),
                    ),
                },
                ..row(theme, 16.0)
            },
            Visual::Group,
            &[spot, heading, trailing],
        )
    }
}

/// 垫在下面的底色，上下、左右各比盒子出头 `outset` 点（负数为缩进）；不醒目标出时透明（照样建，撤掉时才有淡出可播）。
pub(super) fn spotlight<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    key: Key,
    (vertical, horizontal): (f32, f32),
) -> Result<NodeId, RenderError> {
    let color = theme.colors.selected;
    let color = if ui.spotlighted(key) {
        color
    } else {
        color.transparent()
    };
    let (side, edge) = (
        LengthPercentageAuto::length(-theme.px(horizontal)),
        LengthPercentageAuto::length(-theme.px(vertical)),
    );
    let node = ui.scene().node(
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: side,
                right: side,
                top: edge,
                bottom: edge,
            },
            ..Style::default()
        },
        solid(Some(color), None, theme.px(theme.radius)),
        &[],
    )?;
    ui.fade(node, key.child("spot"), SPOTLIGHT_FADE, Easing::Ease);
    Ok(node)
}

/// 标题旁的小标签：小一号的次要色字，垫一块控件槽色的圆角底。
fn badge<M>(ui: &mut Ui<M>, theme: &WidgetTheme, text: &str) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
    let text = label(ui, theme, text, theme.small - 1.0, 500, colors.secondary)?;
    ui.scene().node(
        Style {
            padding: padding(theme, 1.0, 6.0),
            flex_shrink: 0.0,
            ..row(theme, 0.0)
        },
        solid(Some(colors.well), None, theme.px(theme.radius) / 2.0),
        &[text],
    )
}
