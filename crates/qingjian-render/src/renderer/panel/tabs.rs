//! 分类标签放不下时看得见哪几个：以当前分类为中心往两边扩，直到再放一个就超宽。

/// `widths` 是各标签的宽，`gap` 是标签间距，`available` 是能用的总宽。返回看得见的首尾下标（含）；没有标签时是 `(0, 0)`。
pub(super) fn visible(widths: &[f32], current: usize, gap: f32, available: f32) -> (usize, usize) {
    let current = current.min(widths.len().saturating_sub(1));
    let (mut first, mut last) = (current, current);
    let mut used = widths.get(current).copied().unwrap_or(0.0);
    loop {
        let mut grew = false;
        if last + 1 < widths.len() && used + gap + widths[last + 1] <= available {
            last += 1;
            used += gap + widths[last];
            grew = true;
        }
        if first > 0 && used + gap + widths[first - 1] <= available {
            first -= 1;
            used += gap + widths[first];
            grew = true;
        }
        if !grew {
            return (first, last);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::visible;

    #[test]
    fn grows_around_the_current_tab_until_full() {
        let widths = [10.0; 8];
        assert_eq!(visible(&widths, 0, 2.0, 1000.0), (0, 7));
        // 放得下四个（10 × 4 + 2 × 3 = 46）
        assert_eq!(visible(&widths, 5, 2.0, 46.0), (4, 7));
        assert_eq!(visible(&widths, 0, 2.0, 46.0), (0, 3));
        assert_eq!(visible(&widths, 7, 2.0, 46.0), (4, 7));
        assert_eq!(visible(&[], 3, 2.0, 46.0), (0, 0));
    }
}
