//! 短句缓存按语言和原文分别索引；只返回当前请求的一种目标语言。
use qingjian_core::{PredictionKind, PredictionRequest, SentenceTranslation};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(crate) struct PhraseStore {
    entries: BTreeMap<String, String>,

    path: Option<PathBuf>,
}

fn key(language: &str, source: &str) -> String {
    format!("{language}\u{1}{source}")
}

impl PhraseStore {
    pub(crate) fn load(namespace: u64) -> Self {
        let path = std::env::var_os("APPDATA").map(|p| {
            PathBuf::from(p)
                .join("Qingjian")
                .join(format!("sentence-cache-{namespace:016x}.json"))
        });
        let mut entries: BTreeMap<String, String> = path
            .as_ref()
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() <= 2_000_000))
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        entries.retain(|k, v| {
            k.chars().count() <= 260 && !v.trim().is_empty() && v.chars().count() <= 1024
        });
        for (source, en, ja) in [
            ("集合", "Group up.", "集合しよう。"),
            ("撤退", "Fall back.", "撤退しよう。"),
            ("小心左边", "Watch the left.", "左に気をつけて。"),
            ("我没子弹了", "I'm out of ammo.", "弾が切れた。"),
            ("跟着我", "Follow me.", "ついてきて。"),
            ("需要支援", "Need backup.", "援護が必要。"),
            ("等一下", "Wait a moment.", "ちょっと待って。"),
            ("谢谢", "Thank you.", "ありがとう。"),
            ("你好", "Hello.", "こんにちは。"),
            ("对不起", "Sorry.", "ごめんなさい。"),
            ("干得漂亮", "Well played!", "ナイス！"),
            (
                "我们一起去B点",
                "Let's go to B together.",
                "一緒にB地点へ行こう。",
            ),
            (
                "我们一起去A点",
                "Let's go to A together.",
                "一緒にA地点へ行こう。",
            ),
            ("敌人在右边", "Enemy on the right.", "右に敵がいる。"),
            ("我需要治疗", "I need healing.", "回復が必要。"),
            ("准备好了吗", "Are you ready?", "準備はいい？"),
        ] {
            entries
                .entry(key("en", source))
                .or_insert_with(|| en.to_owned());
            entries
                .entry(key("ja", source))
                .or_insert_with(|| ja.to_owned());
        }

        Self { entries, path }
    }

    pub(crate) fn lookup(&self, request: &PredictionRequest) -> Option<SentenceTranslation> {
        if request.kind != PredictionKind::Compose {
            return None;
        }
        let source = request.translation_source.as_ref()?;
        let text = self.entries.get(&key(&request.target_language, source))?;
        Some(SentenceTranslation {
            source: source.clone(),
            language: request.target_language.parse().ok()?,
            text: text.clone(),
        })
    }

    pub(crate) fn remember(&mut self, translation: &Option<SentenceTranslation>) {
        let Some(t) = translation
            .as_ref()
            .filter(|t| t.source.chars().count() <= 256 && t.text.chars().count() <= 1024)
        else {
            return;
        };
        self.entries
            .insert(key(t.language.code(), &t.source), t.text.clone());
        while self.entries.len() > 512 {
            self.entries.pop_first();
        }
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let tmp = path.with_extension(format!("json-{}.tmp", std::process::id()));
            if let Ok(bytes) = serde_json::to_vec(&self.entries)
                && std::fs::write(&tmp, bytes).is_ok()
            {
                let _ = std::fs::rename(tmp, path);
            }
        }
    }
}
