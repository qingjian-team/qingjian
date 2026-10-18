//! 场景图节点画什么：节点的盒子由布局算出，这里只描述在盒子里怎么画。

use std::sync::Arc;

use super::box_paint::BoxPaint;
use super::fill::Fill;
use super::icon::Icon;
use crate::color::Color;
use crate::text::TextStyle;

#[derive(Debug, Clone)]
pub enum Visual {
    /// 纯容器，自己不画。
    Group,

    /// 盒子：填充、边框、圆角。
    Box(BoxPaint),

    /// 单行文字，盒子左上角是行框顶边；布局时按文字量宽高。
    Text { text: String, style: TextStyle },

    /// 图标：边长 `size`，在盒子里垂直居中、靠左。
    Icon {
        icon: Arc<dyn Icon>,
        size: f32,
        color: Color,
    },
}

impl Visual {
    /// 过渡插值的主色：盒子的纯色填充、文字颜色、图标颜色；渐变、图片与没有填充的盒子没有。
    pub fn color(&self) -> Option<Color> {
        match self {
            Self::Box(BoxPaint {
                fill: Some(Fill::Solid(color)),
                ..
            }) => Some(*color),
            Self::Text { style, .. } => Some(style.color),
            Self::Icon { color, .. } => Some(*color),
            Self::Group | Self::Box(_) => None,
        }
    }

    /// 换成过渡中的颜色；没有主色的不变（没有填充的盒子补上纯色填充）。
    pub fn recolored(&self, color: Color) -> Self {
        let mut visual = self.clone();
        match &mut visual {
            Self::Box(paint) => {
                if matches!(paint.fill, None | Some(Fill::Solid(_))) {
                    paint.fill = Some(Fill::Solid(color));
                }
            }
            Self::Text { style, .. } => style.color = color,
            Self::Icon { color: own, .. } => *own = color,
            Self::Group => {}
        }
        visual
    }

    /// 纯色（圆角）矩形；`radius` 为 0 时画直角。
    pub fn solid(color: Color, radius: f32) -> Self {
        Self::Box(BoxPaint {
            fill: Some(Fill::Solid(color)),
            border: None,
            radius,
        })
    }
}
