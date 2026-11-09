//! 面板从屏幕哪条边弹出，以及弹出后占的矩形。

mod work_area;

pub use work_area::WorkArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// 上下弹出时的高度、左右弹出时的宽度（点）。
const DEPTH: (f32, f32) = (360.0, 420.0);

impl Edge {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    /// 上下弹出的面板宽，卡片横着排、横着滚；左右弹出的竖着排。
    pub fn horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    /// 滑入的起点：面板整个在这条边外面。
    pub fn slide_start(self, area: WorkArea, scale: f32) -> (i32, i32) {
        let (x, y, width, height) = self.rect(area, scale);
        match self {
            Self::Top => (x, area.top - height as i32),
            Self::Bottom => (x, area.bottom),
            Self::Left => (area.left - width as i32, y),
            Self::Right => (area.right, y),
        }
    }

    /// 面板的 `(x, y, 宽, 高)`：贴着这条边、铺满这条边的长度，厚度不超过工作区。
    pub fn rect(self, area: WorkArea, scale: f32) -> (i32, i32, u32, u32) {
        let (width, height) = (area.right - area.left, area.bottom - area.top);
        let tall = ((DEPTH.0 * scale) as i32).min(height);
        let wide = ((DEPTH.1 * scale) as i32).min(width);
        let (x, y, w, h) = match self {
            Self::Top => (area.left, area.top, width, tall),
            Self::Bottom => (area.left, area.bottom - tall, width, tall),
            Self::Left => (area.left, area.top, wide, height),
            Self::Right => (area.right - wide, area.top, wide, height),
        };
        (x, y, w.max(1) as u32, h.max(1) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::{Edge, WorkArea};

    const AREA: WorkArea = WorkArea {
        left: 100,
        top: 0,
        right: 2020,
        bottom: 1040,
    };

    #[test]
    fn panel_hugs_the_chosen_edge() {
        assert_eq!(Edge::Top.rect(AREA, 2.0), (100, 0, 1920, 720));
        assert_eq!(Edge::Bottom.rect(AREA, 2.0), (100, 320, 1920, 720));
        assert_eq!(Edge::Left.rect(AREA, 2.0), (100, 0, 840, 1040));
        assert_eq!(Edge::Right.rect(AREA, 2.0), (1180, 0, 840, 1040));
    }

    #[test]
    fn panel_slides_in_from_outside_its_edge() {
        assert_eq!(Edge::Top.slide_start(AREA, 2.0), (100, -720));
        assert_eq!(Edge::Bottom.slide_start(AREA, 2.0), (100, 1040));
        assert_eq!(Edge::Left.slide_start(AREA, 2.0), (-740, 0));
        assert_eq!(Edge::Right.slide_start(AREA, 2.0), (2020, 0));
    }

    #[test]
    fn panel_never_exceeds_the_work_area() {
        assert_eq!(Edge::Bottom.rect(AREA, 4.0), (100, 0, 1920, 1040));
    }
}
