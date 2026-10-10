use std::sync::LazyLock;
use std::time::Duration;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, CreateChatCompletionRequestArgs,
    CreateChatCompletionResponse, FinishReason, ReasoningEffort, ResponseFormat,
};
use qingjian_core::PredictionRequest;
use reqwest::header::{HeaderMap, HeaderValue};

use crate::config::PredictConfig;
use crate::error::PredictError;
use crate::prompt::{self, Reply};

/// 联想回复的 token 上限：几条短句足够，防止模型长篇大论。
const MAX_TOKENS: u32 = 200;

/// 采样温度：联想要稳，不要花。
const TEMPERATURE: f32 = 0.3;

/// OpenCode Zen / Go 自 2026-09-06 起要求每个请求带这个头，值是同一会话内稳定的 ID，
/// 他们靠它把同一会话路由到同一上游复用 prompt 缓存，缺了直接 400。
const OPENCODE_SESSION_HEADER: &str = "x-opencode-session";

/// 本进程的会话 ID：输入法没有「会话」概念，一个进程算一个，进程内所有客户端共用。
static SESSION_ID: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().to_string());

/// OpenAI 兼容聊天接口的封装：一个请求进、若干联想条目出。
pub struct ChatClient {
    /// 底层客户端。
    client: Client<OpenAIConfig>,

    /// 模型名。
    model: String,

    /// 超时。
    timeout: Duration,

    /// 推理强度；`None` 表示不发这个参数。
    reasoning_effort: Option<ReasoningEffort>,

    /// 接口关思考用的是哪种参数（按接口地址定）。
    thinking_switch: ThinkingSwitch,
}

impl ChatClient {
    pub fn new(config: &PredictConfig, api_key: String) -> Self {
        let openai = OpenAIConfig::new()
            .with_api_base(config.base_url.trim_end_matches('/'))
            .with_api_key(api_key);
        Self {
            client: Client::with_config(openai).with_http_client(http_client(&config.base_url)),
            model: config.model.clone(),
            timeout: Duration::from_millis(config.timeout_ms),
            reasoning_effort: parse_reasoning_effort(&config.reasoning_effort),
            thinking_switch: ThinkingSwitch::for_url(&config.base_url),
        }
    }

    pub async fn complete(&self, request: &PredictionRequest) -> Result<Reply, PredictError> {
        let user = prompt::user_prompt(request);
        tracing::debug!(sequence = request.sequence, %user, "联想请求");
        let content = self
            .chat(prompt::system_prompt(request), &user, MAX_TOKENS)
            .await?;
        Ok(prompt::parse_reply(&content, request))
    }

    /// 一问一答：系统提示 + 用户消息，要 JSON 对象，返回正文。联想与释义兜底共用。
    pub async fn chat(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<String, PredictError> {
        let mut body = self.request_body(system, user, max_tokens)?;
        let raw = match self.send(&body).await {
            Ok(raw) => raw,
            // 「始终思考」的模型（如 GLM-5 系列）会拒绝关思考参数；去掉它按模型默认重试一次
            Err(PredictError::Api(error)) if rejects_thinking_control(&error) => {
                tracing::info!(model = %self.model, error = %error, "模型拒绝关思考，去掉该参数重试");
                strip_thinking_params(&mut body);
                self.send(&body).await?
            }
            Err(error) => return Err(error),
        };
        self.parse_content(raw)
    }

    /// 组请求体：类型化参数先构建成 JSON，再修温度精度、按接口改写关思考参数。
    fn request_body(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<serde_json::Value, PredictError> {
        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(system).into(),
            ChatCompletionRequestUserMessage::from(user).into(),
        ];
        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(&self.model)
            .messages(messages)
            .max_tokens(max_tokens)
            .temperature(TEMPERATURE)
            .response_format(ResponseFormat::JsonObject);
        if let Some(effort) = self.reasoning_effort.clone() {
            args.reasoning_effort(effort);
        }
        let mut body = serde_json::to_value(args.build()?)?;
        quantize_temperature(&mut body);
        if matches!(self.reasoning_effort, Some(ReasoningEffort::None)) {
            self.thinking_switch.disable(&mut body);
        }
        Ok(body)
    }

    /// 发一次请求，返回原始 JSON。
    async fn send(&self, body: &serde_json::Value) -> Result<serde_json::Value, PredictError> {
        tokio::time::timeout(self.timeout, self.client.chat().create_byot(body.clone()))
            .await
            .map_err(|_| PredictError::Timeout(self.timeout.as_millis() as u64))?
            .map_err(PredictError::from)
    }

    /// 从原始响应里取正文；空正文时把原始响应记进日志，方便排查思考占满预算、模型名不对这类问题。
    fn parse_content(&self, raw: serde_json::Value) -> Result<String, PredictError> {
        let response: CreateChatCompletionResponse = serde_json::from_value(raw.clone())?;
        let cut_off = response
            .choices
            .iter()
            .any(|choice| choice.finish_reason == Some(FinishReason::Length));
        let content = response
            .choices
            .into_iter()
            .inspect(
                |choice| tracing::debug!(finish_reason = ?choice.finish_reason, "联想回复结束原因"),
            )
            .find_map(|choice| choice.message.content.filter(|c| !c.trim().is_empty()))
            .ok_or_else(|| {
                // 正文为空时原因五花八门（思考占满额度、模型名不对、接口字段不标准），留下原始响应才查得了
                tracing::warn!(model = %self.model, response = %truncated(&raw), "接口回复里没有正文");
                if cut_off {
                    PredictError::BudgetExhausted
                } else {
                    PredictError::EmptyReply
                }
            })?;
        tracing::debug!(%content, "模型回复");
        Ok(content)
    }
}

/// 日志里的原始响应最多留这么多字符。
const LOGGED_RESPONSE_CHARS: usize = 2000;

fn truncated(response: &serde_json::Value) -> String {
    let text = response.to_string();
    match text.char_indices().nth(LOGGED_RESPONSE_CHARS) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

/// 各家关思考的参数不一样，严格的接口遇到不认识的参数会报 400，所以按接口地址只发对的那个。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThinkingSwitch {
    /// OpenAI 一系：`reasoning_effort: "none"`，请求里已经带了。
    ReasoningEffort,

    /// 智谱（`bigmodel.cn` / `z.ai`）：`thinking: {"type": "disabled"}`，不认 `reasoning_effort`。
    ThinkingType,
}

impl ThinkingSwitch {
    fn for_url(base_url: &str) -> Self {
        let host = host_of(base_url).unwrap_or_default();
        let zhipu = ["bigmodel.cn", "z.ai"]
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
        if zhipu {
            Self::ThinkingType
        } else {
            Self::ReasoningEffort
        }
    }

