//! 主题颜色与素材访问器：变量按当前外观取值，图片与 SVG 从共享缓存里取。

use std::sync::Arc;

use bamboo_slip::{Color, Pixmap, SvgImage};

use super::file::ColorRef;
use super::Theme;

impl Theme {
    /// 主题变量按当前外观取的颜色（`surface`、`accent`…）；没有这个变量时为 `None`。
    /// 渲染器之外还要配色的地方用（手机壳的候选条底色、键盘）。
    pub fn variable(&self, name: &str) -> Option<Color> {
        self.file
            .variables
            .get(name)
            .map(|value| value.get(self.dark).0)
    }

    /// 颜色引用按当前外观取值；变量不存在时透明。
    pub(crate) fn color(&self, color: &ColorRef) -> Color {
        match color {
            ColorRef::Literal(color) => *color,
            ColorRef::Variable(name) => self.file.variables.get(name).map_or(
                Color::rgba(0, 0, 0, 0),
                |value| value.get(self.dark).0,
            ),
        }
    }

    /// 主题里写的图片路径对应的位图；没有、没读进来或是 SVG 时为 `None`。
    pub(crate) fn image(&self, path: &str) -> Option<Arc<Pixmap>> {
        self.assets().image(path)
    }

    /// 主题里写的 SVG 图片；没有、没读进来或不是 SVG 时为 `None`。
    pub(crate) fn svg(&self, path: &str) -> Option<Arc<SvgImage>> {
        self.assets().svg(path)
    }
}
