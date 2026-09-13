//! 偏好设置的各页：一页一个文件，各自建控件（`build`）、按配置刷新（`sync`）。
//! 窗口本体（`window.rs`）只负责把页装进标签视图和底部状态行。

mod about;
mod advanced;
mod candidates;
mod cloud;
mod dictionaries;
mod fuzzy;
mod general;
mod phrases;
mod shortcuts;
mod usage;

pub(super) use about::build as build_about;
pub use about::{REPOSITORY_URL, WEBSITE_URL};
pub(super) use advanced::AdvancedPage;
pub(super) use candidates::CandidatesPage;
pub(super) use cloud::CloudPage;
pub(super) use dictionaries::DictionariesPage;
pub(super) use fuzzy::FuzzyPage;
pub(super) use general::GeneralPage;
pub(super) use phrases::PhrasesPage;
pub(super) use shortcuts::ShortcutsPage;
pub(super) use usage::UsagePage;