    /// 把请求体改成这家接口关思考的写法。
    fn disable(self, body: &mut serde_json::Value) {
        let (Self::ThinkingType, Some(fields)) = (self, body.as_object_mut()) else {
            return;
        };
        fields.remove("reasoning_effort");
        fields.insert(
            "thinking".to_owned(),
            serde_json::json!({ "type": "disabled" }),
        );
    }
}

fn host_of(base_url: &str) -> Option<String> {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
}

/// 请求体里的温度修成两位小数。f32 一进 `serde_json::Value` 就按 f64 存，会带上精度尾巴
/// （0.3 → 0.300000011920929），智谱这类限制两位小数的接口直接 400（code 1210）。
/// 各家接口都不拒绝合法值的两位小数表示，统一修无副作用。
fn quantize_temperature(body: &mut serde_json::Value) {
    let Some(fields) = body.as_object_mut() else {
        return;
    };
    let Some(t) = fields.get("temperature").and_then(|v| v.as_f64()) else {
        return;
    };
    let rounded = (t * 100.0).round() / 100.0;
    fields.insert(
        "temperature".to_owned(),
        serde_json::Number::from_f64(rounded).into(),
    );
}

/// 去掉请求里所有关思考参数，交给模型默认行为。
fn strip_thinking_params(body: &mut serde_json::Value) {
    if let Some(fields) = body.as_object_mut() {
        fields.remove("reasoning_effort");
        fields.remove("thinking");
    }
}

/// 报错是不是「模型始终思考，不支持关闭」类：去掉关思考参数重试或许能通。
/// 智谱的报错是中文（「该模型始终思考，不支持关闭思考」），z.ai 国际站可能给英文；温度之类的 1210 不能误伤。
fn rejects_thinking_control(error: &async_openai::error::OpenAIError) -> bool {
    let message = error.to_string();
    message.contains("思考") || message.to_lowercase().contains("always think")
}

#[cfg(test)]
fn thinking_error(message: &str) -> async_openai::error::OpenAIError {
    async_openai::error::OpenAIError::ApiError(async_openai::error::ApiErrorResponse {
        status_code: reqwest::StatusCode::BAD_REQUEST,
        api_error: async_openai::error::ApiError {
            message: message.to_owned(),
            r#type: None,
            param: None,
            code: None,
            misalignment: None,
        },
    })
}

/// 按接口地址决定 HTTP 客户端：OpenCode 带上它要求的会话头，其他服务用默认客户端。
/// 头装不上（理论上不会）就退回默认客户端，请求照发，让服务端的报错说明问题。
fn http_client(base_url: &str) -> reqwest::Client {
    if !is_opencode(base_url) {
        return reqwest::Client::new();
    }
    let Ok(value) = HeaderValue::from_str(&SESSION_ID) else {
        return reqwest::Client::new();
    };
    let mut headers = HeaderMap::new();
    headers.insert(OPENCODE_SESSION_HEADER, value);
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_default()
}

/// 接口地址是否指向 OpenCode（`opencode.ai` 及其子域）。
fn is_opencode(base_url: &str) -> bool {
    host_of(base_url).is_some_and(|host| host == "opencode.ai" || host.ends_with(".opencode.ai"))
}

/// 配置里的推理强度字符串转成接口枚举；留空不发，认不得的值当留空并记一条警告。
fn parse_reasoning_effort(value: &str) -> Option<ReasoningEffort> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "none" => Some(ReasoningEffort::None),
        "minimal" => Some(ReasoningEffort::Minimal),
        "low" => Some(ReasoningEffort::Low),
        "medium" => Some(ReasoningEffort::Medium),
        "high" => Some(ReasoningEffort::High),
        "xhigh" => Some(ReasoningEffort::Xhigh),
        other => {
            tracing::warn!(value = other, "reasoning_effort 不认识，不发这个参数");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_effort_parses_known_values_and_ignores_the_rest() {
        assert!(matches!(
            parse_reasoning_effort("none"),
            Some(ReasoningEffort::None)
        ));
        assert!(matches!(
            parse_reasoning_effort(" High "),
            Some(ReasoningEffort::High)
        ));
        assert!(parse_reasoning_effort("").is_none());
        assert!(parse_reasoning_effort("maximum").is_none());
    }

    /// 回归 #287 Bug 1：温度上到线上不能带 f32→f64 的精度尾巴。
    #[test]
    fn temperature_is_sent_with_at_most_two_decimals() {
        let client = ChatClient::new(&PredictConfig::default(), "key".to_owned());
        let body = client.request_body("s", "u", 10).unwrap();
        assert_eq!(body["temperature"].as_f64(), Some(0.3), "值应等于 0.3");
        assert_eq!(
            body["temperature"].to_string(),
            "0.3",
            "序列化后不得出现 0.300000011920929 这类精度尾巴"
        );
    }

    /// 回归 #287 Bug 2：始终思考模型的报错能识别，重试时把两类关思考参数都摘掉。
    #[test]
    fn thinking_rejection_is_detected_and_params_stripped() {
        let error = thinking_error("该模型始终思考，不支持关闭思考；请使用 low、high 或 max。");
        assert!(rejects_thinking_control(&error));
        // 温度报错同样走 1210，不能误判成思考拒绝
        let temperature_error = thinking_error("temperature参数非法：限制小数点[2]位");
        assert!(!rejects_thinking_control(&temperature_error));
        assert!(!rejects_thinking_control(&temperature_error));

        let mut body = serde_json::json!({
            "model": "glm-5.3-flash",
            "reasoning_effort": "none",
            "thinking": { "type": "disabled" },
            "temperature": 0.3
        });
        strip_thinking_params(&mut body);
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("thinking").is_none());
        assert_eq!(body["temperature"], 0.3);
    }

