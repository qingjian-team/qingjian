use std::time::Duration;

use super::{Index, signature};
use crate::UpdateError;

/// 官网上的版本索引与它的分离签名；`QINGJIAN_UPDATE_INDEX` 可以换成别的地址（测试用，签名照验）。
const INDEX_URL: &str = "https://qingjian.app/releases.json";

const MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;

const TIMEOUT: Duration = Duration::from_secs(20);

/// 下载索引与签名，验过签名再解析。请求只带 `青简/<版本>` 的 User-Agent，没有任何标识。
pub(crate) fn fetch_index(current_version: &str) -> Result<Index, UpdateError> {
    let url = std::env::var("QINGJIAN_UPDATE_INDEX").unwrap_or_else(|_| INDEX_URL.to_owned());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .user_agent(format!("qingjian/{current_version}"))
            .build()?;
        let index = download(&client, &url).await?;
        let signature_bytes = download(&client, &format!("{url}.sig")).await?;
        let signature_text =
            String::from_utf8(signature_bytes).map_err(|_| UpdateError::MalformedSignature)?;
        signature::verify(&index, &signature_text)?;
        Index::parse(&index)
    })
}

async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, UpdateError> {
    let response = client.get(url).send().await?.error_for_status()?;
    // 流式读：服务端给多大就收多大，超限立刻断，别先把整个响应读进内存再检查。
    let mut stream = response.bytes_stream();
    let mut out = Vec::new();
    while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = chunk?;
        if out.len().saturating_add(chunk.len()) > MAX_INDEX_BYTES {
            return Err(UpdateError::TooLarge(MAX_INDEX_BYTES));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}
