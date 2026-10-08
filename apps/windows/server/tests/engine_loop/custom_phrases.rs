//! 自定义短语在 Windows 壳里的闭环：`AssemblySpec` 里带的 `[[custom_phrases]]` 必须真正装进
//! Engine，敲到该输入码时目标文本是固定位置上的候选。
//!
//! 为什么要有这一条：Windows 壳此前**从没把 `custom_phrases` 传给 Engine**（装配与热加载两条路
//! 都漏了），于是 `config.toml` 里写好的自定义短语、以及官网文档描述的行为，在 Windows 上
//! 一直是空转——配置读进来了、校验也过，但候选里永远不出现。这类「接线漏了」的缺陷不会被
//! Core 的单元测试抓到（Core 侧行为是对的），只有装配这一层能抓住。

use qingjian_core::CustomPhrase;

use super::support::*;

fn phrases() -> Vec<CustomPhrase> {
    vec![
        CustomPhrase {
            code: "dsh".to_owned(),
            text: "dsh-deepseek harness".to_owned(),
            position: 1,
            enabled: true,
        },
        CustomPhrase {
            code: "d".to_owned(),
            text: "deepseek".to_owned(),
            position: 2,
            enabled: true,
        },
    ]
}

/// 装配时带自定义短语的 Router（对照 `support::router` 那条不带短语的路径）。
fn router_with_phrases(config: RouterConfig) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        custom_phrases: phrases(),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    engine.set_shift_letter_compose(config.shift_letter_compose);
    let mut router = Router::new(engine, config);
    open_session(&mut router, SESSION, None);
    router
}

/// 逐个字母敲一串（中文模式，不加修饰键）。
fn type_letters(router: &mut Router, text: &str) -> Frame {
    let mut last = None;
    for c in text.chars() {
        last = Some(press(router, letter(c)).2);
    }
    last.expect("typed at least one letter")
}

#[test]
fn custom_phrases_from_the_assembly_spec_land_in_the_candidates() {
    let mut router = router_with_phrases(RouterConfig::default());

    let frame = type_letters(&mut router, "dsh");
    let texts = candidate_texts(&frame);
    assert_eq!(
        texts.first().copied(),
        Some("dsh-deepseek harness"),
        "敲 dsh 时自定义短语应在第 1 位，实际候选：{texts:?}"
    );
}

#[test]
fn a_phrase_pinned_to_the_second_slot_leaves_the_first_alone() {
    let mut router = router_with_phrases(RouterConfig::default());

    let frame = type_letters(&mut router, "d");
    let texts = candidate_texts(&frame);
    assert_ne!(
        texts.first().copied(),
        Some("deepseek"),
        "position=2 不该占第 1 位，实际候选：{texts:?}"
    );
    assert_eq!(
        texts.get(1).copied(),
        Some("deepseek"),
        "position=2 应落在第 2 位，实际候选：{texts:?}"
    );
}
