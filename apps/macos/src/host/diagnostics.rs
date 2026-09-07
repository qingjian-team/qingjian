//! 诊断与日志：「关于」页的诊断信息（密钥抹掉）、输入日志开关与清空、打开外部程序。

use super::*;

impl Host {
    /// 给作者排查问题用的环境摘要：版本、系统、加载的数据、配置原文（密钥抹掉）、日志目录。
    pub(super) fn diagnostics(&self) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        let _ = writeln!(out, "青简 {} ({})", self.version, self.build);
        let os = NSProcessInfo::processInfo().operatingSystemVersionString();
        let _ = writeln!(out, "macOS {os} · {}", std::env::consts::ARCH);
        let _ = writeln!(out, "主词库：{} 条", self.engine.dictionary().len());
        for info in &self.dictionary_list {
            let state = if info.broken {
                "坏文件"
            } else if info.enabled {
                "启用"
            } else {
                "关闭"
            };
            let kind = if info.builtin {
                "随包词库"
            } else {
                "导入词库"
            };
            let _ = writeln!(out, "{kind}：{} · {} 条 · {state}", info.name, info.entries);
        }
        let languages: Vec<&str> = self.languages.iter().map(|l| l.code()).collect();
        let _ = writeln!(out, "释义表：{}", languages.join(" "));
        let _ = writeln!(
            out,
            "输入日志：{}",
            match Self::input_log_path() {
                Some(path) if self.input_log_enabled == Some(true) =>
                    format!("开，{} 行", InputLog::line_count(&path)),
                _ => "关".to_owned(),
            }
        );
        let usage = self.engine.usage_summary();
        let _ = writeln!(
            out,
            "输入统计：累计 {} 字 · {} 天",
            usage.total.hanzi, usage.days
        );
        let vocabulary = self.engine.vocabulary_summary();
        let _ = writeln!(
            out,
            "词汇（{}）：见过 {} · 看熟 {} · 上屏过 {}",
            self.engine.learning_language().code(),
            vocabulary.seen,
            vocabulary.familiar,
            vocabulary.committed
        );
        let _ = writeln!(
            out,
            "云联想：{}",
            if self.engine.prediction_enabled() {
                "开"
            } else {
                "关"
            }
        );
        if let Some(error) = self.settings.error() {
            let _ = writeln!(out, "配置错误：{error}");
        }
        if let Some(dir) = logging::log_dir() {
            let _ = writeln!(out, "日志目录：{}", dir.display());
        }
        if let Some(path) = self.settings.path() {
            let _ = writeln!(out, "配置文件：{}", path.display());
            if let Ok(text) = std::fs::read_to_string(path) {
                out.push_str("--- config.toml ---\n");
                out.push_str(&redact_secrets(&text));
            }
        }
        out
    }

    /// 输入日志文件的路径（数据目录下 `input-log.jsonl`）。
    pub fn input_log_path() -> Option<PathBuf> {
        paths::user_data_dir().map(|dir| dir.join("input-log.jsonl"))
    }

    /// 按开关接上 / 拆掉输入日志。
    pub(super) fn open_input_log(&mut self, enabled: bool) {
        match Self::input_log_path().filter(|_| enabled) {
            Some(path) => {
                tracing::info!(path = %path.display(), "输入日志开着");
                self.engine.set_input_logger(Box::new(InputLog::open(path)));
            }
            None => self.engine.set_input_logger(Box::new(NoInputLogger)),
        }
    }

    pub(super) fn clear_input_log(&mut self) {
        let Some(path) = Self::input_log_path() else {
            return;
        };
        self.engine.set_input_logger(Box::new(NoInputLogger));
        match InputLog::clear(&path) {
            Ok(()) => {
                tracing::info!(path = %path.display(), "输入日志已清空");
                self.preferences.set_status("输入日志已清空");
            }
            Err(error) => {
                tracing::warn!(%error, "输入日志清空失败");
                self.preferences.set_status("输入日志清空失败，见日志");
            }
        }
        let enabled = self.input_log_enabled.unwrap_or(false);
        self.open_input_log(enabled);
    }
}

/// 配置原文里的密钥行抹掉值（`api_key = "…"`；`api_key_env` 是变量名，不抹）。
pub(super) fn redact_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("api_key") && !trimmed.starts_with("api_key_env") {
                "api_key = \"<已隐去>\"".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 写进系统剪贴板。
pub(super) fn copy_to_pasteboard(text: &str) {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
}

pub(super) fn open_with_system(args: &[&str]) {
    if let Err(error) = std::process::Command::new("open").args(args).spawn() {
        tracing::warn!(%error, ?args, "open 失败");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn redaction_hides_key_values_only() {
        let text =
            "[predict]\napi_key = \"sk-secret\"\napi_key_env = \"QINGJIAN_API_KEY\"\nmodel = \"x\"";
        let redacted = super::redact_secrets(text);
        assert!(!redacted.contains("sk-secret"));
        assert!(redacted.contains("api_key_env = \"QINGJIAN_API_KEY\""));
        assert!(redacted.contains("model = \"x\""));
    }
}
