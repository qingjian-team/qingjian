//! 显式选区翻译的进程内实现。下载器在设置程序，推理线程没有网络依赖。

mod asset;
mod backend;
mod catalog;
mod error;
mod generation;
mod package;
mod predictor;
mod tokenization;
mod worker;

pub use asset::Asset;
pub use backend::Backend;
pub use catalog::{MODEL_DIR, installed, packages};
pub use error::OfflineError;
pub use package::Package;
pub use predictor::NativePredictor;
