//! 键位提示窗口放哪：贴着候选窗、不压光标行、不出屏。先试上一次用的那一侧（同一段输入里不左右跳），
//! 再按左边、右边、背向光标、光标另一侧挨个试，第一个整块放得下的就用；都放不下返回 `None`，这次不显示。
//! 上下两侧先与候选窗左对齐、再右对齐，贴屏幕边时自然落到候选窗的左下 / 右下；
//! 候选窗的斜角不用另试：斜角放得下时，同一高度上与候选窗对齐的位置一定也放得下。
//!
//! 只知道光标行的矩形，拿不到整个输入框，所以能保证的是不盖光标那一行，与候选窗同样的约定。

use objc2_foundation::{NSPoint, NSRect, NSSize};

/// 键盘与候选窗、光标行之间的间隙（点）。
const GAP: f64 = 4.0;

/// 键盘贴在候选窗的哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,

    /// 背向光标：候选窗在光标下方就再往下，在上方就再往上。
    Away,

    Right,

    /// 光标行的另一侧。
    Toward,
}

/// 没有上一次时的优先顺序：左右与候选窗并排，视线横着一扫就到；上下要多挪一段视线。
const ORDER: [Side; 4] = [Side::Left, Side::Right, Side::Away, Side::Toward];

/// `size` 是键盘内容区大小，`candidate` 是候选窗内容区，`caret` 是光标行，`screen` 是那块屏幕的可见区域；坐标都是屏幕坐标（左下为原点）。
/// `sticky` 是这段输入上一次用的那一侧。返回内容区左下角与用了哪一侧。
pub fn place(
    size: NSSize,
    candidate: NSRect,
    caret: NSRect,
    screen: NSRect,
    sticky: Option<Side>,
) -> Option<(NSPoint, Side)> {
    let candidate_below = candidate.origin.y < caret.origin.y;
    // 光标行与候选窗合起来的上下边：键盘竖着摆时要整个让开
    let low = candidate.origin.y.min(caret.origin.y);
    let high = max_y(candidate).max(max_y(caret));
    let below = low - GAP - size.height;
    let above = high + GAP;
    let (away, toward) = if candidate_below {
        (below, above)
    } else {
        (above, below)
    };
    // 横着摆时与候选窗靠光标的那条边对齐（候选窗在下方对顶边，在上方对底边），挨着拼音行
    let beside_y = if candidate_below {
        max_y(candidate) - size.height
    } else {
        candidate.origin.y
    };
    // 每一侧要试的位置：上下两侧先与候选窗左对齐、再右对齐
    let left = candidate.origin.x - GAP - size.width;
    let right = max_x(candidate) + GAP;
    let left_aligned = candidate.origin.x;
    let right_aligned = max_x(candidate) - size.width;
    let origins = |side: Side| -> Vec<NSPoint> {
        match side {
            Side::Left => vec![NSPoint::new(left, beside_y)],
            Side::Right => vec![NSPoint::new(right, beside_y)],
            Side::Away => vec![
                NSPoint::new(left_aligned, away),
                NSPoint::new(right_aligned, away),
            ],
            Side::Toward => vec![
                NSPoint::new(left_aligned, toward),
                NSPoint::new(right_aligned, toward),
            ],
        }
    };
    sticky.into_iter().chain(ORDER).find_map(|side| {
        origins(side).into_iter().find_map(|origin| {
            let rect = NSRect::new(origin, size);
            (fits(rect, screen) && !overlaps(rect, caret) && !overlaps(rect, candidate))
                .then_some((origin, side))
        })
    })
}

fn fits(rect: NSRect, screen: NSRect) -> bool {
    rect.origin.x >= screen.origin.x
        && rect.origin.y >= screen.origin.y
        && max_x(rect) <= max_x(screen)
        && max_y(rect) <= max_y(screen)
}

fn overlaps(a: NSRect, b: NSRect) -> bool {
    a.origin.x < max_x(b) && b.origin.x < max_x(a) && a.origin.y < max_y(b) && b.origin.y < max_y(a)
}

fn max_x(rect: NSRect) -> f64 {
    rect.origin.x + rect.size.width
}

