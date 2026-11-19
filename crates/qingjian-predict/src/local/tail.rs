//! llama-server 的输出（全在 stderr）：逐行转进调试日志，留最后几十行，服务意外退出时拿来说明原因。
//! 缺省的输出只有加载过程与每次请求的耗时，不含请求的原文。

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::ChildStderr;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

/// 留多少行。
const KEEP: usize = 40;

pub(crate) struct Tail {
    /// 最近的几十行，旧的在前。
    lines: Arc<Mutex<VecDeque<String>>>,

    /// 读输出的线程，管道到头时自己结束。
    reader: Option<JoinHandle<()>>,
}

impl Tail {
    /// 起一个线程读 `stderr`；没有管道（理论上不会）就什么也不留。
    pub fn follow(stderr: Option<ChildStderr>) -> Self {
        let lines = Arc::new(Mutex::new(VecDeque::with_capacity(KEEP)));
        let reader = stderr.and_then(|stderr| {
            let lines = Arc::clone(&lines);
            std::thread::Builder::new()
                .name("qingjian-llama-log".to_owned())
                .spawn(move || read(stderr, &lines))
                .ok()
        });
        Self { lines, reader }
    }

    /// 服务退出后调：等读线程把管道里剩下的读完，返回留下的行。
    pub fn finish(mut self) -> Vec<String> {
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }
}

fn read(stderr: ChildStderr, lines: &Mutex<VecDeque<String>>) {
    let mut reader = BufReader::new(stderr);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&buffer).trim_end().to_owned();
        if line.is_empty() {
            continue;
        }
        tracing::debug!(%line, "llama-server");
        let mut lines = lines.lock().unwrap_or_else(PoisonError::into_inner);
        if lines.len() == KEEP {
            lines.pop_front();
        }
        lines.push_back(line);
    }
}
