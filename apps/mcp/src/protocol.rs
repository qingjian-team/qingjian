//! MCP 传输层：stdio 上的 JSON-RPC 2.0 消息，Content-Length 分帧（LSP 风格，MCP 规范同款）。
//! 只实现首版要的：request / response / notification 三种消息与 `Content-Length` 头。
//! 其余头（Content-Type 等）忽略；多个消息可在同一帧（不常见，分开解析）。

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::error::Error;

/// 从 `reader` 读一帧：`Content-Length: N` 头 + 空行 + N 字节 JSON。EOF 返回 `None`。
pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<Value>, Error> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            let len = rest
                .trim()
                .parse::<usize>()
                .map_err(|_| Error::Protocol("Content-Length 不是数字".into()))?;
            content_length = Some(len);
        }
        // 其它头忽略
    }
    let len = content_length.ok_or_else(|| Error::Protocol("帧缺少 Content-Length 头".into()))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(Error::from)
}

/// 把 `value` 作为一帧写给 `writer`。
pub fn write_frame(writer: &mut impl Write, value: &Value) -> Result<(), Error> {
    let body = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}

/// 成功响应。`id` 从请求带回来，可能是数字、字符串或 null。
pub fn response(id: Value, result: Value) -> Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// 错误响应。JSON-RPC 码：-32700 解析错误、-32600 无效请求、-32601 方法不存在、-32602 无效参数。
pub fn error_response(id: Value, code: i32, message: &str) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

/// 解析后的请求：notification 时 `id` 为 `None`（不回响应）。
pub struct Request {
    pub id: Option<Value>,
    pub method: String,
    pub params: Value,
}

/// 校验并拆一帧消息。解析错误返回 JSON-RPC 解析 / 无效请求错误响应（有 id 才回）。
pub fn parse_request(msg: &Value) -> Result<Request, Option<(Value, i32, String)>> {
    let method = msg.get("method").and_then(Value::as_str).ok_or(Some((
        msg.get("id").cloned().unwrap_or(Value::Null),
        -32600,
        "请求缺少 method".into(),
    )))?;
    let id = msg.get("id").cloned().filter(|v| !v.is_null());
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    Ok(Request {
        id,
        method: method.to_owned(),
        params,
    })
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, BufWriter, Cursor};

    use super::*;

    #[allow(clippy::panic)] // 测试断言
    #[test]
    fn frame_round_trips() {
        let msg = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
        let mut out = Vec::new();
        write_frame(&mut BufWriter::new(&mut out), &msg).unwrap();
        let mut reader = BufReader::new(Cursor::new(out));
        let back = read_frame(&mut reader).unwrap().unwrap();
        assert_eq!(back, msg);
    }

    #[allow(clippy::panic)] // 测试断言
    #[test]
    fn missing_content_length_is_an_error() {
        let input = "X-Header: 1\r\n\r\n{}";
        let mut reader = BufReader::new(Cursor::new(input.as_bytes()));
        assert!(read_frame(&mut reader).is_err());
    }

    #[allow(clippy::panic)] // 测试断言
    #[test]
    fn two_frames_in_one_stream_parse_separately() {
        let mut out = Vec::new();
        write_frame(
            &mut BufWriter::new(&mut out),
            &serde_json::json!({"method": "a"}),
        )
        .unwrap();
        write_frame(
            &mut BufWriter::new(&mut out),
            &serde_json::json!({"method": "b"}),
        )
        .unwrap();
        let mut reader = BufReader::new(Cursor::new(out));
        let first = read_frame(&mut reader).unwrap().unwrap();
        let second = read_frame(&mut reader).unwrap().unwrap();
        assert_eq!(first["method"], "a");
        assert_eq!(second["method"], "b");
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

    #[allow(clippy::panic)] // 测试断言
    #[test]
    fn parse_request_splits_id_and_params() {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": { "name": "lookup", "arguments": { "input": "ni" } }
        });
        let req = parse_request(&msg).unwrap();
        assert_eq!(req.id, Some(Value::from(7)));
        assert_eq!(req.method, "tools/call");
        assert_eq!(req.params["name"], "lookup");
    }

    #[allow(clippy::panic)] // 测试断言
    #[test]
    fn notification_has_no_id() {
        let msg = serde_json::json!({ "method": "notifications/initialized" });
        let req = parse_request(&msg).unwrap();
        assert!(req.id.is_none());
    }
}
