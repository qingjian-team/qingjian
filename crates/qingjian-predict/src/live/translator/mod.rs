//! 独立线程驱动本机 Ollama，请求更新立即取消旧任务，结果仅接受最新修订。

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::{Client, Url};
use serde_json::{Value, json};
use tokio::task::JoinHandle;

use super::{LiveTranslationConfig, Translation};

pub struct LiveTranslator {
    requests: Sender<Option<(u64, String)>>,

    responses: Receiver<Translation>,

    latest: u64,
}

impl LiveTranslator {
    pub fn new(config: LiveTranslationConfig) -> Result<Self, String> {
        let endpoint = local_endpoint(&config.base_url)?;
        if config.model.trim().is_empty() {
            return Err("请设置本机翻译模型名称".into());
        }
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(config.timeout_ms.clamp(100, 120000)))
            .build()
            .map_err(|_| "无法初始化本机翻译连接".to_owned())?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "无法初始化本机翻译任务".to_owned())?;
        let (requests, incoming) = mpsc::channel::<Option<(u64, String)>>();
        let (outgoing, responses) = mpsc::channel();
        thread::Builder::new()
            .name("qingjian-live-translation".into())
            .spawn(move || {
                let mut active: Option<JoinHandle<Translation>> = None;
                let mut pending = None;
                loop {
                    // 完全空闲时阻塞等待按键请求，不持续唤醒常驻输入法进程。
                    let request = if active.is_none() && pending.is_none() {
                        incoming
                            .recv()
                            .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
                    } else {
                        incoming.recv_timeout(Duration::from_millis(8))
                    };
                    match request {
                        Ok(request) => {
                            let mut request = request;
                            while let Ok(newer) = incoming.try_recv() {
                                request = newer;
                            }
                            if let Some(task) = active.take() {
                                task.abort();
                            }
                            pending = request.map(|request| (Instant::now(), request));
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    // 每次推进运行时让 HTTP 和任务取消得到处理，不阻塞接收更新。
                    runtime.block_on(async { tokio::time::sleep(Duration::from_millis(1)).await });
                    if active.as_ref().is_some_and(JoinHandle::is_finished)
                        && let Some(task) = active.take()
                        && let Ok(reply) = runtime.block_on(task)
                    {
                        let _ = outgoing.send(reply);
                    }
                    if pending.as_ref().is_some_and(|(since, _)| {
                        since.elapsed() >= Duration::from_millis(config.debounce_ms.min(5000))
                    }) && let Some((_, (id, source))) = pending.take()
                    {
                        let client = client.clone();
                        let endpoint = endpoint.clone();
                        let model = config.model.clone();
                        active = Some(runtime.spawn(async move {
                            let result = translate(&client, endpoint, &model, &source).await;
                            Translation { id, source, result }
                        }));
                    }
                }
                if let Some(task) = active {
                    task.abort();
                }
            })
            .map_err(|_| "无法启动本机翻译任务".to_owned())?;
        Ok(Self {
            requests,
            responses,
            latest: 0,
        })
    }

    pub fn submit(&mut self, source: impl Into<String>) -> u64 {
        self.latest = self.latest.wrapping_add(1);
        let source = source.into();
        if source.trim().is_empty() {
            let _ = self.requests.send(None);
        } else {
            let _ = self.requests.send(Some((self.latest, source)));
        }
        self.latest
    }

    pub fn cancel(&mut self) {
        self.latest = self.latest.wrapping_add(1);
        let _ = self.requests.send(None);
    }

    pub fn poll(&mut self) -> Option<Translation> {
        loop {
            match self.responses.try_recv() {
                Ok(reply) if reply.id == self.latest => return Some(reply),
                Ok(_) => continue,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            }
        }
    }
}

fn local_endpoint(base: &str) -> Result<Url, String> {
    let mut url = Url::parse(base).map_err(|_| "本机模型地址格式无效".to_owned())?;
    let loopback = match url.host_str() {
        Some("localhost") => {
            url.set_host(Some("127.0.0.1"))
                .map_err(|_| "本机模型地址无效".to_owned())?;
            true
        }
        Some(host) => host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()),
        None => false,
    };
    if !loopback
        || url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return Err("实时翻译仅支持本机 HTTP 地址，例如 http://localhost:11434".into());
    }
    url.set_path("/api/generate");
    Ok(url)
}

async fn translate(
    client: &Client,
    endpoint: Url,
    model: &str,
    source: &str,
) -> Result<String, String> {
    let target = target_language(source);
    let prompt = format!(
        "Translate the following segment into {target}, without additional explanation. Translate only the text provided. If it is incomplete, preserve its incompleteness; do not invent or complete missing content.\n\n{source}"
    );
    let response = client.post(endpoint)
        .header("Content-Type", "application/json")
        .body(json!({"model": model, "prompt": prompt, "stream": false, "keep_alive": "5m", "options": {"temperature": 0, "num_predict": 512}}).to_string())
        .send().await.map_err(|error| if error.is_timeout() { "本机翻译超时，请检查模型是否已加载".to_owned() } else { "无法连接本机 Ollama，请确认服务已启动".to_owned() })?;
    if !response.status().is_success() {
        return Err(format!(
            "本机翻译失败（HTTP {}），请检查模型名称与 Ollama 服务",
            response.status().as_u16()
        ));
    }
    let body = response
        .text()
        .await
        .map_err(|_| "无法读取本机翻译结果".to_owned())?;
    let value: Value =
        serde_json::from_str(&body).map_err(|_| "本机翻译返回了无效格式".to_owned())?;
    let text = value
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return Err("本机模型未返回译文".into());
    }
    if value.get("done_reason").and_then(Value::as_str) == Some("length") {
        return Err("句子过长，译文未完成，请分句输入".into());
    }
    Ok(text.to_owned())
}

fn target_language(source: &str) -> &'static str {
    let mut han = 0;
    let mut latin_words = 0;
    let mut in_latin_word = false;
    for ch in source.chars() {
        if ('\u{3400}'..='\u{9fff}').contains(&ch) {
            han += 1;
        }
        if ch.is_ascii_alphabetic() && !in_latin_word {
            latin_words += 1;
        }
        in_latin_word = ch.is_ascii_alphabetic();
    }
    // 英文按词而非字母计数，避免一个产品名压过整句中文。
    if han > 0 && han >= latin_words {
        "English"
    } else {
        "Chinese"
    }
}

#[cfg(test)]
mod tests;
