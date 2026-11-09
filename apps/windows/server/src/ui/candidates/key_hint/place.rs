//! 键位提示窗口放哪：贴着候选窗、不压光标行、不出工作区。先试上一次用的那一侧（同一段输入里不左右跳），
//! 再按左边、右边、背向光标、光标另一侧挨个试，第一个整块放得下的就用；都放不下返回 `None`，这次不显示。
//! 规则同 macOS 壳的 `candidates::key_hint::place`，坐标换成 Windows 的屏幕像素（y 向下）。

use windows::Win32::Foundation::RECT;

/// 键盘与候选窗、光标行之间的间隙（逻辑像素，按 DPI 放大后传进 [`place`]）。
pub(super) const GAP: i32 = 4;

/// 键盘贴在候选窗的哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Left,

    /// 背向光标：候选窗在光标下方就再往下，在上方就再往上。
    Away,

    Right,

    /// 光标行的另一侧。
    Toward,
}

/// 没有上一次时的优先顺序：左右与候选窗并排，视线横着一扫就到；上下要多挪一段视线。
const ORDER: [Side; 4] = [Side::Left, Side::Right, Side::Away, Side::Toward];

/// `size` 是键盘内容区大小，`candidate` 是候选窗内容区，`caret` 是光标行，`work` 是那块显示器的工作区。
/// `sticky` 是这段输入上一次用的那一侧。返回内容区左上角与用了哪一侧。
pub(super) fn place(
    size: (i32, i32),
    candidate: RECT,
    caret: RECT,
    work: RECT,
    gap: i32,
    sticky: Option<Side>,
) -> Option<((i32, i32), Side)> {
    let (width, height) = size;
    let candidate_below = candidate.top > caret.top;
    // 光标行与候选窗合起来的上下边：键盘竖着摆时要整个让开
    let top = candidate.top.min(caret.top);
    let bottom = candidate.bottom.max(caret.bottom);
    let below = bottom + gap;
    let above = top - gap - height;
    let (away, toward) = if candidate_below {
        (below, above)
    } else {
        (above, below)
    };
    // 横着摆时与候选窗靠光标的那条边对齐（候选窗在下方对顶边，在上方对底边），挨着拼音行
    let beside_y = if candidate_below {
        candidate.top
    } else {
        candidate.bottom - height
    };
    let left = candidate.left - gap - width;
    let right = candidate.right + gap;
    let left_aligned = candidate.left;
    let right_aligned = candidate.right - width;
    let origins = |side: Side| -> Vec<(i32, i32)> {
        match side {
            Side::Left => vec![(left, beside_y)],
            Side::Right => vec![(right, beside_y)],
            Side::Away => vec![(left_aligned, away), (right_aligned, away)],
            Side::Toward => vec![(left_aligned, toward), (right_aligned, toward)],
        }
    };
    sticky.into_iter().chain(ORDER).find_map(|side| {
        origins(side).into_iter().find_map(|(x, y)| {
            let rect = RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            };
            (fits(rect, work) && !overlaps(rect, caret) && !overlaps(rect, candidate))
                .then_some(((x, y), side))
        })
    })
}

fn fits(rect: RECT, work: RECT) -> bool {
    rect.left >= work.left
        && rect.top >= work.top
        && rect.right <= work.right
        && rect.bottom <= work.bottom
}

fn overlaps(a: RECT, b: RECT) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: RECT = RECT {
        left: 0,
        top: 0,
        right: 1440,
        bottom: 900,
    };

    const KEYBOARD: (i32, i32) = (472, 174);

    fn rect(x: i32, y: i32, width: i32, height: i32) -> RECT {
        RECT {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    #[test]
    fn prefers_the_left_of_the_candidates() {
        let caret = rect(700, 282, 2, 18);
        let candidate = rect(700, 304, 300, 146);
        let (origin, side) = place(KEYBOARD, candidate, caret, WORK, GAP, None).unwrap();
        assert_eq!(side, Side::Left);
        assert_eq!(origin, (700 - GAP - KEYBOARD.0, 304));
    }

    #[test]
    fn goes_right_when_the_line_has_just_started() {
        let caret = rect(100, 282, 2, 18);
        let candidate = rect(100, 304, 300, 146);
        let (origin, side) = place(KEYBOARD, candidate, caret, WORK, GAP, None).unwrap();
        assert_eq!(side, Side::Right);
        assert_eq!(origin, (400 + GAP, 304));
    }

    #[test]
    fn goes_below_when_neither_side_fits() {
        let work = rect(0, 0, 800, 900);
        let caret = rect(300, 282, 2, 18);
        let candidate = rect(300, 304, 300, 146);
        let (origin, side) = place(KEYBOARD, candidate, caret, work, GAP, None).unwrap();
        assert_eq!(side, Side::Away);
        assert_eq!(origin.1, 450 + GAP);
    }

    #[test]
    fn keeps_the_side_it_used_while_it_still_fits() {
        let caret = rect(700, 282, 2, 18);
        let candidate = rect(700, 304, 300, 146);
        let (_, side) = place(KEYBOARD, candidate, caret, WORK, GAP, Some(Side::Away)).unwrap();
        assert_eq!(side, Side::Away);
    }

    #[test]
    fn goes_above_candidates_that_sit_above_the_caret() {
        // 光标在屏幕底部，候选窗翻到了上方
        let caret = rect(100, 842, 2, 18);
        let candidate = rect(100, 692, 300, 146);
        let (origin, side) =
            place(KEYBOARD, candidate, caret, WORK, GAP, Some(Side::Away)).unwrap();
        assert_eq!(side, Side::Away);
        assert_eq!(origin.1, 692 - GAP - KEYBOARD.1);
    }

    #[test]
    fn right_aligns_below_at_the_right_edge() {
        let caret = rect(1300, 282, 2, 18);
        let candidate = rect(1140, 304, 300, 146);
        let (origin, _) = place(KEYBOARD, candidate, caret, WORK, GAP, Some(Side::Away)).unwrap();
        assert_eq!(origin.0, 1440 - KEYBOARD.0);
    }

    #[test]
    fn hides_when_the_work_area_is_smaller_than_the_keyboard() {
        let work = rect(0, 0, 400, 300);
        let caret = rect(10, 82, 2, 18);
        let candidate = rect(10, 104, 200, 146);
        assert_eq!(place(KEYBOARD, candidate, caret, work, GAP, None), None);
    }
}
