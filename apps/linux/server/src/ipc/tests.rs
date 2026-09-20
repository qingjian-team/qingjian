//! 真实连接处理器与 Router 的资源回收；不增加产品诊断消息。
use super::{Request, connection::serve_connection};
use crate::dispatch::{Router, RouterConfig};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use std::os::unix::net::UnixStream;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

#[test]
fn close_and_disconnect_reclaim_every_router_session() {
    let (sender, receiver) = mpsc::sync_channel::<Request>(128);
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let worker = thread::spawn(move || {
        let mut router = Router::new(
            Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()),
            RouterConfig::default(),
        );
        for (message, reply) in receiver {
            let response = router.handle_linux(message);
            observed.store(router.session_count(), Ordering::SeqCst);
            let _ = reply.send(response);
        }
        assert_eq!(router.session_count(), 0);
    });
    let (mut client, server) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let connection = thread::spawn(move || serve_connection(server, sender, json!({"version": 3})));
    let open = |client: &mut UnixStream, id| {
        write_message(
            client,
            &json!({"OpenSession": {"session": id, "app": null, "protocol": PROTOCOL_VERSION}}),
        )
        .unwrap();
        assert_eq!(
            read_message::<_, Value>(client).unwrap().unwrap()["Update"]["session"],
            id
        );
    };
    for id in 1..=128 {
        open(&mut client, id);
    }
    assert_eq!(count.load(Ordering::SeqCst), 128);
    for id in 1..=128 {
        write_message(&mut client, &json!({"CloseSession": {"session": id}})).unwrap();
    }
    open(&mut client, 129); // 所有无回包通知已完成的顺序屏障。
    assert_eq!(count.load(Ordering::SeqCst), 1);
    for id in 130..=256 {
        open(&mut client, id);
    }
    drop(client);
    connection.join().unwrap();
    worker.join().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
