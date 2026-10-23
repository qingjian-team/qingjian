//! 横排、高亮移动键选了 ← / → 时，方向键在拼音行与候选两区之间怎么走：
//! ↑ / ↓ 换区，← / → 只在当前区里移动。各平台壳共用这一套，壳只记焦点、执行动作。
//! 光标在拼音里按 ↓ 进候选、再按 ↓ 展开；↑ 不收起矩阵（收起用 Esc），在第一排或单行时回拼音行。

mod action;
mod arrow;
mod focus;
mod matrix;

pub use action::ArrowAction;
pub use arrow::Arrow;
pub use focus::Focus;
pub use matrix::Matrix;

pub fn route(arrow: Arrow, focus: Focus, matrix: Matrix) -> ArrowAction {
    match (focus, arrow) {
        (Focus::Pinyin, Arrow::Left) => ArrowAction::MoveCursor(-1),
        (Focus::Pinyin, Arrow::Right) => ArrowAction::MoveCursor(1),
        (Focus::Pinyin, Arrow::Down) => ArrowAction::Focus(Focus::Candidates),
        (Focus::Pinyin, Arrow::Up) => ArrowAction::Stay,
        (Focus::Candidates, Arrow::Left) => ArrowAction::MoveHighlight(-1),
        (Focus::Candidates, Arrow::Right) => ArrowAction::MoveHighlight(1),
        (Focus::Candidates, Arrow::Down) => match matrix {
            Matrix::Off => ArrowAction::Stay,
            Matrix::Closed => ArrowAction::Expand,
            Matrix::Open { .. } => ArrowAction::MoveRow(1),
        },
        (Focus::Candidates, Arrow::Up) => match matrix {
            Matrix::Open { first_row: false } => ArrowAction::MoveRow(-1),
            _ => ArrowAction::Focus(Focus::Pinyin),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_goes_from_pinyin_to_candidates_then_expands() {
        assert_eq!(
            route(Arrow::Down, Focus::Pinyin, Matrix::Closed),
            ArrowAction::Focus(Focus::Candidates)
        );
        assert_eq!(
            route(Arrow::Down, Focus::Candidates, Matrix::Closed),
            ArrowAction::Expand
        );
        assert_eq!(
            route(
                Arrow::Down,
                Focus::Candidates,
                Matrix::Open { first_row: true }
            ),
            ArrowAction::MoveRow(1)
        );
        assert_eq!(
            route(Arrow::Down, Focus::Candidates, Matrix::Off),
            ArrowAction::Stay
        );
    }

    #[test]
    fn up_climbs_rows_then_returns_to_pinyin_without_collapsing() {
        assert_eq!(
            route(
                Arrow::Up,
                Focus::Candidates,
                Matrix::Open { first_row: false }
            ),
            ArrowAction::MoveRow(-1)
        );
        for matrix in [
            Matrix::Off,
            Matrix::Closed,
            Matrix::Open { first_row: true },
        ] {
            assert_eq!(
                route(Arrow::Up, Focus::Candidates, matrix),
                ArrowAction::Focus(Focus::Pinyin)
            );
        }
        assert_eq!(
            route(Arrow::Up, Focus::Pinyin, Matrix::Closed),
            ArrowAction::Stay
        );
    }

    #[test]
    fn left_and_right_stay_inside_the_focused_area() {
        assert_eq!(
            route(Arrow::Left, Focus::Pinyin, Matrix::Closed),
            ArrowAction::MoveCursor(-1)
        );
        assert_eq!(
            route(Arrow::Left, Focus::Candidates, Matrix::Closed),
            ArrowAction::MoveHighlight(-1)
        );
        assert_eq!(
            route(
                Arrow::Right,
                Focus::Candidates,
                Matrix::Open { first_row: true }
            ),
            ArrowAction::MoveHighlight(1)
        );
    }
}
