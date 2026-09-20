//! 回放评测：把输入日志里每次上屏时的键重新喂给引擎，看现在的排序会不会把当时选的词放在首选。
//!
//! 只在内存里学习（CLI 没给 `--user-dict` 就是内存学习器），不写任何文件；按日志顺序回放，
//! 命中的候选照样上屏，让上文（个人 n-gram、上一个词）跟真实使用一样往前走。
//! 云端词、云端整句、原样上屏、译词不评：它们不是本地排序的结果，只计数。
//! 其他事件（重打、直通、云端联想、上文断开、会话）按 `docs/plan/model-eval.md` 里的尺子计数。

mod line;
mod report;
mod scheme;
mod tally;

use std::path::Path;

use qingjian_core::{Engine, InputLogEntry, InputSource};
use qingjian_dictionary::CodeTable;

pub use report::Report;

use line::Line;
use scheme::SchemeSwitcher;
use tally::Tally;

/// 跑一遍日志，返回报告。`code_table` 是给日志里形码那些行用的（`--wubi` 给的码表）。
pub fn run(
    engine: &mut Engine,
    path: &Path,
    show_misses: usize,
    code_table: Option<CodeTable>,
) -> Result<Report, ReplayError> {
    let text = std::fs::read_to_string(path).map_err(|source| ReplayError::Read {
        path: path.to_owned(),
        source,
    })?;
    let mut report = Report::default();
    let mut switcher = SchemeSwitcher::new(code_table);
    for (number, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line: Line = match serde_json::from_str(raw) {
            Ok(line) => line,
            Err(error) => {
                report.unparsable += 1;
                tracing::warn!(line = number + 1, %error, "日志行解析失败，跳过");
                continue;
            }
        };
        match line.entry {
            InputLogEntry::Retract { .. } => report.retracts += 1,
            InputLogEntry::Retype { .. } => report.retypes += 1,
            InputLogEntry::Session { .. } => report.sessions += 1,
            InputLogEntry::Break { .. } => {
                report.breaks += 1;
                engine.break_chain();
            }
            InputLogEntry::Passthrough { text } => {
                // 直通字符也进标点状态与上文链，与真实使用一致
                report.passthrough_chars += text.chars().count();
                for c in text.chars() {
                    engine.note_passthrough(c);
                }
            }
            InputLogEntry::Prediction { .. } => {
                report.predictions += 1;
                report.prediction_pending = true;
            }
            InputLogEntry::Commit(commit) => {
                // 2026-09-12 以前的日志里壳每次回车 / 失焦都记一条空的原样上屏，不算数
                if commit.keys.is_empty() && commit.text.is_empty() {
                    // 那一条对应的是一次回车 / 失焦：上文链照样断，只是不计数
                    report.empty += 1;
                    engine.clear();
                    engine.break_chain();
                    continue;
                }
                if std::mem::take(&mut report.prediction_pending)
                    && matches!(
                        commit.source,
                        InputSource::Cloud | InputSource::CloudSentence
                    )
                {
                    report.predictions_accepted += 1;
                }
                replay_commit(engine, &mut switcher, &commit, &mut report, show_misses)
            }
        }
    }
    Ok(report)
}

