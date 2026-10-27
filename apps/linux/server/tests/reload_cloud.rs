//! 真实 Server 热切换云服务地址、模型、密钥与开关，旧服务的迟到回复不能进入候选。
#![cfg(target_os = "linux")]

#[path = "support/server.rs"]
mod server;

use qingjian_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use server::Server;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

fn endpoint(
    model: &str,
    key: &str,
) -> (String, Receiver<()>, Sender<&'static str>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let model = model.to_owned();
    let key = key.to_owned();
    let (requested, received) = mpsc::channel();
    let (respond, response) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let started = Instant::now();
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(started.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(10));
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut headers = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(!line.is_empty());
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line.to_ascii_lowercase());
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        assert!(headers.contains(&format!("authorization: bearer {key}")));
        assert_eq!(
            serde_json::from_slice::<Value>(&body).unwrap()["model"],
            model
        );
        requested.send(()).unwrap();
        let sentence = response.recv_timeout(Duration::from_secs(10)).unwrap();
        let content =
            json!({"words":[{"text":"妮","pinyin":"ni"}],"sentence":sentence}).to_string();
        let body = json!({"id":"test","object":"chat.completion","created":1,"model":model,"choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]}).to_string();
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    });
    (address, received, respond, worker)
}

fn config(path: &Path, source: &str, revision: u64) {
    std::fs::write(path, source).unwrap();
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(revision))
        .unwrap();
}

fn until(stream: &mut UnixStream, mut ready: impl FnMut(&Value) -> bool) -> Value {
    let started = Instant::now();
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "cloud config did not reload"
        );
        write_message(stream, &json!({"Poll":{"session":1}})).unwrap();
        let message = read_message::<_, Value>(stream).unwrap().unwrap();
        let frame = &message["Update"]["frame"];
        if ready(frame) {
            return frame.clone();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn reload_rebuilds_cloud_and_discards_old_service_reply() {
    let directory =
        std::env::temp_dir().join(format!("qingjian-cloud-reload-{}", std::process::id()));
    let path = directory.join("config/qingjian/config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    config(&path, "[predict]\nenabled = false\n", 100);
    let mut server = Server::start(directory.clone());
    let mut stream = server.connect();
    for message in [
        json!({"OpenSession":{"session":1,"app":null,"protocol":PROTOCOL_VERSION}}),
        json!({"LinuxHello":{"version":3,"session":1,"generation":1,"context":"reload-cloud"}}),
        json!({"LinuxEvent":{"session":1,"event":{"Capabilities":{"sensitive":false,"password":false,"disabled":false}}}}),
    ] {
        write_message(&mut stream, &message).unwrap();
        read_message::<_, Value>(&mut stream).unwrap().unwrap();
    }
    for c in "ni".chars() {
        write_message(&mut stream, &json!({"LinuxEvent":{"session":1,"event":{"Key":{"release":false,"event":{"virtual_key":c as u32,"character":c,"modifiers":{"ctrl":false,"shift":false,"alt":false,"win":false,"caps":false,"english_mode":false}}}}}})).unwrap();
        read_message::<_, Value>(&mut stream).unwrap().unwrap();
    }
    let (first_url, first_request, first_reply, first_worker) = endpoint("first", "first-key");
    let (second_url, second_request, second_reply, second_worker) =
        endpoint("second", "second-key");
    let source = |url: &str, model: &str, slots: usize| {
        format!(
            "[predict]\nenabled = true\nbase_url = {url:?}\nmodel = {model:?}\napi_key = \"{model}-key\"\nslots = {slots}\ndebounce_ms = 0\nmin_interval_ms = 0\nreasoning_effort = \"\"\n"
        )
    };
    config(&path, &source(&first_url, "first", 2), 200);
    until(&mut stream, |_| first_request.try_recv().is_ok());
    config(&path, &source(&second_url, "second", 1), 300);
    let frame = until(&mut stream, |_| second_request.try_recv().is_ok());
    assert!(frame["sentence"].is_null());
    first_reply.send("旧服务结果").unwrap();
    second_reply.send("你好").unwrap();
    let frame = until(&mut stream, |frame| frame["sentence"] == "你好");
    assert!(
        frame["candidates"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["kind"] == "Cloud")
    );
    config(&path, "[predict]\nenabled = false\n", 400);
    until(&mut stream, |frame| {
        frame["sentence"].is_null()
            && frame["candidates"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["kind"] != "Cloud")
    });
    first_worker.join().unwrap();
    second_worker.join().unwrap();
    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}

/// 假服务：同一端口收任意多个请求，每个请求体转给通道，回固定的词 + 整句。
/// 往返回的发送端发一条即停（worker 持有监听器的克隆，光 drop 原件关不掉它）。
fn loop_endpoint(key: &str) -> (String, Receiver<Value>, Sender<()>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let handle = listener.try_clone().unwrap();
    let key = key.to_owned();
    let (requested, received) = mpsc::channel();
    let (stop, stop_signal) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        loop {
            if stop_signal.try_recv().is_ok() {
                break;
            }
            let mut stream = match handle.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut headers = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                let Ok(read) = reader.read_line(&mut line) else {
                    break;
                };
                if read == 0 {
                    break;
                }
                headers.push_str(&line.to_ascii_lowercase());
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
                if line == "\r\n" {
                    let body = if length == 0 {
                        Vec::new()
                    } else {
                        let mut body = vec![0; length];
                        if reader.read_exact(&mut body).is_err() {
                            break;
                        }
                        body
                    };
                    if !headers.contains(&format!("authorization: bearer {key}")) {
                        break;
                    }
                    let Ok(request) = serde_json::from_slice::<Value>(&body) else {
                        break;
                    };
                    if requested.send(request).is_err() {
                        break;
                    }
                    let content = json!({"words":[{"text":"妮","pinyin":"ni"}],"sentence":"你好"})
                        .to_string();
                    let reply = json!({"id":"test","object":"chat.completion","created":1,"model":"combo","choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]}).to_string();
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                        reply.len()
                    );
                    break;
                }
            }
        }
    });
    (address, received, stop, worker)
}

