//! 通过真实云联想 worker 和本地 HTTP 服务验证清空输入后的请求取消与间隔。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use qingjian_predict::{CloudPredictor, PredictConfig};

use crate::support::{
    Router, RouterConfig, function_key, letter, press, router_with, type_letters,
};

fn accept_until(listener: &TcpListener, timeout: Duration) -> Option<TcpStream> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => return Some(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => panic!("accept: {error}"),
        }
        if started.elapsed() >= timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn clearing_input_cancels_pending_requests(clear: impl Fn(&mut Router)) {
    for sentence_translation in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let mut router = router_with(RouterConfig {
            cloud_enabled: true,
            sentence_translation,
            ..RouterConfig::default()
        });
        let predictor = CloudPredictor::new(&PredictConfig {
            enabled: true,
            base_url: format!("http://{}", listener.local_addr().unwrap()),
            api_key: Some("test".into()),
            debounce_ms: 300,
            min_interval_ms: 3000,
            sentence_translation,
            ..PredictConfig::default()
        })
        .unwrap();
        router.engine_mut().set_predictor(Box::new(predictor));

        type_letters(&mut router, "xiangkaifa");
        clear(&mut router);
        assert!(router.engine_mut().composition().is_empty());
        assert!(
            accept_until(&listener, Duration::from_millis(600)).is_none(),
            "清空输入后不应发出联想或译文请求（整句翻译：{sentence_translation}）"
        );

        // 新输入应只等停键，不能被已取消的输入占掉 3 秒网络间隔。
        type_letters(&mut router, "nihao");
        let mut stream = accept_until(&listener, Duration::from_millis(1500))
            .expect("下一次联想不应等待已取消请求的 3 秒间隔");
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let body = String::from_utf8(body).unwrap();
        assert!(body.contains("nihao"), "实际发出的应是新输入：{body}");
        assert!(!body.contains("xiangkaifa"));
        let response = r#"{"id":"test","object":"chat.completion","created":0,"model":"test","choices":[{"index":0,"finish_reason":"stop","message":{"role":"assistant","content":"{\"words\":[]}"}}]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).unwrap();
    }
}

#[test]
fn quick_commit_cancels_pending_cloud_requests() {
    clearing_input_cancels_pending_requests(|router| {
        let (_, commit, frame) = press(router, letter(' '));
        assert_eq!(commit.as_deref(), Some("想开发"));
        assert!(frame.is_empty());
    });
}

#[test]
fn deleting_all_pinyin_cancels_pending_cloud_requests() {
    clearing_input_cancels_pending_requests(|router| {
        for _ in "xiangkaifa".chars() {
            press(router, function_key(0x08));
        }
    });
}

#[test]
fn escape_cancels_pending_cloud_requests() {
    clearing_input_cancels_pending_requests(|router| {
        let (_, commit, frame) = press(router, function_key(0x1B));
        assert!(commit.is_none());
        assert!(frame.is_empty());
    });
}
