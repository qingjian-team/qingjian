//! 云服务页的本机模型：选或填 GGUF 模型文件与 llama-server 程序的路径。清空照存：模型清空就是不用本机模型，
//! 程序清空就是让输入法自己找。起停服务是输入法的事（`qingjian_predict` 的本机模型），这里只写配置。

use std::path::Path;

use super::Settings;
use crate::i18n::{tr, tr_args};
use crate::message::FieldId;

impl Settings {
    /// 「本机模型」的「选择…」。
    pub(super) fn pick_local_model(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_title(tr("cloud.local_model_pick"))
            .add_filter(tr("cloud.local_model_files"), &["gguf"]);
        if let Some(dir) = parent(&self.config.predict.local_model) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.commit_local_path(FieldId::LocalModel, &path.to_string_lossy());
            self.reset_field(FieldId::LocalModel);
        }
    }

    /// 「llama-server」的「选择…」。Windows 只列 .exe；macOS 与 Linux 的程序没有扩展名，不设筛选。
    pub(super) fn pick_local_server(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(tr("cloud.local_server_pick"));
        if cfg!(windows) {
            dialog = dialog.add_filter(tr("cloud.local_server_programs"), &["exe"]);
        }
        if let Some(dir) = parent(&self.config.predict.local_server) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.commit_local_path(FieldId::LocalServer, &path.to_string_lossy());
            self.reset_field(FieldId::LocalServer);
        }
    }

    /// 存一个路径：没改的不存，清空照存；文件不在、模型不是 .gguf 就在这一行下面说一句，不存。
    /// 资源管理器「复制文件地址」带的引号去掉。
    pub(super) fn commit_local_path(&mut self, id: FieldId, text: &str) {
        let path = text.trim().trim_matches(['"', '\'']).trim();
        let predict = &self.config.predict;
        let current = match id {
            FieldId::LocalModel => &predict.local_model,
            _ => &predict.local_server,
        };
        if path == current.trim() {
            return;
        }
        if !path.is_empty() && !Path::new(path).is_file() {
            self.notice = Some((id.key(), tr_args("cloud.local_missing", &[("path", &path)])));
            return;
        }
        let model = id == FieldId::LocalModel;
        if model && !path.is_empty() && !is_gguf(path) {
            self.notice = Some((id.key(), tr("cloud.local_not_gguf").to_owned()));
            return;
        }
        self.write("predict", id.key(), path.to_owned());
        if model && !path.is_empty() {
            self.notice = Some((id.key(), tr("cloud.local_model_saved").to_owned()));
        }
    }
}

/// 已填路径所在的目录，选文件时从那里开始；没填或目录不在时 `None`（对话框用系统记住的位置）。
fn parent(path: &str) -> Option<&Path> {
    Path::new(path.trim()).parent().filter(|dir| dir.is_dir())
}

fn is_gguf(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
}

#[cfg(test)]
mod tests {
    use super::{is_gguf, parent};

    #[test]
    fn only_gguf_files_count_as_models() {
        assert!(is_gguf("/models/Qwen3.5-2B-Q6_K.gguf"));
        assert!(is_gguf("D:/模型/Qwen3.5-2B-Q6_K.GGUF"));
        assert!(!is_gguf("/models/Qwen3.5-2B-Q6_K.safetensors"));
        assert!(!is_gguf("/models/gguf"));
    }

    #[test]
    fn the_dialog_starts_where_the_current_file_is() {
        let dir = std::env::temp_dir();
        let file = dir.join("qingjian-no-such-model.gguf");
        assert_eq!(parent(&file.to_string_lossy()), Some(dir.as_path()));
        assert_eq!(parent(""), None);
        assert_eq!(parent("/no/such/dir/model.gguf"), None);
    }
}
