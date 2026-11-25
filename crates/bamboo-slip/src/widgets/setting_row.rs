//! 一行设置：左边标题（可带小标签、一行说明、再一层小字、一个悬停说明的 ⓘ），右边控件，控件下面还能挂附加项（勾选框、链接）。
//! 控件很宽的（主题卡片）用 [`SettingRow::stacked`] 放到标题下面。
//! [`Look::Fluent`] 下使用方用 [`Ui::row_icons`] 给了图标的行，标题左边再画一个。

use std::sync::Arc;
use std::time::Duration;

use taffy::{
    AlignItems, Dimension, FlexWrap, JustifyContent, LengthPercentage, LengthPercentageAuto,
    NodeId, Position, Rect, Style,
};

use super::WidgetTheme;
use super::info::info_with_link;
use super::link::Link;
use super::parts::{column, label, padding, row, solid};
use super::theme::Look;
use crate::animation::Easing;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Key, Ui};

/// 行首图标的边长（点）。
const ICON: f32 = 20.0;

/// 醒目标出（[`Ui::spotlighted`]）撤掉后底色淡出的时长。
const SPOTLIGHT_FADE: Duration = Duration::from_millis(700);

pub struct SettingRow<'a> {
    title: &'a str,

    detail: Option<&'a str>,

    caption: Option<NodeId>,

    badge: Option<&'a str>,

    highlight: Option<&'a str>,

    info: Option<&'a str>,

    control: Option<NodeId>,

    below: Vec<NodeId>,

    /// 横跨整行、排在标题与控件下面的一块（下载进度）；与这一行是一体，中间不分隔。
    footer: Option<NodeId>,

    stacked: bool,

    /// 使用方补的搜索同义词。
    keywords: &'a [&'a str],

    /// 区分同名的行（同一帧里两处都有同一个名字时）；不给时 key 只由标题派生。
    id: Option<&'a str>,
}

impl<'a> SettingRow<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            detail: None,
            caption: None,
            badge: None,
            highlight: None,
            info: None,
            control: None,
            below: Vec::new(),
            footer: None,
            stacked: false,
            keywords: &[],
            id: None,
        }
    }

    /// 见 [`Self::id`] 字段。
    pub fn id(self, id: &'a str) -> Self {
        Self {
            id: Some(id),
            ..self
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

    /// 标题右边强调色的小标签（「当前」这类），排在 [`Self::badge`] 后面。
    pub fn highlight(self, text: &'a str) -> Self {
        Self {
            highlight: Some(text),
            ..self
        }
    }

    /// 这一行的说明：[`Look::Plain`] 收进标题右边的 ⓘ（悬停浮出），别的外观照系统设置写在标题下面一行。
    /// [`Ui::info_links`] 给了这一行的链接时，前者点 ⓘ 打开，后者在说明末尾跟一个链接。
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
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
    ) -> Result<NodeId, RenderError> {
        let key = match self.id {
            Some(id) => ui.key(("row", self.title, id)),
            None => ui.key(("row", self.title)),
        };
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
        let weight = if theme.look == Look::Plain { 500 } else { 400 };
        let mut title = label(ui, theme, self.title, theme.font.size, weight, colors.text)?;
        let (info, detail) = match theme.look {
            Look::Plain => (self.info, self.detail),
            Look::Mac | Look::Fluent => (None, self.detail.or(self.info)),
        };
        if let Some(text) = info {
            let link = ui.info_link(self.title);
            let mark = match self.id {
                Some(id) => info_with_link(ui, theme, ("info", self.title, id), text, link)?,
                None => info_with_link(ui, theme, ("info", self.title), text, link)?,
            };
            title = ui.scene().node(
                Style {
                    align_items: AlignItems::CENTER,
                    ..row(theme, 6.0)
                },
                Visual::Group,
                &[title, mark],
            )?;
        }
        if self.badge.is_some() || self.highlight.is_some() {
            let mut children = vec![title];
            if let Some(text) = self.badge {
                children.push(badge(ui, theme, text, false)?);
            }
            if let Some(text) = self.highlight {
                children.push(badge(ui, theme, text, true)?);
            }
            title = ui.scene().node(
                Style {
                    align_items: AlignItems::CENTER,
                    ..row(theme, 8.0)
                },
                Visual::Group,
                &children,
            )?;
        }
        let mut texts = vec![title];
        let mut line = Vec::new();
        if let Some(detail) = detail {
            // 说明长了折行，不把右边的控件挤出去
            line.push(ui.scene().node(
                Style {
                    flex_shrink: 1.0,
                    ..Style::default()
                },
                Visual::Text {
                    text: detail.to_owned(),
                    style: theme.text(theme.small, 400, colors.secondary).wrapped(),
                },
                &[],
            )?);
        }
        // 系统设置的写法：链接跟在说明后面，放不下时折到下一行
        if theme.look != Look::Plain
            && let Some((message, hint)) = ui.info_link(self.title)
        {
            let link = Link::new(hint).points(theme.small);
            line.push(match self.id {
                Some(id) => link.build(ui, theme, ("info", self.title, id), message)?,
                None => link.build(ui, theme, ("info", self.title), message)?,
            });
        }
        if !line.is_empty() {
            texts.push(ui.scene().node(
                Style {
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::BASELINE,
                    gap: taffy::Size {
                        width: LengthPercentage::length(theme.px(4.0)),
                        height: LengthPercentage::length(0.0),
                    },
                    ..row(theme, 0.0)
                },
                Visual::Group,
                &line,
            )?);
        }
        if let Some(caption) = self.caption {
            texts.push(caption);
        }
        let mut heading = ui.scene().node(
            Style {
                flex_shrink: 1.0,
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(0.0),
                    height: LengthPercentageAuto::auto(),
                },
                ..column(theme, if theme.look == Look::Plain { 2.0 } else { 1.0 })
            },
            Visual::Group,
            &texts,
        )?;
        // Windows 设置的卡片：标题左边一个单色线条图标
        if theme.look == Look::Fluent
            && let Some(icon) = ui.row_icon(self.title)
        {
            heading = leading_icon(ui, theme, icon, heading)?;
        }
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
            let described = detail.is_some();
            self.inline(ui, theme, spot, heading, described)?
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
        described: bool,
    ) -> Result<NodeId, RenderError> {
        let footer = self.footer;
        let (vertical, height) = inline_metrics(theme, described);
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
                    padding: padding(theme, vertical, 0.0),
                    ..column(theme, 10.0)
                },
                Visual::Group,
                &[spot, line, footer],
            );
        }
        ui.scene().node(
            Style {
                padding: padding(theme, vertical, 0.0),
                justify_content: JustifyContent::SPACE_BETWEEN,
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::auto(),
                    height: taffy::LengthPercentageAuto::length(theme.px(height)),
                },
                ..row(theme, 16.0)
            },
            Visual::Group,
            &[spot, heading, trailing],
        )
    }
}

