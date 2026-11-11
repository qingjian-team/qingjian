//! 配置读取被阻塞时，输入线程仍能处理按键。

use std::ffi::CString;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};

use super::{CONFIG_POLL_INTERVAL, poll, temp_dir, watched};

#[test]
fn blocked_config_read_does_not_block_keys() {
    let directory = temp_dir("blocked-config");
    let mut router = watched(&directory, "[general]\npage_size = 5\n");
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    let path = directory.join("config.toml");
    std::fs::remove_file(&path).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    let (sender, receiver) = mpsc::channel();
    let input = thread::spawn(move || {
        let started = Instant::now();
        router.tick();
        assert!(router.next_tick() <= Duration::from_millis(20));
        for _ in 0..20 {
            router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
            router.tick();
        }
        router.handle(ClientMessage::Key {
            session: SessionId(1),
            event: KeyEvent::new(78, Some('n'), KeyModifiers::default()),
        });
        sender.send((router, started.elapsed())).unwrap();
    });
    let early = receiver.recv_timeout(Duration::from_millis(150));
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut writer = loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(writer) => break writer,
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
                assert!(
                    Instant::now() < deadline,
                    "reload never opened the configuration"
                );
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("cannot release blocked configuration: {error}"),
        }
    };
    writer.write_all(b"[general]\npage_size = 7\n").unwrap();
    drop(writer);
    let (mut router, elapsed) =
        early.unwrap_or_else(|_| receiver.recv_timeout(Duration::from_secs(3)).unwrap());
    input.join().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, "[general]\npage_size = 7\n").unwrap();
    poll(&mut router);
    assert_eq!(router.engine.composition().text(), "n");
    assert_eq!(router.config.page_size, 7);
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        elapsed < Duration::from_millis(100),
        "input blocked for {elapsed:?}"
    );
}