fn press_combo(stream: &mut UnixStream, key: char, ctrl: bool, alt: bool, before: &str) {
    write_message(
        stream,
        &json!({"LinuxEvent":{"session":1,"event":{"Key":{"release":false,"surrounding":{"before":before,"after":""},"event":{"virtual_key":key as u32,"character":key,"modifiers":{"ctrl":ctrl,"shift":false,"alt":alt,"win":false,"caps":false,"english_mode":false}}}}}}),
    )
    .unwrap();
    read_message::<_, Value>(stream).unwrap().unwrap();
}

fn press_letter(stream: &mut UnixStream, key: char) {
    press_combo(stream, key, false, false, "");
}

#[test]
fn reload_picks_up_the_new_predict_combo() {
    let directory =
        std::env::temp_dir().join(format!("qingjian-combo-reload-{}", std::process::id()));
    let path = directory.join("config/qingjian/config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let (url, requests, stop, worker) = loop_endpoint("reload-combo-key");
    let source = |url: &str, appearance: Option<&str>, combo: Option<&str>| {
        let general = appearance
            .map(|value| format!("\n[general]\nappearance = {value:?}\n"))
            .unwrap_or_default();
        let shortcut = combo
            .map(|value| format!("\n[shortcut]\npredict = {value:?}\n"))
            .unwrap_or_default();
        format!(
            "[predict]\nenabled = true\nbase_url = {url:?}\nmodel = \"combo\"\napi_key = \"reload-combo-key\"\ndebounce_ms = 0\nmin_interval_ms = 0\nreasoning_effort = \"\"\n{general}{shortcut}"
        )
    };
    config(&path, &source(&url, None, None), 100);
    let mut server = Server::start(directory.clone());
    let mut stream = server.connect();
    for message in [
        json!({"OpenSession":{"session":1,"app":null,"protocol":PROTOCOL_VERSION}}),
        json!({"LinuxHello":{"version":3,"session":1,"generation":1,"context":"combo-reload"}}),
        json!({"LinuxEvent":{"session":1,"event":{"Capabilities":{"sensitive":false,"password":false,"disabled":false}}}}),
    ] {
        write_message(&mut stream, &message).unwrap();
        read_message::<_, Value>(&mut stream).unwrap().unwrap();
    }
    for c in "ni".chars() {
        press_letter(&mut stream, c);
    }
    let automatic = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(automatic["model"], "combo");
    // 缺省组合键 Ctrl + Alt + J 触发按需联想。带上下文且每次不同：
    // 缓存键含上下文，自动请求先完成并进缓存后，按需请求照样真实发网。
    press_combo(&mut stream, 'j', true, true, "前文一");
    requests
        .recv_timeout(Duration::from_secs(5))
        .expect("缺省组合键要触发按需联想");
    // 换组合键（[predict] 不动，只改 [shortcut] 与外观）：外观进帧即热加载已应用。
    config(
        &path,
        &source(&url, Some("dark"), Some("control+alt+k")),
        200,
    );
    until(&mut stream, |frame| frame["theme"] == "dark");
    press_combo(&mut stream, 'k', true, true, "前文二");
    requests
        .recv_timeout(Duration::from_secs(5))
        .expect("新组合键要触发按需联想");
    // 旧组合键退役；配一份没用过的上下文，万一份错也躲不过缓存。
    press_combo(&mut stream, 'j', true, true, "前文三");
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        requests.try_recv().is_err(),
        "旧组合键热加载后不再触发按需联想"
    );
    drop(server);
    stop.send(()).unwrap();
    worker.join().unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
