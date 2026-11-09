//! 云服务页的后台活：测试连接在自己的线程里等结果，到了叫醒窗口；还有把出错原因说成人话。

use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_predict::{
    API_USAGE_FILE, ConnectionReport, ConnectionTest, PredictConfig, PredictError,
};

use crate::i18n::{tr, tr_args};
use crate::support::Feature;

type Outcome = Result<ConnectionReport, PredictError>;

#[derive(Default)]
pub struct CloudTest {
    /// 测试线程放结果的地方。
    result: Arc<Mutex<Option<Outcome>>>,

    running: bool,
}

impl CloudTest {
    pub fn running(&self) -> bool {
        self.running
    }

    /// 开始测试；当场就失败（没有密钥）时返回那句话。
    pub fn start(&mut self, config: &PredictConfig, waker: Option<Waker>) -> Result<(), String> {
        // 测试连接也是一次真请求，记进统计页的云服务用量
        let usage = crate::system::paths::user_dir()
            .filter(|_| Feature::ApiUsage.here())
            .map(|dir| dir.join(API_USAGE_FILE));
        let test =
            ConnectionTest::start_with_usage(config, usage).map_err(|error| describe(&error))?;
        self.running = true;
        let result = Arc::clone(&self.result);
        std::thread::spawn(move || {
            let outcome = test.wait();
            if let Ok(mut slot) = result.lock() {
                *slot = Some(outcome);
            }
            if let Some(waker) = waker {
                waker.wake();
            }
        });
        Ok(())
    }

    /// 结果到了就取走，说成一句话。
    pub fn finish(&mut self) -> Option<String> {
        let outcome = self.result.lock().ok()?.take()?;
        self.running = false;
        Some(match outcome {
            Ok(report) => tr_args(
                "cloud_test.ok",
                &[
                    ("model", &report.model),
                    ("ms", &report.elapsed.as_millis()),
                ],
            ),
            Err(error) => tr_args("cloud_test.failed", &[("error", &describe(&error))]),
        })
    }
}

fn describe(error: &PredictError) -> String {
    match error {
        PredictError::MissingApiKey(_) => tr("cloud_test.no_key").to_owned(),
        PredictError::Timeout(ms) => tr_args("cloud_test.timeout", &[("ms", ms)]),
        PredictError::EmptyReply => tr("cloud_test.empty").to_owned(),
        PredictError::BudgetExhausted => tr("cloud_test.budget").to_owned(),
        PredictError::Encode(error) => tr_args("cloud_test.encode", &[("error", error)]),
        PredictError::Runtime(error) => tr_args("cloud_test.runtime", &[("error", error)]),
        PredictError::Api(error) => tr_args("cloud_test.api", &[("error", error)]),
    }
}
