//! 对拍模型包附带的测试向量（fp32 ONNX 的输出）。要设 `QINGJIAN_ZHIYI_DIR` 指向解压的模型目录，没设就跳过。

use std::collections::HashMap;
use std::path::PathBuf;

use candle_core::{Device, Tensor};

use super::ChoiceScorer;

#[test]
fn matches_the_reference_vectors() {
    let Some(dir) = std::env::var_os("QINGJIAN_ZHIYI_DIR").map(PathBuf::from) else {
        eprintln!("没设 QINGJIAN_ZHIYI_DIR，跳过");
        return;
    };
    let scorer = ChoiceScorer::load(&dir).unwrap();
    let vectors: HashMap<String, Tensor> =
        candle_core::safetensors::load(dir.join("test_vectors.safetensors"), &Device::Cpu).unwrap();
    let ints = |name: String| -> Vec<u32> {
        vectors[&name]
            .to_vec1::<i64>()
            .unwrap()
            .into_iter()
            .map(|v| v as u32)
            .collect()
    };
    for case in 0..5 {
        let expected: Vec<f32> = vectors[&format!("{case}.logits")].to_vec1().unwrap();
        let got = scorer
            .score_ids(
                &ints(format!("{case}.input_ids")),
                &ints(format!("{case}.unit_ids")),
                &ints(format!("{case}.marker_pos")),
            )
            .unwrap();
        eprintln!("case {case}: expected {expected:?} got {got:?}");
        let best = |v: &[f32]| {
            v.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
        };
        assert_eq!(best(&expected), best(&got), "case {case}");
        for (e, g) in expected.iter().zip(&got) {
            assert!((e - g).abs() < 0.5, "case {case}: {expected:?} vs {got:?}");
        }
    }
}

/// ONNX 版与 candle 版对拍：同样的输入首选一致、分差不大。要设 `QINGJIAN_ZHIYI_DIR`（safetensors 版）、
/// `QINGJIAN_ZHIYI_ONNX_DIR`（ONNX 版）与 `QINGJIAN_ONNXRUNTIME`（运行库路径），缺一个就跳过。
#[cfg(feature = "zhiyi-onnx")]
#[test]
fn onnx_agrees_with_candle() {
    let var = |name| std::env::var_os(name).map(PathBuf::from);
    let (Some(dir), Some(onnx_dir), Some(runtime)) = (
        var("QINGJIAN_ZHIYI_DIR"),
        var("QINGJIAN_ZHIYI_ONNX_DIR"),
        var("QINGJIAN_ONNXRUNTIME"),
    ) else {
        eprintln!("没设知意的两个模型目录与运行库，跳过");
        return;
    };
    let candle = ChoiceScorer::load(&dir).unwrap();
    let onnx = ChoiceScorer::load_onnx(&onnx_dir, &runtime, 2).unwrap();
    let cases: [(&str, &str, &[&str], &[&str]); 4] = [
        (
            "今天天气",
            "henhao",
            &["hen", "hao"],
            &["很好", "很号", "恨好"],
        ),
        ("", "shi", &["shi"], &["是", "时", "事", "十", "使"]),
        (
            "我们一起去",
            "chifan",
            &["chi", "fan"],
            &["吃饭", "迟饭", "池帆"],
        ),
        ("He said", "hello", &[], &["hello", "help", "held"]),
    ];
    for (context, keys, syllables, candidates) in cases {
        let syllables: Vec<String> = syllables.iter().map(|s| (*s).to_owned()).collect();
        let typed = |scorer: &ChoiceScorer| {
            scorer
                .align(keys, &syllables)
                .unwrap_or_else(|| scorer.letter_units(keys))
        };
        let a = candle.score(context, &typed(&candle), candidates).unwrap();
        let b = onnx.score(context, &typed(&onnx), candidates).unwrap();
        eprintln!("{keys}: candle {a:?} onnx {b:?}");
        let best = |v: &[f32]| {
            v.iter()
                .enumerate()
                .max_by(|x, y| x.1.total_cmp(y.1))
                .map(|(i, _)| i)
        };
        assert_eq!(best(&a), best(&b), "{keys}");
        assert!((a[0] - b[0]).abs() < 1.0, "{keys}: {a:?} vs {b:?}");
    }
}