/// 行首图标（20 点，与标题隔 16 点）跟标题那一栏排成一行。
fn leading_icon<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    icon: Arc<dyn Icon>,
    heading: NodeId,
) -> Result<NodeId, RenderError> {
    let side = theme.px(ICON);
    let icon = ui.scene().node(
        Style {
            size: taffy::Size {
                width: Dimension::length(side),
                height: Dimension::length(side),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Icon {
            icon,
            size: side,
            color: theme.colors.text,
        },
        &[],
    )?;
    ui.scene().node(
        Style {
            align_items: AlignItems::CENTER,
            flex_shrink: 1.0,
            min_size: taffy::Size {
                width: LengthPercentageAuto::length(0.0),
                height: LengthPercentageAuto::auto(),
            },
            ..row(theme, 16.0)
        },
        Visual::Group,
        &[icon, heading],
    )
}

/// 标题在左、控件在右的一行上下留白与最低高度（点）：照各系统设置里一行的高度。
fn inline_metrics(theme: &WidgetTheme, described: bool) -> (f32, f32) {
    match theme.look {
        Look::Plain => (10.0, theme.control_height + 20.0),
        Look::Mac => (7.0, 38.0),
        Look::Fluent if described => (11.0, 68.0),
        Look::Fluent => (10.0, 52.0),
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
/// `accent` 是强调色的字垫一层淡淡的强调色，不然是次要色的字垫凹槽色。
fn badge<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
    accent: bool,
) -> Result<NodeId, RenderError> {
    let colors = &theme.colors;
    let (ink, fill) = if accent {
        (
            colors.accent,
            Color {
                a: 40,
                ..colors.accent
            },
        )
    } else {
        (colors.secondary, colors.well)
    };
    let text = label(ui, theme, text, theme.small - 1.0, 500, ink)?;
    ui.scene().node(
        Style {
            padding: padding(theme, 1.0, 6.0),
            flex_shrink: 0.0,
            ..row(theme, 0.0)
        },
        solid(Some(fill), None, theme.px(theme.radius) / 2.0),
        &[text],
    )
}
