//! 云服务页的后台活：测试连接在自己的线程里等结果，到了叫醒窗口；还有把出错原因说成人话。

use std::sync::{Arc, Mutex};

use bamboo_slip::Waker;
use qingjian_predict::{
    API_USAGE_FILE, ConnectionReport, ConnectionTest, LocalServerError, PredictConfig, PredictError,
};

use crate::i18n::{tr, tr_args};
use crate::support::Feature;

type Outcome = Result<ConnectionReport, PredictError>;

#[derive(Default)]
pub struct CloudTest {
    /// 测试线程放结果的地方。
    result: Arc<Mutex<Option<Outcome>>>,

    running: bool,

    /// 测的是本机模型：超时的原因与接口不同（多半是只用 CPU，头一次请求慢），说法跟着换。
    local: bool,
}

impl CloudTest {
    pub fn running(&self) -> bool {
        self.running
    }

    /// 开始测试；当场就失败（没有密钥）时返回那句话。用本机模型时测试线程先等它加载好（最多 20 秒），
    /// 输入法已经起好同一个模型就直接用它，没起就由这里临时起一个、测完停掉。
    pub fn start(&mut self, config: &PredictConfig, waker: Option<Waker>) -> Result<(), String> {
        // 测试连接也是一次真请求，记进统计页的云服务用量
        let usage = crate::system::paths::user_dirs()
            .filter(|_| Feature::ApiUsage.here())
            .map(|dirs| dirs.records().join(API_USAGE_FILE));
        self.local = config.uses_local_model();
        let test = ConnectionTest::start_with_usage(config, usage)
            .map_err(|error| describe(&error, self.local))?;
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
            Err(error) => tr_args(
                "cloud_test.failed",
                &[("error", &describe(&error, self.local))],
            ),
        })
    }
}

/// 出错原因说成人话；`local` 是测的本机模型。
fn describe(error: &PredictError, local: bool) -> String {
    match error {
        PredictError::MissingApiKey(_) => tr("cloud_test.no_key").to_owned(),
        PredictError::Timeout(ms) if local => tr_args("cloud_test.local_timeout", &[("ms", ms)]),
        PredictError::Timeout(ms) => tr_args("cloud_test.timeout", &[("ms", ms)]),
        PredictError::EmptyReply => tr("cloud_test.empty").to_owned(),
        PredictError::BudgetExhausted => tr("cloud_test.budget").to_owned(),
        PredictError::Encode(error) => tr_args("cloud_test.encode", &[("error", error)]),
        PredictError::Runtime(error) => tr_args("cloud_test.runtime", &[("error", error)]),
        PredictError::Api(error) => tr_args("cloud_test.api", &[("error", error)]),
        PredictError::LocalServer(error) => describe_local(error),
    }
}

fn describe_local(error: &LocalServerError) -> String {
    match error {
        LocalServerError::ServerNotFound => tr("cloud_test.local_not_found").to_owned(),
        LocalServerError::ModelNotFound(path) => {
            tr_args("cloud_test.local_no_model", &[("path", path)])
        }
        LocalServerError::PortTaken(port) => {
            tr_args("cloud_test.local_port_taken", &[("port", port)])
        }
        LocalServerError::Spawn(error) => tr_args("cloud_test.local_spawn", &[("error", error)]),
        LocalServerError::Exited(error) => tr_args("cloud_test.local_exited", &[("error", error)]),
        LocalServerError::Loading => tr("cloud_test.local_loading").to_owned(),
    }
}
