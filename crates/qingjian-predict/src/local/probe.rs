//! 看端口上有没有服务、是不是这个模型：直连 127.0.0.1 发一条最简的 HTTP/1.1 `GET /props`，不经代理。
//! llama-server 加载模型时一律回 503；加载好后 `/props` 回模型名与模型路径（这个接口它不记日志）。

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// 连接的时限。Windows 上连没人听的本机端口要两秒左右才回拒绝，连不上一律当端口空着。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// 连上以后读写的时限。
const IO_TIMEOUT: Duration = Duration::from_secs(2);

/// 回复最多收这么多（`/props` 带着对话模板，十几 KB）。
const MAX_RESPONSE: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Probe {
    /// 没有程序在听。
    Free,

    /// llama-server 正在加载模型。
    Loading,

    /// 加载好了的 llama-server：模型名（`--alias`）与模型路径（`-m` 原样）。
    Serving { alias: String, model: String },

    /// 有程序在听却没答上话（超时、连上就断、回的不是 HTTP）：多半是正在起或正在退出的 llama-server。
    Silent,

    /// 别的程序，或要密钥的 llama-server。
    Other,
}

pub(crate) fn probe(port: u16) -> Probe {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(stream) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) else {
        return Probe::Free;
    };
    match get(stream, port, "/props") {
        Ok((503, _)) => Probe::Loading,
        Ok((200, body)) => serving(&body).unwrap_or(Probe::Other),
        Ok(_) => Probe::Other,
        Err(_) => Probe::Silent,
    }
}

/// 发请求、收回复，返回状态码与正文。
fn get(mut stream: TcpStream, port: u16, path: &str) -> io::Result<(u16, Vec<u8>)> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    )?;
    let mut raw = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&buffer[..read]);
        // 按 Content-Length 收全了就不等对方关连接
        if let Some(response) = parse_response(&raw, false) {
            return Ok(response);
        }
        if raw.len() > MAX_RESPONSE {
            return Err(io::Error::other("response too large"));
        }
    }
    parse_response(&raw, true).ok_or_else(|| io::Error::other("malformed response"))
}

/// 拆开 HTTP 回复：状态码与正文。正文按 Content-Length 截，分块传输的拼起来；
/// 两样都没有时正文到连接关闭为止（`closed`）。还没收全返回 `None`。
pub(crate) fn parse_response(raw: &[u8], closed: bool) -> Option<(u16, Vec<u8>)> {
    let split = find(raw, b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..split]).ok()?;
    let body = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split_whitespace().nth(1)?.parse().ok()?;
    let mut length = None;
    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse::<usize>().ok();
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            chunked = value.to_ascii_lowercase().contains("chunked");
        }
    }
    let body = if chunked {
        dechunk(body)?
    } else if let Some(length) = length {
        body.get(..length)?.to_vec()
    } else if closed {
        body.to_vec()
    } else {
        return None;
    };
    Some((status, body))
}

/// 拼起分块传输的正文；没收到结尾的零长度块返回 `None`。
fn dechunk(mut data: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let end = find(data, b"\r\n")?;
        let size = std::str::from_utf8(&data[..end]).ok()?;
        let size = usize::from_str_radix(size.split(';').next()?.trim(), 16).ok()?;
        data = &data[end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(data.get(..size)?);
        data = data.get(size + 2..)?;
    }
}

/// `/props` 的正文里取模型名与模型路径；不是 JSON 或缺这两项返回 `None`。
pub(crate) fn serving(body: &[u8]) -> Option<Probe> {
    let props: serde_json::Value = serde_json::from_slice(body).ok()?;
    let field = |name: &str| props.get(name)?.as_str().map(str::to_owned);
    Some(Probe::Serving {
        alias: field("model_alias")?,
        model: field("model_path")?,
    })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
