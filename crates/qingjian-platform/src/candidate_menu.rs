//! 候选右键菜单：某个候选能做哪几件事（置顶 / 取消置顶 / 降权 / 取消降权 / 删除），以及做完给用户看的一句话。
//! 各壳只管弹菜单、认点中的项；置顶与取消置顶改的是配置里的自定义短语，壳做完要重读配置。

use std::path::Path;

use qingjian_core::{Candidate, CandidateKind, CustomPhrase, Engine, custom_phrase};

use crate::Config;

/// 菜单里的一项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateAction {
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
}

impl CandidateAction {
    const ALL: [Self; 5] = [
        Self::Pin,
        Self::Unpin,
        Self::Demote,
        Self::Undemote,
        Self::Delete,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pin => "置顶",
            Self::Unpin => "取消置顶",
            Self::Demote => "降权",
            Self::Undemote => "取消降权",
            Self::Delete => "删除",
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
        CandidateAction::Pin | CandidateAction::Unpin => {
            let Some((code, stored)) = engine.pin_target(candidate) else {
                return notice(format!("「{text}」这里不能置顶"));
            };
            let mut list = phrases.to_vec();
            let changed = if action == CandidateAction::Pin {
                custom_phrase::pin(&mut list, &code, &stored)
            } else {
                custom_phrase::unpin(&mut list, &code, &stored)
            };
            if !changed {
                return notice(format!("「{text}」没有要改的"));
            }
            let Some(path) = config_path else {
                return notice("没有配置文件，置顶存不下来".to_owned());
            };
            if let Err(error) = Config::set_custom_phrases(path, &list) {
                tracing::warn!(%error, "置顶写配置失败");
                return notice(format!("没存上：{error}"));
            }
            ActionOutcome {
                notice: if action == CandidateAction::Pin {
                    format!("「{text}」已置顶，输入 {code} 时排第一")
                } else {
                    format!("已取消置顶「{text}」")
                },
                phrases_changed: true,
            }
        }
        CandidateAction::Demote => {
            engine.demote(candidate);
            notice(format!("「{text}」已降权，往后挪了一位"))
        }
        CandidateAction::Undemote => {
            engine.undemote(candidate);
            notice(format!("已取消「{text}」的降权"))
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
        assert_eq!(CandidateAction::from_id(6), None);
    }
}
