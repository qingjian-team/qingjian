//! 实际启动 Server 读取 TOML 配置，关闭辅助语言不应回落英文释义。
#![cfg(target_os = "linux")]
#[path = "support/server.rs"]
mod server;
use qingjian_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use server::Server;

#[test]
fn learning_language_off_disables_annotations_in_real_server() {
    for (language, expected) in [("en", true), ("off", false), (" OFF ", false)] {
        let directory = std::env::temp_dir().join(format!(
            "qingjian-config-{}-{}",
            std::process::id(),
            language.trim()
        ));
        std::fs::create_dir_all(directory.join("config/qingjian")).unwrap();
        std::fs::create_dir_all(directory.join("resources/assets/glossary")).unwrap();
        std::fs::write(
            directory.join("resources/assets/glossary/glossary-en.tsv"),
            "你好\thello\n",
        )
        .unwrap();
        std::fs::write(
            directory.join("config/qingjian/config.toml"),
            format!("[general]\nlearning_language = {language:?}\n"),
        )
        .unwrap();
        let mut server = Server::start(directory.clone());
        let mut stream = server.connect();
        write_message(
            &mut stream,
            &json!({"OpenSession": {"session": 1, "app": null, "protocol": PROTOCOL_VERSION}}),
        )
        .unwrap();
        read_message::<_, Value>(&mut stream).unwrap();
        write_message(
            &mut stream,
            &json!({"LinuxHello": {"version": 3, "session": 1, "generation": 1, "context": "config-test"}}),
        )
        .unwrap();
        read_message::<_, Value>(&mut stream).unwrap();
        write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {"sensitive": false, "password": false, "disabled": false}}}})).unwrap();
        read_message::<_, Value>(&mut stream).unwrap();
        let mut result = Value::Null;
        for character in "nihao".chars() {
            write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Key": {"release": false, "event": {"virtual_key": character as u32, "character": character, "modifiers": {"ctrl": false, "shift": false, "alt": false, "win": false, "caps": false, "english_mode": false}}}}}})).unwrap();
            result = read_message::<_, Value>(&mut stream).unwrap().unwrap();
        }
        let items = result["KeyResult"]["frame"]["candidates"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(items[0]["text"], "你好");
        if expected {
            assert_eq!(items[0]["translation"]["senses"][0]["text"], "hello");
        } else {
            assert!(
                items
                    .iter()
                    .all(|candidate| candidate["translation"].is_null()),
                "{result}"
            );
        }
        drop(server);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn enabled_predict_config_reaches_local_openai_endpoint() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let endpoint = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut request = vec![0; length];
        reader.read_exact(&mut request).unwrap();
        let request = String::from_utf8(request).unwrap();
        assert!(request.contains("ni"));
        assert!(request.contains("这是前文"));
        assert!(request.contains("后文"));
        let content = r#"{"words":[{"text":"妮","pinyin":"ni"}],"sentence":"你好"}"#;
        let body = json!({"id":"chatcmpl-test","object":"chat.completion","created":1,"model":"test","choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]}).to_string();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });

    let directory =
        std::env::temp_dir().join(format!("qingjian-cloud-config-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("config/qingjian")).unwrap();
    std::fs::write(directory.join("config/qingjian/config.toml"), format!(
        "[predict]\nenabled = true\nbase_url = \"http://{address}\"\nmodel = \"test\"\napi_key = \"test-key\"\ndebounce_ms = 0\nreasoning_effort = \"\"\n"
    )).unwrap();
    let mut server = Server::start(directory);
    let mut stream = server.connect();
    write_message(
        &mut stream,
        &json!({"OpenSession": {"session": 1, "app": null, "protocol": PROTOCOL_VERSION}}),
    )
    .unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    write_message(&mut stream, &json!({"LinuxHello": {"version": 3, "session": 1, "generation": 1, "context": "cloud-config-test"}})).unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {"sensitive": false, "password": false, "disabled": false}}}})).unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    for character in "ni".chars() {
        write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Key": {"release": false, "surrounding": {"before": "这是前文", "after": "后文"}, "event": {"virtual_key": character as u32, "character": character, "modifiers": {"ctrl": false, "shift": false, "alt": false, "win": false, "caps": false, "english_mode": false}}}}}})).unwrap();
        read_message::<_, Value>(&mut stream).unwrap();
    }
    let started = Instant::now();
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "cloud reply did not reach Linux frame"
        );
        write_message(&mut stream, &json!({"Poll": {"session": 1}})).unwrap();
        let result = read_message::<_, Value>(&mut stream).unwrap().unwrap();
        let frame = &result["Update"]["frame"];
        if frame["sentence"] == "你好" {
            assert!(
                frame["candidates"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["text"] == "妮" && item["kind"] == "Cloud")
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    endpoint.join().unwrap();
}
