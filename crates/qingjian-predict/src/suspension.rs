//! 账户层面失败（余额不足、密钥无效）后的暂停：充值或换密钥之前每次请求都必然失败，
//! 照发只会刷日志、白跑网络，所以停一段时间再试；改配置时壳会重建 Worker，暂停随之清零。

use std::time::{Duration, Instant};

use crate::error::PredictError;

/// 暂停多久再试一次。
pub const ACCOUNT_PAUSE: Duration = Duration::from_secs(10 * 60);

/// 一个后台线程的暂停状态。
#[derive(Debug, Default)]
pub struct Suspension {
    /// 暂停到什么时候；`None` 表示没在暂停。
    until: Option<Instant>,
}

impl Suspension {
    /// 现在是否还在暂停中；暂停到期时记一条日志并恢复。
    pub fn active(&mut self, now: Instant) -> bool {
        match self.until {
            Some(until) if now < until => true,
            Some(_) => {
                self.until = None;
                tracing::info!("云端请求暂停到期，重新尝试");
                false
            }
            None => false,
        }
    }

    /// 按这次的错误决定要不要暂停；暂停了返回 `true`（调用方就不必再为这次失败记日志）。
    pub fn record(&mut self, error: &PredictError, now: Instant, what: &str) -> bool {
        if !error.is_account_failure() {
            return false;
        }
        self.until = Some(now + ACCOUNT_PAUSE);
        tracing::warn!(
            %error,
            pause_minutes = ACCOUNT_PAUSE.as_secs() / 60,
            "{what}失败：账户不可用（余额不足或密钥无效），暂停云端请求，改配置或到期后再试"
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use async_openai::error::{ApiError, ApiErrorResponse, OpenAIError};

    use super::*;

    fn api_error(status: u16) -> PredictError {
        PredictError::Api(OpenAIError::ApiError(ApiErrorResponse {
            status_code: reqwest::StatusCode::from_u16(status).unwrap(),
            api_error: ApiError {
                message: "Insufficient Balance".to_owned(),
                r#type: None,
                param: None,
                code: None,
                misalignment: None,
            },
        }))
    }

    #[test]
    fn account_failures_pause_until_the_deadline() {
        let now = Instant::now();
        let mut suspension = Suspension::default();
        assert!(!suspension.active(now));
        assert!(suspension.record(&api_error(402), now, "联想"));
        assert!(suspension.active(now + Duration::from_secs(60)));
        assert!(!suspension.active(now + ACCOUNT_PAUSE));
        assert!(!suspension.active(now + ACCOUNT_PAUSE));
    }

    #[test]
    fn transient_failures_do_not_pause() {
        let now = Instant::now();
        let mut suspension = Suspension::default();
        for status in [400, 429, 500, 503] {
            assert!(!suspension.record(&api_error(status), now, "联想"));
        }
        assert!(!suspension.record(&PredictError::Timeout(5000), now, "联想"));
        assert!(!suspension.active(now));
        for status in [401, 402, 403] {
            assert!(suspension.record(&api_error(status), now, "联想"));
        }
    }
}
