//! 剪贴板历史里的一条：文字或图片。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardEntry {
    /// 复制的文字，原样（图片条目为空）。
    #[serde(default)]
    pub text: String,

    /// 图片在 `clipboard-images/` 里的文件名；文字条目为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// 最近一次复制的 Unix 秒。
    pub at: i64,
}

impl ClipboardEntry {
    pub fn is_image(&self) -> bool {
        self.image.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::ClipboardEntry;

    #[test]
    fn only_entries_with_a_file_name_are_images() {
        let text = ClipboardEntry {
            text: "文字".to_owned(),
            image: None,
            at: 1,
        };
        assert!(!text.is_image());
        let image = ClipboardEntry {
            text: String::new(),
            image: Some("a.png".to_owned()),
            at: 2,
        };
        assert!(image.is_image());
    }

    #[test]
    fn missing_optional_fields_deserialize() {
        let entry: ClipboardEntry = serde_json::from_str(r#"{"at": 3}"#).unwrap();
        assert_eq!(entry.text, "");
        assert!(entry.image.is_none());
        assert_eq!(entry.at, 3);
    }

    #[test]
    fn text_entries_skip_the_null_image_field() {
        let entry = ClipboardEntry {
            text: String::new(),
            image: None,
            at: 4,
        };
        let text = serde_json::to_string(&entry).unwrap();
        assert!(!text.contains("image"), "{text}");
    }
}
