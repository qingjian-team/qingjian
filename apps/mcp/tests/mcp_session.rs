//! 集成测试：起真实 `qingjian-mcp` 进程（cwd 在仓库根，数据文件自动探测命中），
//! 走完整 MCP stdio 会话：initialize → tools/list → tools/call(lookup/gloss) → 未知方法。

#![allow(clippy::panic)] // 测试断言

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::Value;

/// 仓库根：`apps/mcp` 上两级。子进程 cwd 放这里，`default_data_file` 才能命中仓库数据。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("仓库根存在")
}

struct McpProcess {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl McpProcess {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qingjian-mcp"))
            .current_dir(repo_root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("拉起 qingjian-mcp");
        let stdin = BufWriter::new(child.stdin.take().expect("子进程 stdin"));
        let stdout = BufReader::new(child.stdout.take().expect("子进程 stdout"));
        Self {
            child,
            stdin,
            stdout,
        }
    }

    /// 发一帧请求，读回一帧响应。
    fn call(&mut self, method: &str, params: Value) -> Value {
        let body = serde_json::to_vec(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.stdin.write_all(&body).unwrap();
        self.stdin.flush().unwrap();
        read_frame(&mut self.stdout).expect("读到响应帧")
    }

    /// 关 stdin（子进程 EOF 退出），等退出码。
    fn close(mut self) {
        drop(self.stdin);
        let status = self.child.wait().expect("子进程退出");
        assert!(status.success(), "子进程异常退出");
    }
}

/// 读一帧（与 src/protocol.rs 同款解析）。
fn read_frame(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).unwrap();
        if n == 0 {
            return None;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = Some(rest.trim().parse::<usize>().unwrap());
        }
    }
    let len = content_length.expect("有 Content-Length");
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[test]
fn full_mcp_session_serves_lookup_and_gloss() {
    let mut server = McpProcess::spawn();

    // initialize
    let resp = server.call(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" },
        }),
    );
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["serverInfo"]["name"], "qingjian-mcp");
    assert!(resp["result"]["protocolVersion"].is_string());
    assert!(resp["result"]["capabilities"]["tools"].is_object());

    // tools/list
    let resp = server.call("tools/list", serde_json::json!({}));
    let names: Vec<String> = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"lookup".to_string()));
    assert!(names.contains(&"gloss".to_string()));

    // tools/call lookup：yishi 应有候选，且首个候选带译文（glossary-en 随仓库）
    let resp = server.call(
        "tools/call",
        serde_json::json!({
            "name": "lookup",
            "arguments": { "input": "yishi", "limit": 5 },
        }),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let data: Value = serde_json::from_str(text).unwrap();
    let candidates = data["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty(), "yishi 应有候选");
    assert!(
        data["segmentations"]
            .as_array()
            .unwrap()
            .contains(&Value::from("yi shi".to_string()))
    );
    assert!(candidates[0]["text"].is_string());

    // tools/call gloss：青简 应查得到译文
    let resp = server.call(
        "tools/call",
        serde_json::json!({
            "name": "gloss",
            "arguments": { "word": "青简" },
        }),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let data: Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["word"], "青简");
    assert!(data["translation"].is_object() || data["translation"].is_null());

    // 未知方法 → JSON-RPC -32601
    let resp = server.call("not-a-method", serde_json::json!({}));
    assert_eq!(resp["error"]["code"], -32601);

    server.close();
}

#[test]
fn lookup_respects_limit_and_fuzzy() {
    let mut server = McpProcess::spawn();
    let _ = server.call(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {},
        }),
    );
    let resp = server.call(
        "tools/call",
        serde_json::json!({
            "name": "lookup",
            "arguments": { "input": "yishi", "limit": 2 },
        }),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let data: Value = serde_json::from_str(text).unwrap();
    assert_eq!(data["candidates"].as_array().unwrap().len(), 2);
    server.close();
}