fn replay_commit(
    engine: &mut Engine,
    switcher: &mut SchemeSwitcher,
    commit: &qingjian_core::CommitEntry,
    report: &mut Report,
    show_misses: usize,
) {
    // 这条日志所属方案回放不了（形码但没给 `--wubi`）：只计数。拿拼音的读法去喂形码的键会算出
    // 看着像真的、实则无意义的命中率。
    if !switcher.can_replay(&commit.scheme) {
        report.skip(commit.source);
        engine.clear();
        engine.break_chain();
        return;
    }
    switcher.apply(engine, &commit.scheme);
    let Some(tally) = report.tally_for(commit.source) else {
        // 不是本地排序给出的（云端词、原样上屏……）：只计数；那次上屏的词没法接进上文，断链。
        // 原样上屏照样走一遍 `take_raw`：个人英文词（`gist`）与「这个串不纠」都是从这里学的，不走它回放里的英文候选与纠错就比真实使用差
        report.skip(commit.source);
        if commit.source == InputSource::Raw && !commit.keys.is_empty() {
            engine.set_english_mode(commit.english);
            feed(engine, &commit.keys);
            engine.take_raw();
        }
        engine.clear();
        engine.break_chain();
        return;
    };
    let tally: &mut Tally = tally;
    tally.total += 1;
    tally.note_logged(commit);
    let scope = if commit.scope.is_empty() {
        commit.keys.as_str()
    } else {
        commit.scope.as_str()
    };
    engine.set_english_mode(commit.english);
    // 日志里记的是实际敲键：含触发键的按壳那样逐键重喂（进辅码态、码段进码段），
    // 否则按当时的作用域喂。进过辅码态时留下拼音段，上屏后拿它量学习闭环。
    let pinyin = if commit.keys.contains(engine.aux_code_key()) {
        let (aux, pinyin) = feed(engine, &commit.keys);
        aux.then_some(pinyin)
    } else {
        engine.set_input(scope);
        None
    };
    let query = match engine.query() {
        Ok(query) => query,
        Err(_) => {
            tally.unparsable += 1;
            engine.clear();
            return;
        }
    };
    let position = query
        .candidates
        .items
        .iter()
        .position(|c| c.text == commit.text);
    match position {
        Some(0) => tally.top1 += 1,
        Some(index) if index < 5 => tally.top5 += 1,
        Some(_) => tally.found_later += 1,
        None => tally.missing += 1,
    }
    if let Some(index) = position {
        tally.rank_sum += index + 1;
    }
    if commit.corrected {
        tally.corrected_then += 1;
        if query.correction.is_some() {
            tally.corrected_now += 1;
        }
    }
    if position != Some(0) && report.misses.len() < show_misses {
        let top: Vec<String> = query
            .candidates
            .items
            .iter()
            .take(3)
            .map(|c| c.text.clone())
            .collect();
        report.misses.push(format!(
            "[{}] {scope:<16} 选了 {:<8} 现在前三 {}{}",
            source_label(commit.source),
            commit.text,
            top.join(" / "),
            position.map_or(String::from("（不在候选里）"), |i| format!(
                "（第 {} 位）",
                i + 1
            )),
        ));
    }
    // 照着当时的选择上屏，让上下文往前走；不在候选里就原样清掉
    match position.map(|i| query.candidates.items[i].clone()) {
        Some(candidate) => {
            engine.commit(&candidate);
            engine.clear();
        }
        None => engine.clear(),
    }
    // 辅码选词的闭环：学习记完之后，同样的拼音改成纯拼音输入，这个词该排到首选
    if let Some(pinyin) = pinyin {
        report.aux_total += 1;
        engine.set_input(&pinyin);
        let top1 = engine
            .query()
            .ok()
            .and_then(|query| query.candidates.items.first().map(|c| c.text.clone()));
        if top1.as_deref() == Some(commit.text.as_str()) {
            report.aux_top1 += 1;
        }
        engine.clear();
    }
}

/// 与壳一样逐个喂键：触发键进辅码态、之后的字母进码段，其余进拼音缓冲区。
/// 返回 (是否进过辅码态, 进辅码态那一刻的拼音缓冲区)。
fn feed(engine: &mut Engine, keys: &str) -> (bool, String) {
    engine.clear();
    let mut aux = false;
    let mut pinyin = String::new();
    for key in keys.chars() {
        if engine.aux_trigger(key) {
            engine.enter_aux();
            aux = true;
            pinyin = engine.composition().text().to_owned();
        } else if engine.in_aux() {
            if !engine.push_aux_code(key) {
                engine.clear_aux();
                engine.push(key);
            }
        } else {
            engine.push(key);
        }
    }
    (aux, pinyin)
}

/// 回放的错误。
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("cannot read input log {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// 没命中例子里标来源用的短名，与报告各计数板同名。
fn source_label(source: InputSource) -> &'static str {
    match source {
        InputSource::Word => "词",
        InputSource::Sentence => "整句",
        InputSource::English => "英文",
        _ => "其他",
    }
}
