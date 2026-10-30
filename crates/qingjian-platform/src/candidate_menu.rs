//! 候选右键菜单：某个候选能做哪几件事，以及做完给用户看的一句话。
//! 各壳只管弹菜单、认点中的项；置顶与取消置顶改的是配置里的自定义短语，壳做完要重读配置。

use std::path::Path;

use qingjian_core::{Candidate, CandidateKind, CustomPhrase, Engine, Language, custom_phrase};

use crate::Config;
use crate::i18n::{tr, tr_args};

/// 菜单里的一项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateAction {
    /// 用户输入并覆盖当前学习语言的译文。
    EditTranslation,

    /// 请云模型按当前学习语言重新生成译文。
    RegenerateTranslation,

    /// 在当前输入码下排第一：写一条权重最高的自定义短语。
    Pin,

    /// 删掉当前输入码下这条自定义短语。
    Unpin,

    /// 在当前输入码下往后挪一格（记进学习数据）。
    Demote,

    /// 去掉当前输入码下对它的降权。
    Undemote,

    /// 用户词整个删掉，词库词清掉对它的学习记录。
    Delete,

    /// 提高平台自有候选的选择权重；普通中文候选不展示，由平台候选上下文使用。
    Promote,
}

/// 不经过中文 `Composition` 的输入法候选上下文。
///
/// 越南语、将来的日语/韩语等只需提供稳定命名空间、当前输入码和译词目标语言，
/// 即可复用候选排序学习和右键菜单；命名空间会隔离各输入法及中文拼音的学习数据。
#[derive(Debug, Clone, Copy)]
pub struct IndependentInputContext<'a> {
    pub namespace: &'a str,
    pub input: &'a str,
    pub translation_language: Language,
}

impl IndependentInputContext<'_> {
    pub fn learning_key(self) -> String {
        format!("{}:{}", self.namespace, self.input)
    }
}

impl CandidateAction {
    const ALL: [Self; 8] = [
        Self::EditTranslation,
        Self::RegenerateTranslation,
        Self::Pin,
        Self::Unpin,
        Self::Demote,
        Self::Undemote,
        Self::Delete,
        Self::Promote,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::EditTranslation => tr("candidate_menu.edit_translation"),
            Self::RegenerateTranslation => tr("candidate_menu.regenerate_translation"),
            Self::Pin => tr("candidate_menu.pin"),
            Self::Unpin => tr("candidate_menu.unpin"),
            Self::Demote => tr("candidate_menu.demote"),
            Self::Undemote => tr("candidate_menu.undemote"),
            Self::Delete => tr("candidate_menu.delete"),
            Self::Promote => tr("candidate_menu.promote"),
        }
    }

    /// 菜单项的编号（从 1 起，0 留给「什么都没点」）。
    pub fn id(self) -> u32 {
        Self::ALL.iter().position(|a| *a == self).unwrap_or(0) as u32 + 1
    }

    pub fn from_id(id: u32) -> Option<Self> {
        Self::ALL.get(id.checked_sub(1)? as usize).copied()
    }
}

/// 这个候选在菜单里给哪几项；`phrases` 是配置文件里的自定义短语（不含并进来的系统文本替换）。
pub fn actions(
    engine: &Engine,
    candidate: &Candidate,
    phrases: &[CustomPhrase],
) -> Vec<CandidateAction> {
    let mut actions = Vec::new();
    if engine.translation_target(candidate).is_some() {
        actions.push(CandidateAction::EditTranslation);
        if engine.prediction_enabled() {
            actions.push(CandidateAction::RegenerateTranslation);
        }
    }
    if let Some((code, text)) = engine.pin_target(candidate) {
        let mut pinned = phrases.to_vec();
        if custom_phrase::pin(&mut pinned, &code, &text) {
            actions.push(CandidateAction::Pin);
        }
        if matches!(candidate.kind, CandidateKind::Custom(_))
            && phrases.iter().any(|p| p.code == code && p.text == text)
        {
            actions.push(CandidateAction::Unpin);
        }
    }
    if engine.demotable(candidate) {
        actions.push(CandidateAction::Demote);
        if engine.demotion(candidate) > 0 {
            actions.push(CandidateAction::Undemote);
        }
    }
    if candidate.kind.forgettable() {
        actions.push(CandidateAction::Delete);
    }
    actions
}

/// 独立输入法候选的公共菜单项。它不依赖中文拼音 Composition，也不提供中文自定义短语和删除用户词。
pub fn independent_actions(
    engine: &Engine,
    candidate: &Candidate,
    context: IndependentInputContext<'_>,
) -> Vec<CandidateAction> {
    let key = context.learning_key();
    let mut actions = vec![CandidateAction::Promote, CandidateAction::Demote];
    if engine.external_candidate_demotion(&key, &candidate.text) > 0 {
        actions.push(CandidateAction::Undemote);
    }
    if engine.prediction_enabled() {
        actions.push(CandidateAction::RegenerateTranslation);
    }
    actions
}

