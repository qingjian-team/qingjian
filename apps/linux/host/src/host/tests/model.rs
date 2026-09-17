//! 本地整句模型测试：后台装载、防抖重排与热开关。

use super::*;

/// 假打分器：偏爱指定句子，其余打大负分（引擎自己的重排测试同款思路）。
struct Prefers(&'static str);

impl qingjian_core::sentence::SentenceScorer for Prefers {
    fn score(&self, _context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { 0.0 } else { -100.0 })
            .collect()
    }
}

#[test]
fn model_poll_idle_is_inert() {
    // 没接模型：定时问一句立刻歇，不空转。
    let mut h = sample_host();
    assert_eq!(h.model_poll(), 0, "无模型无组句时应返回 0(停定时器)");
    type_str(&mut h, "ni");
    assert_eq!(h.model_poll(), 0, "无模型时组句中也没什么可等");
}

#[test]
fn model_rescoring_requests_after_debounce_and_repaints() {
    // 接上异步打分器 → 敲拼音攒下整句路径 → 防抖到点发请求 → 分回来要求重画。
    let mut h = sample_host();
    h.engine
        .set_async_sentence_scorer(Some(Box::new(Prefers("你好"))));
    type_str(&mut h, "nihao");
    assert!(h.engine.rescoring_pending(), "组句后应有整句路径等着打分");
    assert!(h.rescore_deadline.is_some(), "查询后应起防抖计时");
    // 把防抖截止拨到现在，循环 poll 等后台线程把分送回来。
    h.rescore_deadline = Some(std::time::Instant::now());
    let start = std::time::Instant::now();
    let mut repainted = false;
    while start.elapsed() < std::time::Duration::from_secs(2) {
        if h.model_poll() & 1 != 0 {
            repainted = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(repainted, "重排结果到了应要求重画");
    assert!(h.composing(), "重画只是换排序,组句不受影响");
}

#[test]
fn model_poll_keeps_polling_bit_while_work_pending() {
    // 重画那次返回值也要带「继续定时」位，否则 shim 的一次性定时器带着在途工作停摆。
    let mut h = sample_host();
    h.engine
        .set_async_sentence_scorer(Some(Box::new(Prefers("你好"))));
    type_str(&mut h, "nihao");
    h.rescore_deadline = Some(std::time::Instant::now());
    // 造一个「加载线程还挂着」的在途状态：重画后仍须继续轮询。
    let (_tx, rx) = std::sync::mpsc::channel();
    h.model_loader = Some(rx);
    let start = std::time::Instant::now();
    let mut repaint = None;
    while start.elapsed() < std::time::Duration::from_secs(2) {
        let poll = h.model_poll();
        if poll & 1 != 0 {
            repaint = Some(poll);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let poll = repaint.expect("应有一次重画");
    assert!(
        poll & 0b10 != 0,
        "加载线程仍在途:重画那次也应带继续定时位,实得 {poll:#b}"
    );
    drop(_tx);
}

#[test]
fn model_config_disable_unloads_scorer() {
    // 热加载 [model] enabled = false：卸掉打分器，不再重排。
    let mut h = sample_host();
    h.engine
        .set_async_sentence_scorer(Some(Box::new(Prefers("你好"))));
    assert!(h.engine.has_sentence_scorer());
    let mut config = qingjian_platform::Config::default();
    config.model.enabled = false;
    h.apply_config(&config);
    assert!(!h.engine.has_sentence_scorer(), "关掉配置应卸掉打分器");
}

/// 真数据在才跑（data/generated/dict.qj + data/model/model.qjm，均不进 git）：
/// 模型后台加载完成时，加载窗口里敲出的那一轮也要拿到整句重排。
#[test]
fn model_attached_mid_composition_rescores_that_round() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = repo.join("data/generated/dict.qj");
    let model = repo.join("data/model/model.qjm");
    if !dict.is_file() || !model.is_file() {
        eprintln!("跳过:没有真词库/真模型(CI 上属正常)");
        return;
    }
    let dir = temp_data_dir("lateattach");
    std::fs::remove_file(dir.join("dict.tsv")).unwrap();
    std::os::unix::fs::symlink(&dict, dir.join("dict.qj")).unwrap();
    std::fs::create_dir_all(dir.join("model")).unwrap();
    std::os::unix::fs::symlink(&model, dir.join("model/model.qjm")).unwrap();
    let mut h =
        Host::init(dir, Some(qingjian_platform::Config::default())).expect("真词库应能装配");
    // 模型还在后台加载时就把词敲出来（加载窗口里的那一轮）。
    type_str(&mut h, "nihao");
    assert!(h.model_loader.is_some(), "模型应还在后台加载");
    // 按 shim 的方式轮询：模型接上后，这一轮必须出现重画位（bit0 = 重排换了排序要重画）。
    let started = std::time::Instant::now();
    let mut repainted = false;
    while started.elapsed() < std::time::Duration::from_secs(30) {
        if h.model_poll() & 1 != 0 {
            repainted = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        repainted,
        "模型接上后应补重排并给出重画位,而不是这一轮永远错过"
    );
}
