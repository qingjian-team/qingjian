//! 真实 Linux Server 子进程与独立资源目录。
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct Server {
    child: Child,
    directory: PathBuf,
}
impl Server {
    pub fn start(directory: PathBuf) -> Self {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let root = directory.join("resources");
        std::fs::create_dir_all(root.join("assets/sample")).unwrap();
        std::fs::copy(
            repository.join("assets/sample/dict.tsv"),
            root.join("assets/sample/dict.tsv"),
        )
        .unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_qingjian-linux-server"))
            .env("QINGJIAN_SOCKET", directory.join("server.sock"))
            .env("QINGJIAN_RESOURCES", &root)
            .env("QINGJIAN_DICT", root.join("assets/sample/dict.tsv"))
            .env("XDG_CONFIG_HOME", directory.join("config"))
            .env("XDG_DATA_HOME", directory.join("data"))
            .env("XDG_STATE_HOME", directory.join("state"))
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        Self { child, directory }
    }
    pub fn connect(&mut self) -> UnixStream {
        let started = Instant::now();
        loop {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "server exited early"
            );
            if let Ok(stream) = UnixStream::connect(self.directory.join("server.sock")) {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                return stream;
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
