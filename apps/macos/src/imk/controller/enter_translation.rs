//! 可选的回车译词入口；只路由按键，译词上屏复用现有路径。

use qingjian_core::{Candidate, Language};
use qingjian_platform::Modifiers;

use super::QingjianInputController;
use crate::host;
use crate::imk::TextClient;

impl QingjianInputController {
    pub(super) fn try_enter_translation(
        &self,
        key: u16,
        pressed: Modifiers,
        client: TextClient<'_>,
    ) -> bool {
        let index = host::with(|h| {
            let index = h.session.highlighted;
            should_commit(
                h.settings.config().shortcut.enter_translation,
                key,
                pressed,
                !h.engine.composition().is_empty(),
                h.session.layout.candidate(index),
            )
            .then_some(index)
        })
        .flatten();
        match index {
            Some(index) => self.commit_translation_at(Some(index), 0, client),
            None => false,
        }
    }
}

fn should_commit(
    enabled: bool,
    key: u16,
    pressed: Modifiers,
    composing: bool,
    candidate: Option<&Candidate>,
) -> bool {
    enabled
        && matches!(key, 36 | 76)
        && pressed.is_empty()
        && composing
        && candidate
            .and_then(|candidate| candidate.translation.as_ref())
            .filter(|translation| translation.language == Language::English)
            .and_then(|translation| translation.senses().first())
            .is_some_and(|sense| !sense.text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::{CandidateKind, Sense, Translation};

    fn candidate(language: Language, text: &str) -> Candidate {
        Candidate {
            text: "开发".into(),
            kind: CandidateKind::Chinese,
            syllables: vec!["kai".into(), "fa".into()],
            reading: None,
            translation: Some(Translation::new(
                language,
                vec![Sense {
                    part_of_speech: None,
                    text: text.into(),
                    reading: None,
                    fresh: false,
                }],
            )),
            aux_code: None,
        }
    }

    #[test]
    fn both_return_keys_use_the_highlighted_english_translation() {
        let candidate = candidate(Language::English, "develop");
        for key in [36, 76] {
            assert!(should_commit(
                true,
                key,
                Modifiers::default(),
                true,
                Some(&candidate)
            ));
        }
    }

    #[test]
    fn disabled_empty_or_non_english_translation_preserves_original_return() {
        let mut candidate = candidate(Language::English, "develop");
        assert!(!should_commit(
            false,
            36,
            Modifiers::default(),
            true,
            Some(&candidate)
        ));
        assert!(!should_commit(true, 36, Modifiers::default(), true, None));
        candidate.translation = None;
        assert!(!should_commit(
            true,
            36,
            Modifiers::default(),
            true,
            Some(&candidate)
        ));
        for (language, text) in [
            (Language::English, "  "),
            (Language::Chinese, "开发"),
            (Language::Japanese, "開発"),
        ] {
            candidate = super::tests::candidate(language, text);
            assert!(!should_commit(
                true,
                36,
                Modifiers::default(),
                true,
                Some(&candidate)
            ));
        }
        candidate.translation = Some(Translation::new(Language::English, vec![]));
        assert!(!should_commit(
            true,
            36,
            Modifiers::default(),
            true,
            Some(&candidate)
        ));
    }

    #[test]
    fn space_tab_page_keys_modifiers_and_idle_return_are_untouched() {
        let candidate = candidate(Language::English, "develop");
        for key in [49, 48, 27, 24] {
            assert!(!should_commit(
                true,
                key,
                Modifiers::default(),
                true,
                Some(&candidate)
            ));
        }
        assert!(!should_commit(
            true,
            36,
            Modifiers::default(),
            false,
            Some(&candidate)
        ));
        for pressed in [
            Modifiers::SHIFT,
            Modifiers::CONTROL,
            Modifiers::OPTION,
            Modifiers {
                command: true,
                ..Modifiers::default()
            },
        ] {
            assert!(!should_commit(true, 36, pressed, true, Some(&candidate)));
        }
    }
}
