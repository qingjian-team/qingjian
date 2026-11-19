//! 本机服务的测试：启动参数、找程序、端口探测、退出原因、状态通知与租约共用；端口上用假服务答话，不起真的 llama-server。

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::error::LocalServerError;
use super::locate::{locate, search};
use super::probe::{Probe, parse_response, probe, serving};
use super::server::LocalServer;
use super::spec::{ALIAS, LaunchSpec};
use super::status::{State, Status};
use crate::config::PredictConfig;

/// 等服务就绪或报错的上限：Windows 上探一个空端口要两秒左右。
const WAIT: Duration = Duration::from_secs(15);

fn spec(model: &str, port: u16) -> LaunchSpec {
    LaunchSpec {
        model: model.to_owned(),
        server: None,
        port,
        extra: Vec::new(),
    }
}

/// 一个此刻没人用的端口。
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// 每个测试自己的临时目录。
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-local-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 起一个对什么请求都回同一句的 HTTP 服务，返回端口；放下返回的标志就停。
struct FakeServer {
    port: u16,
    stop: Arc<AtomicBool>,
}

impl FakeServer {
    fn start(status: u16, body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
                    if line == "\r\n" {
                        break;
                    }
                    line.clear();
                }
                let _ = write!(
                    reader.get_mut(),
                    "HTTP/1.1 {status} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Self { port, stop }
    }

    fn props(model: &str) -> Self {
        let body = serde_json::json!({
            "model_alias": ALIAS,
            "model_path": model,
            "total_slots": 2,
        });
        Self::start(200, body.to_string())
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[test]
fn spec_comes_from_the_local_keys_and_user_arguments_go_last() {
    assert!(LaunchSpec::from_config(&PredictConfig::default()).is_none());
    let config = PredictConfig {
        local_model: " /models/Qwen3.5-2B-Q6_K.gguf ".to_owned(),
        local_server: " ".to_owned(),
        local_args: vec!["-ngl".to_owned(), "0".to_owned()],
        ..PredictConfig::default()
    };
    let spec = LaunchSpec::from_config(&config).unwrap();
    assert_eq!(spec.model, "/models/Qwen3.5-2B-Q6_K.gguf");
    assert_eq!(spec.server, None);
    assert_eq!(spec.port, 18686);
    let arguments: Vec<String> = spec
        .arguments()
        .into_iter()
        .map(|argument| argument.into_string().unwrap())
        .collect();
    let joined = arguments.join(" ");
    assert!(joined.starts_with("-m /models/Qwen3.5-2B-Q6_K.gguf --host 127.0.0.1 --port 18686"));
    for expected in [
        "--alias qingjian-local",
        "--reasoning off",
        "--cors-origins localhost",
        "--no-slots",
        "-c 8192",
    ] {
        assert!(joined.contains(expected), "{expected}");
    }
    assert!(joined.ends_with("-ngl 0"));
}

#[test]
fn program_is_found_in_the_first_directory_that_has_it() {
    let dir = temp_dir("locate");
    let (empty, full) = (dir.join("empty"), dir.join("full"));
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(&full).unwrap();
    let name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let program = full.join(name);
    std::fs::write(&program, b"").unwrap();
    assert_eq!(
        search(&[empty.clone(), full.clone()]),
        Some(program.clone())
    );
    assert_eq!(search(&[empty]), None);
    // 指定了路径就只认它
    assert_eq!(locate(Some(&program)), Some(program.clone()));
    assert_eq!(locate(Some(&full.join("missing"))), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn responses_are_split_by_length_chunks_or_closing() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nokEXTRA";
    assert_eq!(parse_response(raw, false), Some((200, b"ok".to_vec())));
    let partial = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nok";
    assert_eq!(parse_response(partial, false), None);
    let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n3;x=1\r\n!!!\r\n0\r\n\r\n";
    assert_eq!(
        parse_response(chunked, false),
        Some((200, b"ok!!!".to_vec()))
    );
    let unfinished = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n";
    assert_eq!(parse_response(unfinished, true), None);
    let closing = b"HTTP/1.1 503 Service Unavailable\r\n\r\nbusy";
    assert_eq!(parse_response(closing, false), None);
    assert_eq!(parse_response(closing, true), Some((503, b"busy".to_vec())));
    assert_eq!(parse_response(b"garbage", true), None);
}

#[test]
fn props_name_the_alias_and_model() {
    let body =
        br#"{"model_alias":"qingjian-local","model_path":"C:\\models\\a.gguf","total_slots":2}"#;
    assert_eq!(
        serving(body),
        Some(Probe::Serving {
            alias: "qingjian-local".to_owned(),
            model: "C:\\models\\a.gguf".to_owned(),
        })
    );
    assert_eq!(serving(br#"{"model_alias":"x"}"#), None);
    assert_eq!(serving(b"<html></html>"), None);
}

#[test]
fn probe_tells_free_loading_serving_and_other_apart() {
    assert_eq!(probe(free_port()), Probe::Free);
    let loading = FakeServer::start(503, r#"{"error":{"message":"Loading model"}}"#.to_owned());
    assert_eq!(probe(loading.port), Probe::Loading);
    let serving = FakeServer::props("/models/a.gguf");
    assert_eq!(
        probe(serving.port),
        Probe::Serving {
            alias: ALIAS.to_owned(),
            model: "/models/a.gguf".to_owned(),
        }
    );
    let other = FakeServer::start(404, "not found".to_owned());
    assert_eq!(probe(other.port), Probe::Other);
}

/// 输出都照 llama-server 出错时的原样。
#[test]
fn exit_reason_is_the_last_error_without_the_log_prefix() {
    let exit = exit_status();
    let reason = |lines: &[&str]| {
        let lines: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        super::supervisor::reason(&lines, exit)
    };
    // 端口被占：收尾那句「exiting due to」也是报错级别，但不说原因
    assert_eq!(
        reason(&[
            "0.00.002.536 W srv  server_setup: security: no API key is set and CORS allows all origins",
            "0.00.002.611 E srv         start: couldn't bind HTTP server socket, hostname: 127.0.0.1, port: 18686",
            "0.00.002.626 I srv    operator(): operator(): cleaning up before exit...",
            "0.00.002.722 E srv  server_run_i: exiting due to HTTP server error",
        ]),
        "srv start: couldn't bind HTTP server socket, hostname: 127.0.0.1, port: 18686"
    );
    // 模型文件坏了
    assert_eq!(
        reason(&[
            "0.00.004.363 E gguf_init_from_reader: invalid magic characters: 'not ', expected 'GGUF'",
            "0.00.004.412 E srv    load_model: failed to load model, '/models/broken.gguf'",
            "0.00.004.426 I srv    operator(): operator(): cleaning up before exit...",
            "0.00.004.774 E srv  server_run_i: exiting due to model loading error",
        ]),
        "srv load_model: failed to load model, '/models/broken.gguf'"
    );
    // 参数错不走日志，没有行首
    assert_eq!(
        reason(&["error: invalid argument: --no-such-flag"]),
        "error: invalid argument: --no-such-flag"
    );
    // 没有报错就取最后一行，再没有就是退出码
    assert_eq!(
        reason(&["0.00.010.000 I loading", "0.00.010.000 I hello   world"]),
        "hello world"
    );
    assert_eq!(reason(&[]), exit.to_string());
}

/// 一个真实的退出状态（`ExitStatus` 没有公开的构造方法）。
fn exit_status() -> std::process::ExitStatus {
    let mut command = if cfg!(windows) {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "exit 3"]);
        command
    } else {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "exit 3"]);
        command
    };
    command.status().unwrap()
}

#[test]
fn status_wakes_waiters_when_it_changes() {
    let status = Arc::new(Status::new());
    assert_eq!(status.wait(Duration::from_millis(20)), State::Starting);
    let setter = Arc::clone(&status);
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        setter.set(State::Ready);
    });
    assert_eq!(status.wait(Duration::from_secs(10)), State::Ready);
    thread.join().unwrap();
}

