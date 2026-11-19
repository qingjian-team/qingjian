//! 后台线程：起 llama-server、等它加载好、看着它，退出了隔一阵再起；租约放手时停掉。
//! 端口上已有同一模型的服务（别的青简进程起的）就直接用，它没了再自己起。

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use super::error::LocalServerError;
use super::locate::locate;
use super::probe::{Probe, probe};
use super::spec::{ALIAS, LaunchSpec};
use super::status::{State, Status};
use super::tail::Tail;
use super::watchdog::Watchdog;

/// 加载中多久看一次。
const STARTING_POLL: Duration = Duration::from_millis(200);

/// 自己起的服务能用以后，多久看一次它还在不在。
const RUNNING_POLL: Duration = Duration::from_millis(500);

/// 用着别的进程起的服务时，多久看一次它还在不在。
const ADOPTED_POLL: Duration = Duration::from_secs(2);

/// 换模型时端口上还是青简起的旧模型，最多等它这么久让出端口。
const HANDOFF: Duration = Duration::from_secs(15);

/// 端口上的程序不答话（多半是别的青简进程起的、正在起或正在退出的服务），最多等它这么久再算端口被占。
const SILENT: Duration = Duration::from_secs(10);

/// 失败后第一次重试的间隔；之后每次翻倍，最长 [`RETRY_MAX`]。
const RETRY_FIRST: Duration = Duration::from_secs(2);

const RETRY_MAX: Duration = Duration::from_secs(60);

/// 服务稳稳跑过这么久再退出，重试间隔从头算。
const STABLE: Duration = Duration::from_secs(60);

/// 设成服务的环境变量就会要密钥；青简的请求不带密钥，会被当成密钥无效暂停 10 分钟，所以起服务时去掉
/// （`local_env` 里写了也去掉）。
const KEY_VARIABLES: [&str; 2] = ["LLAMA_API_KEY", "LLAMA_ARG_API_KEY_FILE"];

pub(crate) struct Supervisor {
    spec: LaunchSpec,

    status: Arc<Status>,

    /// 租约放手时断开；等待都在它上面等，一断开马上收工。
    stop: Receiver<()>,

    /// 看门进程靠不住（一起来就把服务结束了）时不再用。
    watchdog: bool,
}

/// 一轮看护的结局。
enum Round {
    /// 租约放手了，线程退出。
    Stopped,

    /// 端口上的情形变了（用着的服务没了、在等的服务加载好了），马上重新看端口。
    Again,

    /// 起不来或退出了；`stable` 表示退出前稳稳跑过一阵。
    Failed {
        error: LocalServerError,
        stable: bool,
    },
}

impl Round {
    fn failed(error: LocalServerError) -> Self {
        Self::Failed {
            error,
            stable: false,
        }
    }
}

impl Supervisor {
    pub fn new(spec: LaunchSpec, status: Arc<Status>, stop: Receiver<()>) -> Self {
        Self {
            spec,
            status,
            stop,
            watchdog: true,
        }
    }

    pub fn run(mut self) {
        let mut retry = RETRY_FIRST;
        loop {
            let (error, stable) = match self.round() {
                Round::Stopped => return,
                Round::Again => continue,
                Round::Failed { error, stable } => (error, stable),
            };
            if stable {
                retry = RETRY_FIRST;
            }
            tracing::warn!(%error, retry_secs = retry.as_secs(), "local model server unavailable");
            self.status.set(State::Failed(error));
            if self.pause(retry) {
                return;
            }
            retry = (retry * 2).min(RETRY_MAX);
        }
    }

    /// 看一眼端口再决定：空着就自己起，已是这个模型就直接用，是旧模型就等它让出来。
    fn round(&mut self) -> Round {
        match probe(self.spec.port) {
            Probe::Free => match self.prepare() {
                Ok(program) => self.serve(&program),
                Err(error) => Round::failed(error),
            },
            Probe::Serving { model, .. } if model == self.spec.model => self.adopt(),
            Probe::Serving { alias, .. } if alias == ALIAS => self.handoff(),
            Probe::Loading => {
                self.status.set(State::Starting);
                if self.pause(STARTING_POLL) {
                    Round::Stopped
                } else {
                    Round::Again
                }
            }
            Probe::Silent => self.settle(),
            Probe::Serving { .. } | Probe::Other => {
                Round::failed(LocalServerError::PortTaken(self.spec.port))
            }
        }
    }

    /// 起服务之前：模型文件在，程序找得到。
    fn prepare(&self) -> Result<PathBuf, LocalServerError> {
        if !Path::new(&self.spec.model).is_file() {
            return Err(LocalServerError::ModelNotFound(self.spec.model.clone()));
        }
        locate(self.spec.server.as_deref()).ok_or(LocalServerError::ServerNotFound)
    }

    /// 自己起一个，看着它直到它退出或租约放手。
    fn serve(&mut self, program: &Path) -> Round {
        tracing::info!(
            program = %program.display(),
            model = %self.spec.model,
            port = self.spec.port,
            "starting local model server"
        );
        self.status.set(State::Starting);
        let mut child = match spawn(program, &self.spec) {
            Ok(child) => child,
            Err(error) => return Round::failed(LocalServerError::Spawn(error.to_string())),
        };
        let tail = Tail::follow(child.stderr.take());
        let mut watchdog = self.guard(child.id());
        let started = Instant::now();
        let mut ready = false;
        loop {
            // 管道还开着，看门进程却自己结束了，多半已经结束了服务：这个平台上它靠不住，以后不用
            if watchdog.as_mut().is_some_and(Watchdog::fired) {
                tracing::warn!(
                    "watchdog of the local model server exited early, not using it again"
                );
                watchdog = None;
                self.watchdog = false;
            }
            if let Ok(Some(exit)) = child.try_wait() {
                drop(watchdog);
                let lines = tail.finish();
                tracing::warn!(%exit, output = %lines.join("\n"), "local model server exited");
                return Round::Failed {
                    error: LocalServerError::Exited(reason(&lines, exit)),
                    stable: started.elapsed() >= STABLE,
                };
            }
            if !ready && matches!(probe(self.spec.port), Probe::Serving { .. }) {
                ready = true;
                self.status.set(State::Ready);
                tracing::info!(
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "local model server ready"
                );
            }
            if self.pause(if ready { RUNNING_POLL } else { STARTING_POLL }) {
                drop(watchdog);
                let _ = child.kill();
                let _ = child.wait();
                tracing::info!("local model server stopped");
                return Round::Stopped;
            }
        }
    }

