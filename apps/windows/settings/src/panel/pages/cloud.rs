//! 「云服务」页：本地整句模型开关（`[model]`）、`[predict]` 各项与「测试连接」（后台线程跑）。

use qingjian_predict::{ConnectionTest, PredictConfig};
use std::path::PathBuf;
use windows_reactor::{
    Button, CancellationToken, ChildrenControl, ContentControl, NumberBox, Orientation,
    PasswordBox, StackPanel, TextBlock, TextBox, ToggleSwitch, View, ViewContext,
};

use crate::panel::cloud_status::CloudStatus;
use crate::panel::controls::{field, labeled, note, page};
use crate::panel::{Message, Settings};

/// 后台跑一次连通性测试，轮询到有结果或被取消。
pub(crate) fn run_test(
    config: &PredictConfig,
    usage_path: PathBuf,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let test = ConnectionTest::start_with_usage(config, Some(usage_path))
        .map_err(|error| error.to_string())?;
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

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let p = &settings.config.predict;
    let status = match &settings.cloud_status {
        CloudStatus::Idle => String::new(),
        CloudStatus::Testing => "测试中…".to_owned(),
        CloudStatus::Ok(message) => message.clone(),
        CloudStatus::Failed(message) => format!("失败：{message}"),
    };
    let rows = [
        field(
            "本地整句模型",
            "随包的小模型在本机给整句候选重新排序，全程离线；停键后几十毫秒生效。关掉只用词库统计。",
            ToggleSwitch::new()
                .is_on(settings.config.model.enabled)
                .on_toggled(context.callback(Message::LocalModel)),
        ),
        field(
            "启用云联想",
            "开启后可在输入拼音时按 Ctrl+Alt+J 获取 AI 候选；也可打开下面的自动联想。当前输入会发送给填写的服务。",
            ToggleSwitch::new()
                .is_on(p.enabled)
                .on_toggled(context.callback(Message::CloudEnabled)),
        ),
        field(
            "停顿后自动联想",
            "关闭时只按快捷键调用，日常输入和选词使用本地候选。快捷键可在「快捷键」页修改。",
            ToggleSwitch::new()
                .is_on(p.automatic)
                .on_toggled(context.callback(Message::CloudAutomatic)),
        ),
        field(
            "自动联想等待（毫秒）",
            "持续停键达到这个时间才发送；继续输入会重新计时。默认 800 毫秒。",
            NumberBox::new()
                .minimum(100.0)
                .maximum(10000.0)
                .value(p.debounce_ms as f64)
                .on_value_changed(context.callback(Message::CloudDebounce)),
        ),
        field(
            "最短请求间隔（毫秒）",
            "自动与快捷键请求共用间隔，默认 3000 毫秒。等待期间只保留最新输入；选词上屏后取消待发请求。缓存命中直接复用。",
            NumberBox::new()
                .minimum(1000.0)
                .maximum(60000.0)
                .value(p.min_interval_ms as f64)
                .on_value_changed(context.callback(Message::CloudMinInterval)),
        ),
        field(
            "AI 优先候选数",
            "AI 返回后按推荐顺序排在最前面，同文的本地候选只显示一次。移动高亮或翻页后保持当前顺序。0 = 只要小字云续写；续写也关闭时不调用。",
            NumberBox::new()
                .minimum(0.0)
                .maximum(9.0)
                .value(p.slots as f64)
                .on_value_changed(context.callback(Message::CloudSlots)),
        ),
        field(
            "小字云续写",
            "在拼音右侧用小字显示续写，按 Tab 采用。关闭后只请求 AI 候选词，省去续写提示和生成用量；停顿联想与 Ctrl+Alt+J 仍可使用。",
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