fn max_y(rect: NSRect) -> f64 {
    rect.origin.y + rect.size.height
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: NSRect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0));

    const KEYBOARD: NSSize = NSSize::new(472.0, 174.0);

    fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
    }

    #[test]
    fn prefers_the_left_of_the_candidates() {
        let caret = rect(700.0, 600.0, 2.0, 18.0);
        let candidate = rect(700.0, 450.0, 300.0, 146.0);
        let (origin, side) = place(KEYBOARD, candidate, caret, SCREEN, None).unwrap();
        assert_eq!(side, Side::Left);
        assert_eq!(
            origin,
            NSPoint::new(700.0 - GAP - KEYBOARD.width, 596.0 - KEYBOARD.height)
        );
    }

    #[test]
    fn goes_right_when_the_line_has_just_started() {
        let caret = rect(100.0, 600.0, 2.0, 18.0);
        let candidate = rect(100.0, 450.0, 300.0, 146.0);
        let (origin, side) = place(KEYBOARD, candidate, caret, SCREEN, None).unwrap();
        assert_eq!(side, Side::Right);
        assert_eq!(origin, NSPoint::new(400.0 + GAP, 596.0 - KEYBOARD.height));
    }

    #[test]
    fn goes_below_when_neither_side_fits() {
        // 窄屏：候选窗左右都放不下
        let screen = rect(0.0, 0.0, 800.0, 900.0);
        let caret = rect(300.0, 600.0, 2.0, 18.0);
        let candidate = rect(300.0, 450.0, 300.0, 146.0);
        let (origin, side) = place(KEYBOARD, candidate, caret, screen, None).unwrap();
        assert_eq!(side, Side::Away);
        assert_eq!(origin.y, 450.0 - GAP - KEYBOARD.height);
    }

    #[test]
    fn keeps_the_side_it_used_while_it_still_fits() {
        // 光标走到了左边放得下的地方，仍留在下方，不跳
        let caret = rect(700.0, 600.0, 2.0, 18.0);
        let candidate = rect(700.0, 450.0, 300.0, 146.0);
        let (_, side) = place(KEYBOARD, candidate, caret, SCREEN, Some(Side::Away)).unwrap();
        assert_eq!(side, Side::Away);
    }

    #[test]
    fn goes_above_candidates_that_sit_above_the_caret() {
        // 光标在屏幕底部，候选窗翻到了上方
        let caret = rect(100.0, 40.0, 2.0, 18.0);
        let candidate = rect(100.0, 62.0, 300.0, 146.0);
        let (origin, side) = place(KEYBOARD, candidate, caret, SCREEN, Some(Side::Away)).unwrap();
        assert_eq!(side, Side::Away);
        assert_eq!(origin.y, 62.0 + 146.0 + GAP);
    }

    #[test]
    fn right_aligns_below_at_the_right_edge() {
        let caret = rect(1300.0, 600.0, 2.0, 18.0);
        let candidate = rect(1140.0, 450.0, 300.0, 146.0);
        let (origin, _) = place(KEYBOARD, candidate, caret, SCREEN, Some(Side::Away)).unwrap();
        assert_eq!(origin.x, 1440.0 - KEYBOARD.width);
    }

    #[test]
    fn never_covers_the_caret_line() {
        let screen = rect(0.0, 0.0, 500.0, 380.0);
        let caret = rect(10.0, 200.0, 2.0, 18.0);
        let candidate = rect(10.0, 50.0, 480.0, 146.0);
        if let Some((origin, _)) = place(KEYBOARD, candidate, caret, screen, None) {
            assert!(!overlaps(NSRect::new(origin, KEYBOARD), caret));
            assert!(!overlaps(NSRect::new(origin, KEYBOARD), candidate));
        }
    }

    #[test]
    fn hides_when_the_screen_is_smaller_than_the_keyboard() {
        let screen = rect(0.0, 0.0, 400.0, 300.0);
        let caret = rect(10.0, 200.0, 2.0, 18.0);
        let candidate = rect(10.0, 50.0, 200.0, 146.0);
        assert_eq!(place(KEYBOARD, candidate, caret, screen, None), None);
    }
}
