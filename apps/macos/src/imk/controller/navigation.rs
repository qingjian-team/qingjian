//! 候选中的 Ctrl 导航：按候选排列方向区分逐项移动与翻页。

use qingjian_platform::{LayoutMode, Modifiers};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Navigation {
    Candidate(isize),
    Page(isize),
    PageEdge(bool),
}

pub(super) fn shortcut(
    enabled: bool,
    composing: bool,
    has_candidates: bool,
    layout: LayoutMode,
    modifiers: Modifiers,
    key: Option<char>,
) -> Option<Navigation> {
    if !enabled
        || !composing
        || !has_candidates
        || !modifiers.control
        || modifiers.shift
        || modifiers.option
        || modifiers.command
    {
        return None;
    }
    let (horizontal_key, delta) = match key? {
        'a' => return Some(Navigation::PageEdge(false)),
        'e' => return Some(Navigation::PageEdge(true)),
        'b' => (true, -1),
        'f' => (true, 1),
        'p' => (false, -1),
        'n' => (false, 1),
        _ => return None,
    };
    if horizontal_key == (layout == LayoutMode::Horizontal) {
        Some(Navigation::Candidate(delta))
    } else {
        Some(Navigation::Page(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::{Navigation, shortcut};
    use qingjian_platform::{LayoutMode, Modifiers};

    fn ctrl() -> Modifiers {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    }

    #[test]
    fn maps_both_layouts() {
        for (layout, cases) in [
            (
                LayoutMode::Horizontal,
                [
                    ('b', Navigation::Candidate(-1)),
                    ('f', Navigation::Candidate(1)),
                    ('p', Navigation::Page(-1)),
                    ('n', Navigation::Page(1)),
                    ('a', Navigation::PageEdge(false)),
                    ('e', Navigation::PageEdge(true)),
                ],
            ),
            (
                LayoutMode::Vertical,
                [
                    ('b', Navigation::Page(-1)),
                    ('f', Navigation::Page(1)),
                    ('p', Navigation::Candidate(-1)),
                    ('n', Navigation::Candidate(1)),
                    ('a', Navigation::PageEdge(false)),
                    ('e', Navigation::PageEdge(true)),
                ],
            ),
        ] {
            for (key, expected) in cases {
                assert_eq!(
                    shortcut(true, true, true, layout, ctrl(), Some(key)),
                    Some(expected)
                );
            }
        }
    }

    #[test]
    fn passes_unmatched_or_disabled_shortcuts_through() {
        let call = |enabled, composing, candidates, modifiers, key| {
            shortcut(
                enabled,
                composing,
                candidates,
                LayoutMode::Vertical,
                modifiers,
                key,
            )
        };
        assert_eq!(call(false, true, true, ctrl(), Some('a')), None);
        assert_eq!(call(false, true, true, ctrl(), Some('f')), None);
        assert_eq!(call(true, false, true, ctrl(), Some('f')), None);
        assert_eq!(call(true, true, false, ctrl(), Some('f')), None);
        assert_eq!(
            call(true, true, true, Modifiers::default(), Some('f')),
            None
        );
        for modifiers in [
            Modifiers {
                shift: true,
                ..ctrl()
            },
            Modifiers {
                option: true,
                ..ctrl()
            },
            Modifiers {
                command: true,
                ..ctrl()
            },
        ] {
            assert_eq!(call(true, true, true, modifiers, Some('f')), None);
        }
        assert_eq!(call(true, true, true, ctrl(), Some('x')), None);
        assert_eq!(call(true, true, true, ctrl(), None), None);
    }
}
