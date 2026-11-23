//! [`Stamp`]：一条内容的 id、时刻与删除状态，同步时只比较它。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub id: String,

    /// 最后修改的 Unix 毫秒。
    pub at: i64,

    pub deleted: bool,

    /// 只是本机不要了（超出条数上限、清空）：别的设备不跟着删，本机也不会再把旧内容要回来。
    #[serde(default)]
    pub forgotten: bool,
}

impl Stamp {
    /// 比 `other` 新：时刻大的新；同一时刻删除优先，免得删了的又回来。
    pub fn newer_than(&self, other: &Stamp) -> bool {
        (self.at, self.deleted) > (other.at, other.deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::Stamp;

    fn stamp(id: &str, at: i64, deleted: bool) -> Stamp {
        Stamp {
            id: id.to_owned(),
            at,
            deleted,
            forgotten: false,
        }
    }

    #[test]
    fn later_stamps_are_newer() {
        assert!(stamp("a", 2, false).newer_than(&stamp("a", 1, false)));
        assert!(!stamp("a", 1, false).newer_than(&stamp("a", 2, false)));
        assert!(!stamp("a", 1, false).newer_than(&stamp("a", 1, false)));
    }

    #[test]
    fn a_delete_beats_the_same_instant() {
        assert!(stamp("a", 1, true).newer_than(&stamp("a", 1, false)));
        assert!(!stamp("a", 1, false).newer_than(&stamp("a", 1, true)));
    }

    #[test]
    fn forgotten_is_optional_on_the_wire() {
        let parsed: Stamp = serde_json::from_str(r#"{"id":"a","at":1,"deleted":false}"#).unwrap();
        assert!(!parsed.forgotten);
        let parsed: Stamp =
            serde_json::from_str(r#"{"id":"a","at":1,"deleted":false,"forgotten":true}"#).unwrap();
        assert!(parsed.forgotten);
    }
}
