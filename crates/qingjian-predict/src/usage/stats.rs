//! 本机累计 API 请求与 token 数；接口缺少用量的请求单独计数。

use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use qingjian_core::storage::write_atomic_str;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::usage::tokens::UsageTokens;

/// 从首次启用统计起累计的本机 API 用量。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiUsageStats {
    /// 实际尝试发送的请求；本地缓存与防抖取消不计入。
    pub requests: u64,

    /// 收到接口 JSON 响应的请求，包含已消耗 token 的空正文响应。
    pub responses: u64,

    /// 网络、HTTP 或超时失败。
    pub failures: u64,

    /// 已结束但未取得 usage 的请求；用量保持未知。
    pub unreported_usage: u64,

    pub input_tokens: u64,

    pub output_tokens: u64,

    pub total_tokens: u64,

    /// 已包含在 input_tokens 中。
    pub cached_input_tokens: u64,

    /// 首次统计请求的 Unix 秒。
    pub since_unix_secs: u64,
}

impl ApiUsageStats {
    /// 文件尚不存在时返回零；读失败或内容损坏时保留错误。
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    /// 请求已发起但尚未记录结果（包括进程中断时未能取得的结果）。
    pub fn unresolved_requests(&self) -> u64 {
        self.requests
            .saturating_sub(self.responses.saturating_add(self.failures))
    }

    pub(crate) fn record_request(path: &Path) {
        Self::record(path, |stats| {
            stats.requests = stats.requests.saturating_add(1);
            if stats.since_unix_secs == 0 {
                stats.since_unix_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
        });
    }

    pub(crate) fn record_response(path: &Path, response: Option<&Value>) {
        Self::record(path, |stats| {
            if response.is_some() {
                stats.responses = stats.responses.saturating_add(1);
            } else {
                stats.failures = stats.failures.saturating_add(1);
            }
            match response.and_then(UsageTokens::from_response) {
                Some(tokens) => {
                    stats.input_tokens = stats.input_tokens.saturating_add(tokens.input);
                    stats.output_tokens = stats.output_tokens.saturating_add(tokens.output);
                    stats.total_tokens = stats.total_tokens.saturating_add(tokens.total);
                    stats.cached_input_tokens = stats
                        .cached_input_tokens
                        .saturating_add(tokens.cached_input);
                }
                None => stats.unreported_usage = stats.unreported_usage.saturating_add(1),
            }
        });
    }

    fn record(path: &Path, change: impl FnOnce(&mut Self)) {
        if let Err(error) = Self::update(path, change) {
            tracing::warn!(%error, "API 用量统计保存失败");
        }
    }

    fn update(path: &Path, change: impl FnOnce(&mut Self)) -> io::Result<()> {
        if let Some(parent) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        // 联想线程、设置里的测试连接与配置热加载后的旧线程可能同时写。
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))?;
        lock.lock()?;
        let mut stats = Self::load(path)?;
        change(&mut stats);
        let text = serde_json::to_string_pretty(&stats).map_err(io::Error::other)?;
        write_atomic_str(path, &text)
    }
}
