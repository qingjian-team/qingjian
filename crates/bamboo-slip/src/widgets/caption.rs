//! Windows 11 标题栏右上角的三个按钮（最小化、最大化 / 还原、关闭）。
//! 每个按钮按 WinUI 标准宽 46 点、撑满标题栏高度，悬停与按下走引擎统一的缓动；
//! 关闭按钮悬停变红底白字，窗口失焦时图标整体变淡。点击与贴靠布局由宿主按 [`WindowRegion`] 交给系统处理。

use std::sync::Arc;

use taffy::{AlignItems, Dimension, FlexDirection, JustifyContent, NodeId, Size, Style};
use tiny_skia::PathBuilder;

use super::WidgetTheme;
use super::parts::solid;
use crate::animation::Easing;
use crate::canvas::Canvas;
use crate::color::Color;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Ui, WindowRegion};

/// 单个标题栏按钮的宽度与高度（点），对齐 Windows 11 原生贴顶三按钮（46 × 32）。
pub const CAPTION_BUTTON_WIDTH: f32 = 46.0;

pub const CAPTION_BUTTON_HEIGHT: f32 = 32.0;

/// 标题栏推荐高度（点，Win11 带搜索框的 Tall TitleBar）。
pub const CAPTION_BAR_HEIGHT: f32 = 48.0;

/// Windows 11 关闭按钮悬停底色。
const CLOSE_HOVER: Color = Color::rgb(0xC4, 0x2B, 0x1C);

const CLOSE_PRESSED: Color = Color::rgba(0xC4, 0x2B, 0x1C, 229);

const CLOSE_TEXT: Color = Color::rgb(0xFF, 0xFF, 0xFF);

const CLOSE_TEXT_PRESSED: Color = Color::gray(255, 220);

#[derive(Debug, Clone, Copy)]
enum CaptionGlyph {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl Icon for CaptionGlyph {
    fn draw(&self, canvas: &mut Canvas, x: f32, y: f32, size: f32, color: Color) {
        let stroke = (size * 0.1).round().max(1.0);
        // 奇数线宽对齐到半像素，水平与竖直线条最锐利
        let align = |v: f32| {
            if (stroke as i32) % 2 == 1 {
                v.floor() + 0.5
            } else {
                v.round()
            }
        };
        let mut path = PathBuilder::new();
        match self {
            CaptionGlyph::Minimize => {
                let mid = align(y + size * 0.5);
                path.move_to(x.round(), mid);
                path.line_to((x + size).round(), mid);
            }
            CaptionGlyph::Maximize => {
                let left = align(x + stroke * 0.5);
                let top = align(y + stroke * 0.5);
                let right = align(x + size - stroke * 0.5);
                let bottom = align(y + size - stroke * 0.5);
                path.move_to(left, top);
                path.line_to(right, top);
                path.line_to(right, bottom);
                path.line_to(left, bottom);
                path.close();
            }
            CaptionGlyph::Restore => {
                let step = (size * 0.2).round().max(2.0);
                // 前景方框（偏左下）
                let left = align(x + stroke * 0.5);
                let top = align(y + step + stroke * 0.5);
                let right = align(x + size - step - stroke * 0.5);
                let bottom = align(y + size - stroke * 0.5);
                path.move_to(left, top);
                path.line_to(right, top);
                path.line_to(right, bottom);
                path.line_to(left, bottom);
                path.close();
                // 后景方框露出的上右两边（偏右上）
                let back_left = align(x + step + stroke * 0.5);
                let back_top = align(y + stroke * 0.5);
                let back_right = align(x + size - stroke * 0.5);
                let back_bottom = align(y + size - step - stroke * 0.5);
                path.move_to(back_left, top);
                path.line_to(back_left, back_top);
                path.line_to(back_right, back_top);
                path.line_to(back_right, back_bottom);
                path.line_to(right, back_bottom);
            }
            CaptionGlyph::Close => {
                let inset = stroke * 0.5;
                path.move_to(x + inset, y + inset);
                path.line_to(x + size - inset, y + size - inset);
                path.move_to(x + size - inset, y + inset);
                path.line_to(x + inset, y + size - inset);
            }
        }
        if let Some(path) = path.finish() {
            match self {
                CaptionGlyph::Close => canvas.stroke_outline(&path, stroke, color),
                _ => canvas.stroke_path(&path, stroke, color),
            }
        }
    }
}

/// 建出最小化、最大化（或还原）、关闭三个并排按钮，高撑满父容器。
pub fn caption_buttons<M: Clone + 'static>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
) -> Result<NodeId, RenderError> {
    let active = ui.window_active();
    let maximized = ui.window_maximized();
    let specs = [
        ("min", CaptionGlyph::Minimize, WindowRegion::Minimize, false),
        (
            "max",
            if maximized {
                CaptionGlyph::Restore
            } else {
                CaptionGlyph::Maximize
            },
            WindowRegion::Maximize,
            false,
        ),
        ("close", CaptionGlyph::Close, WindowRegion::Close, true),
    ];
    let buttons = ui.scope(
        "caption-buttons",
        |ui| -> Result<Vec<NodeId>, RenderError> {
            let mut out = Vec::with_capacity(3);
            for (name, glyph, region, is_close) in specs {
                let key = ui.key(name);
                let state = ui.state(key);
                let fill = if is_close {
                    if state.pressed {
                        Some(CLOSE_PRESSED)
                    } else if state.hovered {
                        Some(CLOSE_HOVER)
                    } else {
                        None
                    }
                } else if state.pressed {
                    Some(theme.colors.pressed)
                } else if state.hovered {
                    Some(theme.colors.hover)
                } else {
                    None
                };
                let icon_color = if is_close && (state.hovered || state.pressed) {
                    if state.pressed {
                        CLOSE_TEXT_PRESSED
                    } else {
                        CLOSE_TEXT
                    }
                } else if active {
                    theme.colors.text
                } else {
                    Color {
                        a: ((u16::from(theme.colors.text.a) * 45) / 100) as u8,
                        ..theme.colors.text
                    }
                };
                let icon_size = theme.px(10.0);
                let icon = ui.scene().node(
                    Style {
                        size: Size {
                            width: Dimension::length(icon_size),
                            height: Dimension::length(icon_size),
                        },
                        flex_shrink: 0.0,
                        ..Style::default()
                    },
                    Visual::Icon {
                        icon: Arc::new(glyph),
                        size: icon_size,
                        color: icon_color,
                    },
                    &[],
                )?;
                let button = ui.scene().node(
                    Style {
                        size: Size {
                            width: Dimension::length(theme.px(CAPTION_BUTTON_WIDTH)),
                            height: Dimension::length(theme.px(CAPTION_BUTTON_HEIGHT)),
                        },
                        flex_shrink: 0.0,
                        justify_content: JustifyContent::CENTER,
                        align_items: AlignItems::CENTER,
                        ..Style::default()
                    },
                    solid(fill, None, 0.0),
                    &[icon],
                )?;
                ui.fade(button, key, theme.hover, Easing::Ease);
                ui.interact(button, key, Interact::new().window_region(region));
                out.push(button);
            }
            Ok(out)
        },
    )?;
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FLEX_START,
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(theme.px(CAPTION_BUTTON_HEIGHT)),
            },
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Group,
        &buttons,
    )
}
