//! 本机模型：`[predict] local_model` 填了 GGUF 文件时，用 llama.cpp 的 llama-server 在本机运行它，
//! 联想、问字、翻译与释义都发给它（只监听 127.0.0.1，不要密钥，不走代理）。
//!
//! 客户端建起来时租用服务（[`LocalServer::lease`]）：同一进程里参数相同的客户端共用一个，最后一个放手时停掉。
//! 后台线程（`supervisor`）负责起服务、等它加载好、退出后隔一阵重起；端口上已有同一模型的服务
//! （比如设置程序测试连接时输入法已经起好了）就直接用，不起第二个。青简被强行结束时由看门进程（`watchdog`）
//! 收掉 llama-server，不留下占着内存的进程。

mod error;
mod locate;
mod probe;
mod server;
mod spec;
mod status;
mod supervisor;
mod tail;
mod watchdog;

#[cfg(test)]
mod tests;

pub use error::LocalServerError;
pub(crate) use server::LocalServer;
pub(crate) use spec::LaunchSpec;

/// Windows 起控制台程序时不开窗口（`CREATE_NO_WINDOW`）。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