/// 独立输入法菜单操作的结果。`translation_request` 有值时，壳应向模型请求该目标语言的译词。
pub struct IndependentActionOutcome {
    pub notice: String,
    pub translation_request: Option<Language>,
}

pub fn perform_independent(
    action: CandidateAction,
    engine: &mut Engine,
    candidate: &Candidate,
    context: IndependentInputContext<'_>,
) -> Option<IndependentActionOutcome> {
    let key = context.learning_key();
    let text = &candidate.text;
    let (notice, translation_request) = match action {
        CandidateAction::Promote => {
            engine.promote_external_candidate(&key, text);
            (format!("「{text}」已提权"), None)
        }
        CandidateAction::Demote => {
            engine.demote_external_candidate(&key, text);
            (format!("「{text}」已降权"), None)
        }
        CandidateAction::Undemote => {
            engine.undemote_external_candidate(&key, text);
            (format!("已取消「{text}」的降权"), None)
        }
        CandidateAction::RegenerateTranslation => (
            "模型正在重新生成译词…".to_owned(),
            Some(context.translation_language),
        ),
        _ => return None,
    };
    Some(IndependentActionOutcome {
        notice,
        translation_request,
    })
}

/// 菜单项做完的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    /// 给用户看的一句话（显示在拼音行旁）。
    pub notice: String,

    /// 配置文件里的自定义短语改了，壳要重读配置把它推给 Engine。
    pub phrases_changed: bool,
}

/// 做菜单里的一项。置顶 / 取消置顶写配置文件 `config_path`，失败的原因放进提示。
pub fn perform(
    action: CandidateAction,
    engine: &mut Engine,
    candidate: &Candidate,
    config_path: Option<&Path>,
    phrases: &[CustomPhrase],
) -> ActionOutcome {
    let text = &candidate.text;
    let notice = |notice: String| ActionOutcome {
        notice,
        phrases_changed: false,
    };
    match action {
        CandidateAction::EditTranslation
        | CandidateAction::RegenerateTranslation
        | CandidateAction::Promote => notice(tr("candidate_menu.needs_window").to_owned()),
        CandidateAction::Pin | CandidateAction::Unpin => {
            let Some((code, stored)) = engine.pin_target(candidate) else {
                return notice(tr_args("candidate_menu.cant_pin", &[("text", &text)]));
            };
            let mut list = phrases.to_vec();
            let changed = if action == CandidateAction::Pin {
                custom_phrase::pin(&mut list, &code, &stored)
            } else {
                custom_phrase::unpin(&mut list, &code, &stored)
            };
            if !changed {
                return notice(tr_args("candidate_menu.unchanged", &[("text", &text)]));
            }
            let Some(path) = config_path else {
                return notice(tr("candidate_menu.no_config").to_owned());
            };
            if let Err(error) = Config::set_custom_phrases(path, &list) {
                tracing::warn!(%error, "置顶写配置失败");
                return notice(tr_args("candidate_menu.not_saved", &[("error", &error)]));
            }
            ActionOutcome {
                notice: if action == CandidateAction::Pin {
                    tr_args("candidate_menu.pinned", &[("text", &text), ("code", &code)])
                } else {
                    tr_args("candidate_menu.unpinned", &[("text", &text)])
                },
                phrases_changed: true,
            }
        }
        CandidateAction::Demote => {
            engine.demote(candidate);
            notice(tr_args("candidate_menu.demoted", &[("text", &text)]))
        }
        CandidateAction::Undemote => {
            engine.undemote(candidate);
            notice(tr_args("candidate_menu.undemoted", &[("text", &text)]))
        }
        CandidateAction::Delete => notice(engine.forget(candidate).describe(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for action in CandidateAction::ALL {
            assert_eq!(CandidateAction::from_id(action.id()), Some(action));
        }
        assert_eq!(CandidateAction::from_id(0), None);
        assert_eq!(CandidateAction::from_id(9), None);
    }

    #[test]
    fn independent_input_namespaces_isolate_learning_keys() {
        let vietnamese = IndependentInputContext {
            namespace: "vi",
            input: "tieengs",
            translation_language: Language::Chinese,
        };
        let japanese = IndependentInputContext {
            namespace: "ja-romaji",
            input: "nihon",
            translation_language: Language::Chinese,
        };
        assert_eq!(vietnamese.learning_key(), "vi:tieengs");
        assert_eq!(japanese.learning_key(), "ja-romaji:nihon");
    }
}
