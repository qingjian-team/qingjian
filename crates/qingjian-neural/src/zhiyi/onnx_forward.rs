//! int8 ONNX 版，ONNX Runtime 跑（Windows 用）：运行库在运行时按路径加载，只用 CPU。
//! 试过 DirectML：每次序列长度都不同，集显（Radeon 880M）上一次 200 ms，比 CPU 两线程（30 ms）慢得多，见 docs/notes/crate-notes.md。

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;

use super::forward::Forward;
use super::input::Sequence;
use crate::NeuralError;

const MODEL_FILE: &str = "laya.int8g.onnx";

pub(super) struct OnnxForward {
    /// `Session::run` 要 `&mut`；打分本来就一次一条。
    session: Mutex<Session>,
}

impl OnnxForward {
    pub fn load(dir: &Path, runtime: &Path, threads: usize) -> Result<Self, NeuralError> {
        load_runtime(runtime)?;
        let session = Session::builder()
            .and_then(|builder| {
                builder
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(Into::into)
            })
            .and_then(|builder| builder.with_intra_threads(threads).map_err(Into::into))
            .and_then(|mut builder| builder.commit_from_file(dir.join(MODEL_FILE)))
            .map_err(onnx_error)?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }
}

impl Forward for OnnxForward {
    fn logits(&self, sequence: &Sequence) -> Result<Vec<f32>, NeuralError> {
        let row = |values: Vec<i64>| Tensor::from_array(([1, values.len()], values));
        let ints = |values: &[u32]| values.iter().map(|&v| i64::from(v)).collect::<Vec<_>>();
        let slots: Vec<i64> = (0..sequence.units.len() as i64)
            .filter(|&i| sequence.units[i as usize] != 0)
            .collect();
        let markers = sequence.markers.len();
        let inputs = ort::inputs![
            "input_ids" => row(ints(&sequence.ids)).map_err(onnx_error)?,
            "attention_mask" => row(vec![1; sequence.ids.len()]).map_err(onnx_error)?,
            "unit_ids" => row(ints(&sequence.units)).map_err(onnx_error)?,
            "marker_pos" => row(ints(&sequence.markers)).map_err(onnx_error)?,
            "marker_mask" => Tensor::from_array(([1, markers], vec![true; markers])).map_err(onnx_error)?,
            "slot_pos" => row(slots).map_err(onnx_error)?,
        ];
        let mut session = self
            .session
            .lock()
            .map_err(|_| NeuralError::Corrupt("ONNX session poisoned"))?;
        let outputs = session.run(inputs).map_err(onnx_error)?;
        let (_, logits) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(onnx_error)?;
        Ok(logits.to_vec())
    }
}

/// 整个进程只加载一次运行库；第二次起用第一次的结果（换路径也不重载）。
fn load_runtime(path: &Path) -> Result<(), NeuralError> {
    static LOADED: OnceLock<Result<(), String>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            ort::init_from(path)
                .map(|builder| {
                    // ONNX Runtime 的 Windows 版缺省开着遥测（ETW），关掉
                    builder.with_name("qingjian").with_telemetry(false).commit();
                })
                .map_err(|error| format!("{}: {error}", path.display()))
        })
        .clone()
        .map_err(NeuralError::Onnx)
}

fn onnx_error(error: impl std::fmt::Display) -> NeuralError {
    NeuralError::Onnx(error.to_string())
}
