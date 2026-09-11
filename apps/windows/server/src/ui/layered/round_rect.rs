//! 内容圆角矩形的几何：到边界的有符号距离、阴影的方向权重。

/// 内容圆角矩形（位图坐标）。
pub(super) struct RoundRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    radius: f64,
}

impl RoundRect {
    pub(super) fn content(content: (i32, i32), margin: i32, radius: i32) -> Self {
        let left = margin as f64;
        let top = margin as f64;
        Self {
            left,
            top,
            right: left + content.0 as f64,
            bottom: top + content.1 as f64,
            radius: radius as f64,
        }
    }

    /// 像素中心到圆角矩形的有符号距离：内部为负、边界为 0、外部为正。
    pub(super) fn distance(&self, x: i32, y: i32) -> f64 {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let cx = (self.left + self.right) / 2.0;
        let cy = (self.top + self.bottom) / 2.0;
        let half_w = (self.right - self.left) / 2.0;
        let half_h = (self.bottom - self.top) / 2.0;
        let dx = (px - cx).abs() - half_w + self.radius;
        let dy = (py - cy).abs() - half_h + self.radius;
        let outside = dx.max(0.0).hypot(dy.max(0.0));
        let inside = dx.max(dy).min(0.0);
        inside + outside - self.radius
    }

    /// 外部一点的「朝下程度」：正上方 0、两侧 0.5、正下 1。
    pub(super) fn down_weight(&self, x: i32, y: i32) -> f64 {
        let px = x as f64 + 0.5;
        let py = y as f64 + 0.5;
        let vx = px - px.clamp(self.left, self.right);
        let vy = py - py.clamp(self.top, self.bottom);
        let len = vx.hypot(vy);
        if len <= 0.0 {
            0.5
        } else {
            (vy / len + 1.0) * 0.5
        }
    }
}
