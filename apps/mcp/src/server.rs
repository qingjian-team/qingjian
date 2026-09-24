//! MCP 会话主循环：stdio 读帧 → JSON-RPC 分发 → 写帧。
//! 首版支持 initialize / notifications/initialized / tools/list / tools/call / ping。

use std::io::{self, BufReader, BufWriter};

use serde_json::json;

use crate::error::Error;
use crate::protocol::{self, Request};
use crate::tools::Tools;

/// MCP 协议版本：固定回一个已发布版本（客户端通常要求不低于它）。
const PROTOCOL_VERSION: &str = "2025-06-18";

/// 阻塞服务：直到 stdin EOF。
pub fn run(tools: Tools) -> Result<(), Error> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    while let Some(msg) = protocol::read_frame(&mut reader)? {
        let request = match protocol::parse_request(&msg) {
            Ok(request) => request,
            Err(Some((id, code, message))) => {
                protocol::write_frame(&mut writer, &protocol::error_response(id, code, &message))?;
                continue;
            }
            Err(None) => continue,
        };
        // notification 不回响应
        let Some(id) = request.id.clone() else {
            continue;
        };
        match dispatch(&tools, &request) {
            Ok(result) => {
                protocol::write_frame(&mut writer, &protocol::response(id, result))?;
            }
            Err((code, message)) => {
                protocol::write_frame(&mut writer, &protocol::error_response(id, code, &message))?;
            }
        }
    }
    Ok(())
}

/// 方法分发。错误带 JSON-RPC 码：工具调用失败 -32000，未知方法 -32601。
fn dispatch(tools: &Tools, request: &Request) -> Result<serde_json::Value, (i32, String)> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "qingjian-mcp", "version": env!("CARGO_PKG_VERSION") },
        })),
        "tools/list" => Ok(Tools::list()),
        "tools/call" => tools
            .call(&request.params)
            .map_err(|e| (-32000, e.to_string())),
        "ping" => Ok(json!({})),
        _ => Err((-32601, format!("未知方法 {}", request.method))),
    }
}
