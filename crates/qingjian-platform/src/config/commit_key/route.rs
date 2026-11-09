//! 组句中一次按键与两个上屏键的关系。

use super::CommitKey;
use crate::protocol::KeyEvent;

/// 上屏高亮候选与原样上屏敲的字母各用哪个键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitKeys {
    pub candidate: CommitKey,

    pub raw: CommitKey,
}

/// [`CommitKeys::route`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitRoute {
    /// 照原来的分派走：不是上屏键，或者正是缺省的那个键（不带 Shift 的空格上屏候选、回车上屏字母）——
    /// 原来的路径里还有直输段、只剩一个 `?` 这类细节，交给它。
    Keep,

    /// 换了键的上屏高亮候选。
    Candidate,

    /// 换了键的原样上屏字母。
    Raw,

    /// 不带 Shift 的空格 / 回车让给了别的键、自己又没配到哪一项：组句中吃掉不做事，免得漏进应用。
    Unbound,
}

impl Default for CommitKeys {
    fn default() -> Self {
        Self {
            candidate: CommitKey::SPACE,
            raw: CommitKey::ENTER,
        }
    }
}

impl CommitKeys {
    /// 组句中（在不在组句由壳判断）这次按键走哪条。两项配成同一个键时上屏候选优先。
    pub fn route(self, event: &KeyEvent) -> CommitRoute {
        let space = CommitKey::SPACE.matches(event);
        let enter = CommitKey::ENTER.matches(event);
        if self.candidate.matches(event) {
            if space {
                CommitRoute::Keep
            } else {
                CommitRoute::Candidate
            }
        } else if self.raw.matches(event) {
            if enter {
                CommitRoute::Keep
            } else {
                CommitRoute::Raw
            }
        } else if space || enter {
            CommitRoute::Unbound
        } else {
            CommitRoute::Keep
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CommitKey, CommitKeys, CommitRoute};
    use crate::protocol::{KeyEvent, KeyModifiers};

    fn key(virtual_key: u32, character: Option<char>, shift: bool) -> KeyEvent {
        KeyEvent::new(
            virtual_key,
            character,
            KeyModifiers {
                shift,
                ..KeyModifiers::default()
            },
        )
    }

    fn space(shift: bool) -> KeyEvent {
        key(0x20, Some(' '), shift)
    }

    fn enter(shift: bool) -> KeyEvent {
        key(0x0D, Some('\r'), shift)
    }

    #[test]
    fn defaults_keep_the_old_paths() {
        let keys = CommitKeys::default();
        for event in [
            space(false),
            enter(false),
            enter(true),
            key(0xBA, Some(';'), false),
        ] {
            assert_eq!(keys.route(&event), CommitRoute::Keep);
        }
    }

    #[test]
    fn rebound_keys_take_over_and_freed_keys_are_swallowed() {
        let swapped = CommitKeys {
            candidate: CommitKey::ENTER,
            raw: CommitKey::Enter { shift: true },
        };
        assert_eq!(swapped.route(&enter(false)), CommitRoute::Candidate);
        assert_eq!(swapped.route(&enter(true)), CommitRoute::Raw);
        assert_eq!(swapped.route(&space(false)), CommitRoute::Unbound);
        let mut ctrl = enter(false);
        ctrl.modifiers.ctrl = true;
        assert_eq!(swapped.route(&ctrl), CommitRoute::Keep);
        let symbol = CommitKeys {
            candidate: CommitKey::Symbol(';'),
            raw: CommitKey::ENTER,
        };
        assert_eq!(
            symbol.route(&key(0xBA, Some(';'), false)),
            CommitRoute::Candidate
        );
        assert_eq!(symbol.route(&key(0xBA, Some(':'), true)), CommitRoute::Keep);
        assert_eq!(symbol.route(&space(false)), CommitRoute::Unbound);
        assert_eq!(symbol.route(&enter(false)), CommitRoute::Keep);
    }

    #[test]
    fn keys_parse_and_round_trip() {
        bamboo_slip_i18n::pin_locale("zh-Hans");
        for (text, key) in [
            ("space", CommitKey::SPACE),
            ("shift+enter", CommitKey::Enter { shift: true }),
            ("Shift+Space", CommitKey::Space { shift: true }),
            (";", CommitKey::Symbol(';')),
        ] {
            assert_eq!(text.parse::<CommitKey>(), Ok(key));
            assert_eq!(key.key_string().parse::<CommitKey>(), Ok(key));
        }
        for bad in ["", "a", "1", "'", "ctrl+enter", "tab", ";;"] {
            assert!(bad.parse::<CommitKey>().is_err(), "{bad}");
        }
        assert_eq!(CommitKey::Enter { shift: true }.label(), "Shift + 回车");
    }
}
