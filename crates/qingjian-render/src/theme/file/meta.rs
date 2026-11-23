//! 主题的元数据。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Meta {
    /// 主题 id，也是主题目录名。
    pub(crate) id: String,

    /// 显示名。
    pub(crate) name: String,

    #[serde(default)]
    pub(crate) author: String,

    /// SPDX 许可证标识。
    #[serde(default)]
    pub(crate) license: String,

    /// 只有一种外观时写 `"light"` / `"dark"`，不再跟随外观设置切换（背景是浅色图片的主题配深色文字就看不清）。
    pub(crate) appearance: Option<LockedAppearance>,
}

/// 主题锁定的外观。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LockedAppearance {
    Light,

    Dark,
}

#[cfg(test)]
mod tests {
    use super::{LockedAppearance, Meta};

    #[test]
    fn optional_fields_default_to_empty() {
        let meta: Meta = serde_json::from_str(r#"{"id":"a","name":"A"}"#).unwrap();
        assert_eq!((meta.id.as_str(), meta.name.as_str()), ("a", "A"));
        assert_eq!(meta.author, "");
        assert_eq!(meta.license, "");
        assert_eq!(meta.appearance, None);
    }

    #[test]
    fn a_locked_appearance_is_lowercase_only() {
        let meta: Meta =
            serde_json::from_str(r#"{"id":"a","name":"A","appearance":"dark"}"#).unwrap();
        assert_eq!(meta.appearance, Some(LockedAppearance::Dark));
        let light: Meta =
            serde_json::from_str(r#"{"id":"a","name":"A","appearance":"light"}"#).unwrap();
        assert_eq!(light.appearance, Some(LockedAppearance::Light));
        assert!(
            serde_json::from_str::<Meta>(r#"{"id":"a","name":"A","appearance":"Dark"}"#).is_err()
        );
        assert!(serde_json::from_str::<Meta>(r#"{"name":"A"}"#).is_err());
        assert!(serde_json::from_str::<Meta>(r#"{"id":"a"}"#).is_err());
    }
}
