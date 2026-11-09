//! 拼音行上各字之间的位置：点拼音行时光标该落到第几个字之后。

#[derive(Debug, Clone, PartialEq)]
pub struct PreeditHit {
    /// 拼音行的上沿（内容区像素）。
    pub y: f32,

    pub height: f32,

    /// 第 `i` 个字之前那道缝的 x（内容区像素），`edges[0]` 是行首、最后一个是行尾，比字数多一个。
    pub edges: Vec<f32>,
}

impl PreeditHit {
    /// 横坐标 `x`（内容区像素）离哪道缝最近，光标就落在那里（第几个字之后）；行外按最近的一头算。
    pub fn caret_at(&self, x: f32) -> usize {
        self.edges
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (*a - x).abs().total_cmp(&(*b - x).abs()))
            .map_or(0, |(i, _)| i)
    }

    /// 点 `y`（内容区像素）落在拼音行这一条里。
    pub fn contains_y(&self, y: f32) -> bool {
        y >= self.y && y < self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::PreeditHit;

    #[test]
    fn taps_snap_to_the_nearest_gap_between_letters() {
        let hit = PreeditHit {
            y: 0.0,
            height: 20.0,
            edges: vec![10.0, 20.0, 30.0, 40.0],
        };
        assert_eq!(hit.caret_at(0.0), 0);
        assert_eq!(hit.caret_at(14.0), 0);
        assert_eq!(hit.caret_at(16.0), 1);
        assert_eq!(hit.caret_at(33.0), 2);
        assert_eq!(hit.caret_at(99.0), 3);
        assert!(hit.contains_y(5.0) && !hit.contains_y(20.0));
    }
}