    /// 端口上已是这个模型（别的青简进程起的）：直接用，隔一阵看它还在不在。
    fn adopt(&mut self) -> Round {
        tracing::info!(
            port = self.spec.port,
            "using the local model server another process started"
        );
        loop {
            self.status.set(State::Ready);
            if self.pause(ADOPTED_POLL) {
                return Round::Stopped;
            }
            match probe(self.spec.port) {
                Probe::Serving { model, .. } if model == self.spec.model => {}
                _ => return Round::Again,
            }
        }
    }

    /// 端口上是青简起的另一个模型：多半是换模型之前的那个还没停，等它让出端口。
    fn handoff(&mut self) -> Round {
        self.status.set(State::Starting);
        let deadline = Instant::now() + HANDOFF;
        while Instant::now() < deadline {
            if self.pause(STARTING_POLL) {
                return Round::Stopped;
            }
            match probe(self.spec.port) {
                Probe::Serving { alias, model } if alias == ALIAS && model != self.spec.model => {}
                _ => return Round::Again,
            }
        }
        Round::failed(LocalServerError::PortTaken(self.spec.port))
    }

    /// 端口上有程序却不答话：别的青简进程起的服务加载失败、正在退出时就是这样，等它有个结果；
    /// 一直不答话才算端口被别的程序占着。
    fn settle(&mut self) -> Round {
        self.status.set(State::Starting);
        let deadline = Instant::now() + SILENT;
        while Instant::now() < deadline {
            if self.pause(STARTING_POLL) {
                return Round::Stopped;
            }
            if probe(self.spec.port) != Probe::Silent {
                return Round::Again;
            }
        }
        Round::failed(LocalServerError::PortTaken(self.spec.port))
    }

    /// 起看门进程；起不来只记一笔，服务照起。
    fn guard(&self, pid: u32) -> Option<Watchdog> {
        if !self.watchdog {
            return None;
        }
        Watchdog::guard(pid)
            .inspect_err(|error| {
                tracing::warn!(%error, "cannot start the watchdog of the local model server");
            })
            .ok()
    }

    /// 等一会儿；租约放手了返回 `true`。
    fn pause(&self, duration: Duration) -> bool {
        !matches!(
            self.stop.recv_timeout(duration),
            Err(RecvTimeoutError::Timeout)
        )
    }
}

/// 起 llama-server：输出全在 stderr，接管道；工作目录设在程序旁边（Windows 版的 DLL 在那里）。
/// 环境变量在输入法自己的环境上加用户给的那些。
fn spawn(program: &Path, spec: &LaunchSpec) -> io::Result<Child> {
    let mut command = Command::new(program);
    command
        .args(spec.arguments())
        .envs(&spec.env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    for variable in KEY_VARIABLES {
        command.env_remove(variable);
    }
    if let Some(dir) = program.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        command.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 控制台程序：不加这个会弹出一个黑窗口
        command.creation_flags(super::CREATE_NO_WINDOW);
    }
    command.spawn()
}

/// 退出原因：输出里最后一条报错，没有就最后一行，再没有就是退出码。不算的报错：收尾的「exiting due to …」
/// （不说原因）与缩进的说明行（CUDA 报错后面的「current device: 1, …」与出错的调用，原因在它们前面那条）。
pub(crate) fn reason(lines: &[String], exit: ExitStatus) -> String {
    let lines: Vec<Line> = lines
        .iter()
        .map(|line| parse_line(line))
        .filter(|line| !line.text.is_empty())
        .collect();
    lines
        .iter()
        .rev()
        .find(|line| line.error && !line.detail && !line.text.contains("exiting due to"))
        .or(lines.last())
        .map_or_else(|| exit.to_string(), |line| line.text.clone())
}

/// 拆开的一行输出。
struct Line {
    /// 级别 E 的日志行，或不走日志、含 error / failed 的行。
    error: bool,

    /// 正文缩进的日志行：接在上一条后面的说明。
    detail: bool,

    /// 去掉行首的时间与级别、连续空白并成一个的正文。
    text: String,
}

/// llama.cpp 的日志行是 `0.00.002.611 E srv  start: …`，级别 E 是报错；参数错这类不走日志的输出没有行首。
fn parse_line(line: &str) -> Line {
    let line = line.trim();
    let mut parts = line.splitn(3, ' ');
    let (error, detail, text) = match (parts.next(), parts.next(), parts.next()) {
        (Some(time), Some(level), Some(rest))
            if !time.is_empty()
                && time.chars().all(|c| c.is_ascii_digit() || c == '.')
                && level.len() == 1 =>
        {
            (level == "E", rest.starts_with(char::is_whitespace), rest)
        }
        _ => {
            let lower = line.to_ascii_lowercase();
            (
                lower.contains("error") || lower.contains("failed"),
                false,
                line,
            )
        }
    };
    Line {
        error,
        detail,
        text: text.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}
