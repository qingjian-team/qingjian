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
            &json!({"LinuxHello": {"version": 2, "generation": 1, "context": "config-test"}}),
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