#[test]
fn missing_model_is_reported_without_starting_anything() {
    let server = LocalServer::lease(spec("/no/such/model.gguf", free_port()));
    assert_eq!(
        server.wait_ready(WAIT),
        Err(LocalServerError::ModelNotFound(
            "/no/such/model.gguf".to_owned()
        ))
    );
}

#[test]
fn missing_program_is_reported() {
    let dir = temp_dir("program");
    let model = dir.join("model.gguf");
    std::fs::write(&model, b"GGUF").unwrap();
    let mut spec = spec(model.to_str().unwrap(), free_port());
    spec.server = Some(dir.join("no-such-llama-server"));
    let server = LocalServer::lease(spec);
    assert_eq!(
        server.wait_ready(WAIT),
        Err(LocalServerError::ServerNotFound)
    );
    drop(server);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn another_program_on_the_port_is_reported() {
    let other = FakeServer::start(404, "not found".to_owned());
    let server = LocalServer::lease(spec("/models/a.gguf", other.port));
    assert_eq!(
        server.wait_ready(WAIT),
        Err(LocalServerError::PortTaken(other.port))
    );
}

#[test]
fn a_running_server_with_the_same_model_is_used_as_is() {
    let running = FakeServer::props("/models/shared.gguf");
    let server = LocalServer::lease(spec("/models/shared.gguf", running.port));
    assert_eq!(server.wait_ready(WAIT), Ok(()));
    assert_eq!(server.check(), Ok(true));
}

#[test]
fn clients_with_the_same_spec_share_one_server() {
    let port = free_port();
    let first = LocalServer::lease(spec("/no/such/shared.gguf", port));
    let second = LocalServer::lease(spec("/no/such/shared.gguf", port));
    assert!(Arc::ptr_eq(&first, &second));
    let other = LocalServer::lease(spec("/no/such/other.gguf", port));
    assert!(!Arc::ptr_eq(&first, &other));
}
