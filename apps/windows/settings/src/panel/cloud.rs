//! 「云服务」设置页：云联想（`[predict]`）开关、云端词位置、整句补全、接口 / 模型 / 密钥、推理强度，
//! 以及「测试连接」按钮（后台线程跑，不卡界面）。对齐 macOS「云服务」页。

use qingjian_predict::{ConnectionTest, PredictConfig};
use windows_reactor::*;

use super::{CloudStatus, Message, Settings, field, labeled, note, page};

/// 在后台线程里跑一次连通性测试：起测试、轮询到有结果或被取消。给 `spawn_background` 用。
pub(super) fn run_test(
    config: &PredictConfig,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let test = ConnectionTest::start(config).map_err(|error| error.to_string())?;
    loop {
        if cancel.is_cancelled() {
            return Err("已取消".to_owned());
        }
        if let Some(result) = test.poll() {
            return result
                .map(|report| {
                    format!(
                        "连接成功：模型 {}，耗时 {} ms",
                        report.model,
                        report.elapsed.as_millis()
                    )
                })
                .map_err(|error| error.to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub(super) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let p = &settings.config.predict;
    let status = match &settings.cloud_status {
        CloudStatus::Idle => String::new(),
        CloudStatus::Testing => "测试中…".to_owned(),
        CloudStatus::Ok(message) => message.clone(),
        CloudStatus::Failed(message) => format!("失败：{message}"),
    };
    let rows = [
        field(
            "启用云联想",
            "开启后组句时会把光标附近的几十个字发给下面的服务，让模型补全整句、联想下文；密钥框里的内容不发送。",
            ToggleSwitch::new()
                .is_on(p.enabled)
                .on_toggled(context.callback(Message::CloudEnabled)),
        ),
        field(
            "云端词格数",
            "云端词到了只补进第一页末尾这几格，前面的本地候选不动；没到就什么都不变。0 = 只要整句补全。",
            NumberBox::new()
                .minimum(0.0)
                .maximum(9.0)
                .value(p.slots as f64)
                .on_value_changed(context.callback(Message::CloudSlots)),
        ),
        field(
            "整句补全",
            "preedit 右侧给出整句补全，按 Tab 采用。",
            ToggleSwitch::new()
                .is_on(p.sentence)
                .on_toggled(context.callback(Message::CloudSentence)),
        ),
        field(
            "接口地址",
            "",
            TextBox::new()
                .text(p.base_url.clone())
                .on_text_changed(context.callback(Message::CloudBaseUrl)),
        ),
        field(
            "模型",
            "",
            TextBox::new()
                .text(p.model.clone())
                .on_text_changed(context.callback(Message::CloudModel)),
        ),
        field(
            "API 密钥",
            "只保存在这台电脑上，不会随配置文件导出，也不显示已填的值。留空则读环境变量 QINGJIAN_API_KEY。",
            PasswordBox::new()
                .password(p.api_key.clone().unwrap_or_default())
                .placeholder_text("留空则读环境变量 QINGJIAN_API_KEY")
                .on_password_changed(context.callback(Message::CloudApiKey)),
        ),
        labeled(
            "",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .children((
                    Button::new()
                        .on_click(context.message(Message::TestConnection))
                        .content("测试连接"),
                    TextBlock::new().text(status),
                )),
        ),
        note(
            "用上面填的地址、模型、密钥发一条最小请求。走不通时先查这里；Server 进程看不到终端里的代理变量。",
        ),
    ];
    page("云服务", StackPanel::new().spacing(16.0).children(rows))
}
