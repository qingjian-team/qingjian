//! 去哪找本地模型：设置里下载的在用户目录，随包的在安装目录。每次加载时现找，下载完不用重启。

use std::path::{Path, PathBuf};

use qingjian_platform::LocalModel;

/// 用户目录与随包根。
#[derive(Debug, Clone, Default)]
pub struct ModelDirs {
    pub user: Option<PathBuf>,
    pub bundled_root: PathBuf,
}

/// 这次要加载的模型。
pub(super) enum ModelFiles {
    /// 含章：`.qjm` 或三件套目录。
    Sentence(PathBuf),

    /// 知意的 int8 ONNX：模型目录与 ONNX Runtime 动态库。
    Zhiyi { dir: PathBuf, runtime: PathBuf },
}

impl ModelDirs {
    /// 选的模型还没下载（或知意缺运行库）为 `None`：不用模型，不再换成另一个。
    pub(super) fn resolve(&self, wanted: LocalModel) -> Option<ModelFiles> {
        match wanted {
            LocalModel::Zhiyi => match (self.zhiyi_dir(), self.onnx_runtime()) {
                (Some(dir), Some(runtime)) => Some(ModelFiles::Zhiyi { dir, runtime }),
                (None, _) => {
                    tracing::warn!("选了知意但没下载，不用模型");
                    None
                }
                (_, None) => {
                    tracing::warn!("找不到 onnxruntime.dll，知意不可用，不用模型");
                    None
                }
            },
            LocalModel::Tongbian => {
                let found = find_model(self.user.as_deref(), &self.bundled_root);
                if found.is_none() {
                    tracing::warn!("选了含章·通变但没下载，不用模型");
                }
                found.map(ModelFiles::Sentence)
            }
        }
    }

    fn zhiyi_dir(&self) -> Option<PathBuf> {
        let dir = self.user.as_deref()?.join("models/zhiyi-onnx");
        dir.join("laya.int8g.onnx").is_file().then_some(dir)
    }

    /// 装机时与 exe 同级，开发时是 `build.ps1` 取下来的那份。
    fn onnx_runtime(&self) -> Option<PathBuf> {
        [
            self.bundled_root.join("onnxruntime.dll"),
            self.bundled_root
                .join("target/installer/onnxruntime/onnxruntime.dll"),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }
}

/// 含章·通变：用户目录（设置里下载的、或自己放的）优先于随包的。含章·知微已下线，它的目录不再找。
pub fn find_model(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    let candidates = [
        user_dir.map(|dir| dir.join("models/hanzhang-tongbian")),
        user_dir.map(|dir| dir.join("model-p2c")),
        Some(bundled_root.join("data/models/hanzhang-tongbian")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find_map(|dir| qingjian_neural::find_model(&dir))
}
