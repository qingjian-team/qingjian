//! 一段识别的后台线程：连上、发配置、边收录音边发包、收结果交回去。

use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use qingjian_core::TranscriptEvent;
use serde_json::json;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::{HandshakeError, Message, WebSocket};

use super::VolcengineConfig;
use super::command::Command;
use super::protocol::{self, Incoming};
use super::reply::Reply;
use crate::VoiceError;
use crate::tls::{self, TlsStream};

/// 线程入口：出错时（不是被取消的）交回一句给用户看的话。
pub fn run(
    config: VolcengineConfig,
    sample_rate: u32,
    commands: Receiver<Command>,
    events: Sender<TranscriptEvent>,
    cancelled: Arc<AtomicBool>,
) {
    let mut stream = Stream {
        sample_rate,
        commands,
        events,
        cancelled,
        last_text: String::new(),
    };
    if let Err(error) = stream.run(&config) {
        if stream.cancelled.load(Ordering::Relaxed) {
            return;
        }
        tracing::warn!(%error, "语音识别出错");
        let _ = stream.events.send(TranscriptEvent::Error(error.describe()));
    }
}

struct Stream {
    sample_rate: u32,

    commands: Receiver<Command>,

    events: Sender<TranscriptEvent>,

    cancelled: Arc<AtomicBool>,

    /// 上一次交回去的中间结果，没变就不再交。
    last_text: String,
}

impl Stream {
    fn run(&mut self, config: &VolcengineConfig) -> Result<(), VoiceError> {
        let mut socket = connect(config)?;
        let request = request(config, self.sample_rate);
        socket.send(Message::binary(protocol::full_request(request.as_bytes())))?;
        // 之后读不到就是没有，回头看看有没有新录音
        socket
            .get_ref()
            .get_ref()
            .set_read_timeout(Some(POLL))
            .map_err(|error| VoiceError::Connect(error.to_string()))?;

        let packet = self.sample_rate as usize * PACKET_MS / 1000;
        let mut pending: Vec<i16> = Vec::new();
        let mut sequence = 2;
        let mut deadline: Option<Instant> = None;
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                let _ = socket.close(None);
                return Ok(());
            }
            if deadline.is_none() {
                let finished = loop {
                    match self.commands.try_recv() {
                        Ok(Command::Audio(samples)) => pending.extend_from_slice(&samples),
                        Ok(Command::Finish) => break true,
                        Err(TryRecvError::Empty) => break false,
                        // 识别方被扔掉了
                        Err(TryRecvError::Disconnected) => return Ok(()),
                    }
                };
                while pending.len() >= packet {
                    let chunk: Vec<i16> = pending.drain(..packet).collect();
                    socket.send(Message::binary(protocol::audio(
                        sequence,
                        &pcm(&chunk),
                        false,
                    )))?;
                    sequence += 1;
                }
                if finished {
                    socket.send(Message::binary(protocol::audio(
                        sequence,
                        &pcm(&pending),
                        true,
                    )))?;
                    pending.clear();
                    deadline = Some(Instant::now() + FINAL_TIMEOUT);
                }
            }
            match socket.read() {
                Ok(Message::Binary(frame)) => match protocol::parse(&frame)? {
                    Incoming::Response { last, payload } => {
                        let reply: Reply = serde_json::from_slice(&payload).unwrap_or_default();
                        if last {
                            let text = reply.text().to_owned();
                            let _ = self.events.send(TranscriptEvent::Final(text));
                            let _ = socket.close(None);
                            return Ok(());
                        }
                        if reply.text() != self.last_text {
                            reply.text().clone_into(&mut self.last_text);
                            let text = self.last_text.clone();
                            let _ = self.events.send(TranscriptEvent::Partial(text));
                        }
                    }
                    Incoming::Error { code, message } => {
                        return Err(VoiceError::Server { code, message });
                    }
                    Incoming::Other => {}
                },
                Ok(Message::Close(_)) => return Err(VoiceError::Closed),
                Ok(_) => {}
                Err(tungstenite::Error::Io(error))
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(error) => return Err(error.into()),
            }
            if deadline.is_some_and(|deadline| Instant::now() > deadline) {
                return Err(VoiceError::Timeout(FINAL_TIMEOUT.as_millis() as u64));
            }
        }
    }
}

/// TLS + WebSocket 握手，带上鉴权头。
fn connect(config: &VolcengineConfig) -> Result<WebSocket<TlsStream>, VoiceError> {
    let mut request = config.endpoint.as_str().into_client_request()?;
    let host = request.uri().host().unwrap_or_default().to_owned();
    let port = request.uri().port_u16().unwrap_or(443);
    let request_id = uuid::Uuid::new_v4().to_string();
    let headers = request.headers_mut();
    for (name, value) in [
        ("X-Api-Key", config.api_key.as_str()),
        ("X-Api-Resource-Id", config.resource_id.as_str()),
        ("X-Api-Request-Id", request_id.as_str()),
        ("X-Api-Connect-Id", request_id.as_str()),
        ("X-Api-Sequence", "-1"),
    ] {
        let value = HeaderValue::from_str(value)
            .map_err(|_| VoiceError::Connect(format!("{name} 里有不能放进请求头的字符")))?;
        headers.insert(name, value);
    }
    let stream = tls::connect(&host, port, CONNECT_TIMEOUT)?;
    match tungstenite::client(request, stream) {
        Ok((socket, response)) => {
            tracing::debug!(logid = logid(response.headers()), "语音识别连上了");
            Ok(socket)
        }
        Err(HandshakeError::Failure(tungstenite::Error::Http(response))) => {
            let status = response.status().as_u16();
            tracing::warn!(
                status,
                logid = logid(response.headers()),
                "语音识别握手被拒"
            );
            Err(VoiceError::Rejected(status))
        }
        Err(HandshakeError::Failure(error)) => Err(error.into()),
        Err(HandshakeError::Interrupted(_)) => Err(VoiceError::Connect("握手被打断".to_owned())),
    }
}

/// 服务端给的排错编号，报问题时用。
fn logid(headers: &tungstenite::http::HeaderMap) -> String {
    headers
        .get("X-Tt-Logid")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

/// 识别参数：PCM 单声道 16 位，带标点与数字规整，开二遍识别（说完后整段再精修一遍）。
fn request(config: &VolcengineConfig, sample_rate: u32) -> String {
    json!({
        "user": { "uid": "qingjian" },
        "audio": { "format": "pcm", "codec": "raw", "rate": sample_rate, "bits": 16, "channel": 1 },
        "request": {
            "model_name": "bigmodel",
            "enable_itn": true,
            "enable_punc": config.punctuation,
            "enable_nonstream": true,
            "result_type": "full",
        },
    })
    .to_string()
}

fn pcm(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

/// 一包音频多长（接口建议 100～200 ms）。
const PACKET_MS: usize = 200;

/// 读不到时多久回头看一眼新录音。
const POLL: Duration = Duration::from_millis(20);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// 说完后最终结果最多等多久。
const FINAL_TIMEOUT: Duration = Duration::from_secs(8);
