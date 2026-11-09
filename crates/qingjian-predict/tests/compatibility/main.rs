//! 用本地 HTTP 服务验证按接口报错降级，不读取密钥或调用外部服务。

mod cache;
mod failures;
mod helpers;
mod parameters;