    #[test]
    fn zhipu_hosts_disable_thinking_with_their_own_field() {
        let mut body = serde_json::json!({ "model": "glm", "reasoning_effort": "none" });
        ThinkingSwitch::for_url("https://open.bigmodel.cn/api/paas/v4").disable(&mut body);
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("reasoning_effort").is_none());
        assert_eq!(
            ThinkingSwitch::for_url("https://api.z.ai/api/paas/v4"),
            ThinkingSwitch::ThinkingType
        );

        let mut body = serde_json::json!({ "model": "x", "reasoning_effort": "none" });
        ThinkingSwitch::for_url("https://api.deepseek.com").disable(&mut body);
        assert_eq!(body["reasoning_effort"], "none");
        assert!(body.get("thinking").is_none());
        assert_eq!(
            ThinkingSwitch::for_url("https://example.com/bigmodel.cn"),
            ThinkingSwitch::ReasoningEffort
        );
    }

    #[test]
    fn opencode_is_recognized_by_host_only() {
        assert!(is_opencode("https://opencode.ai/zen/go/v1"));
        assert!(is_opencode("https://OpenCode.ai/zen/v1/"));
        assert!(is_opencode("https://api.opencode.ai/v1"));
        assert!(!is_opencode("https://api.deepseek.com"));
        assert!(!is_opencode("https://example.com/opencode.ai"));
        assert!(!is_opencode("not a url"));
    }

    #[test]
    fn session_id_is_stable_within_the_process() {
        assert_eq!(*SESSION_ID, *SESSION_ID);
        assert_eq!(SESSION_ID.len(), 36);
    }

    /// 真发一个请求到本地端口，看 OpenCode 客户端带了会话头、默认客户端没带。
    #[test]
    fn opencode_client_sends_the_session_header() {
        assert_eq!(
            header_seen_by_server(http_client("https://opencode.ai/zen/go/v1")),
            Some(SESSION_ID.clone())
        );
        assert_eq!(
            header_seen_by_server(http_client("https://api.deepseek.com")),
            None
        );
    }

    /// 起一个只答一次的 HTTP 服务，返回请求里 `x-opencode-session` 的值。
    fn header_seen_by_server(client: reqwest::Client) -> Option<String> {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = None;
            for line in BufReader::new(&stream).lines() {
                let line = line.unwrap();
                if line.is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case(OPENCODE_SESSION_HEADER)
                {
                    header = Some(value.trim().to_owned());
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .unwrap();
            header
        });
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { client.get(&url).send().await.unwrap() });
        server.join().unwrap()
    }
}
