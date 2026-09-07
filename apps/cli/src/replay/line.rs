use qingjian_core::InputLogEntry;
use serde::Deserialize;

/// 日志文件里的一行：时间戳 + 条目。与 `qingjian-learning` 写出的格式对应。
#[derive(Debug, Deserialize)]
pub struct Line {
    /// 本机时间戳，回放不用。
    #[serde(default)]
    #[allow(dead_code)]
    pub t: String,

    #[serde(flatten)]
    pub entry: InputLogEntry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::InputSource;

    #[test]
    fn parses_commit_and_retract_lines() {
        let commit = r#"{"t":"2026-09-05T20:00:00+08:00","event":"commit","id":1,"scope":"nihao","keys":"nihao","pinyin":"ni'hao","corrected":false,"text":"你好","source":"word","index":0,"top":["你好"],"scheme":"","english":false}"#;
        let line: Line = serde_json::from_str(commit).unwrap();
        let InputLogEntry::Commit(entry) = line.entry else {
            panic!("expected commit");
        };
        assert_eq!(entry.source, InputSource::Word);
        assert_eq!(entry.scope, "nihao");
        let retract = r#"{"t":"2026-09-05T20:00:01+08:00","event":"retract","of":1,"text":"你好","chosen":"拟好"}"#;
        let line: Line = serde_json::from_str(retract).unwrap();
        assert!(matches!(line.entry, InputLogEntry::Retract { of: 1, .. }));
    }
}
