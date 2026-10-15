//! 对着真的 ibus-daemon 跑一遍：建输入上下文 → 切成青简 → 发按键 → 看信号。
//!
//! 这是唯一能验证「IBus 那套 GVariant 对象拼对了没有」的办法——签名错了不会报错，
//! 只会静默不显示，单元测试只能钉住签名字符串，钉不住 daemon 认不认。
//!
//! 这条路要三样东西都在：私有 ibus-daemon、装好组件 XML、**Server 起着**
//! （引擎进程只是前端，候选是 Server 算的）。怎么起见 `apps/linux/ibus/README.md`。
//!
//! **缺哪样都跳过**：CI 的 Linux runner 上没有 ibus-daemon，这个测试直接返回。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{MatchRule, MessageStream, connection::Builder, proxy};

/// 等一条信号最多多久。引擎进程是 daemon 现拉起来的，第一次要等它加载词库。
const SIGNAL_TIMEOUT: Duration = Duration::from_secs(20);

#[proxy(
    interface = "org.freedesktop.IBus",
    default_service = "org.freedesktop.IBus",
    default_path = "/org/freedesktop/IBus"
)]
trait IBus {
    fn create_input_context(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
    fn set_global_engine(&self, name: &str) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.IBus.InputContext",
    default_service = "org.freedesktop.IBus"
)]
trait InputContext {
    fn set_capabilities(&self, capabilities: u32) -> zbus::Result<()>;
    fn set_engine(&self, name: &str) -> zbus::Result<()>;
    fn focus_in(&self) -> zbus::Result<()>;
    fn process_key_event(&self, keyval: u32, keycode: u32, state: u32) -> zbus::Result<bool>;
}

/// IBus 总线地址；拿不到说明本机没跑 ibus，测试跳过。
fn bus_address() -> Option<String> {
    if let Some(address) = std::env::var_os("IBUS_ADDRESS")
        && !address.is_empty()
    {
        return address.into_string().ok();
    }
    let output = std::process::Command::new("ibus")
        .arg("address")
        .output()
        .ok()?;
    let address = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!address.is_empty() && address != "(null)").then_some(address)
}

/// 输入上下文要先声明支持哪些能力，不声明 IBus 连 `FocusIn` 都不让调。
/// 位：preedit(1) / 辅助文本(2) / 候选表(4) / 焦点(8)。
const CAPABILITIES: u32 = 1 | 2 | 4 | 8;

/// 主键盘区的硬件键码（evdev + 8）。
const KEY_N: u32 = 57;
const KEY_I: u32 = 31;
const KEY_H: u32 = 43;
const KEY_A: u32 = 38;
const KEY_O: u32 = 32;
const KEY_SPACE: u32 = 65;

#[tokio::test(flavor = "multi_thread")]
async fn typing_pinyin_reaches_the_panel_and_commits() {
    if std::env::var_os("QINGJIAN_IBUS_TEST").is_none() {
        eprintln!("跳过：没设 QINGJIAN_IBUS_TEST=1（它会动当前会话的输入法并写进真实学习数据）");
        return;
    }
    let Some(address) = bus_address() else {
        eprintln!("跳过：本机没有跑 ibus-daemon");
        return;
    };
    let connection = Builder::address(address.as_str())
        .expect("IBus 地址")
        .build()
        .await
        .expect("连上 IBus");

    let bus = IBusProxy::new(&connection).await.expect("IBus 门面");
    let context_path = bus
        .create_input_context("qingjian-integration-test")
        .await
        .expect("建输入上下文");

    // 先挂上信号流再切引擎，免得漏掉早到的信号
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(context_path.as_ref())
        .expect("按上下文路径过滤")
        .build();
    let signals = MessageStream::for_match_rule(rule, &connection, Some(4096))
        .await
        .expect("订阅信号");
    // 必须订阅后立刻开始收：zbus 的信号流是有界广播，连发几次按键期间不消费就会丢最旧的消息
    let collected = Collected::spawn(signals);

    let context = InputContextProxy::builder(&connection)
        .path(context_path.as_ref())
        .expect("上下文路径")
        .build()
        .await
        .expect("上下文代理");

    // 能力位要在引擎接上来之前声明：IBus 是在上下文与引擎挂钩那一刻决定 preedit 发给客户端还是发给面板的
    context
        .set_capabilities(CAPABILITIES)
        .await
        .expect("声明能力");

    // 开着 global-engine 的 daemon（GNOME 缺省就是）不让按上下文切，得走全局那条
    if let Err(per_context) = context.set_engine("qingjian").await
        && let Err(global) = bus.set_global_engine("qingjian").await
    {
        eprintln!("跳过：切不到 qingjian 引擎（按上下文：{per_context}；全局：{global}）");
        return;
    }
    context.focus_in().await.expect("拿焦点");

    // **等引擎真的挂上来再敲**：切引擎之后 ibus-daemon 还要走一遍
    // FocusOut → Enable → FocusIn 的激活流程（引擎进程的日志里看得到）。
    // 不等就会出现「第一个字母刚进缓冲就被 FocusOut 当原样上屏交出去」——
    // 真人用的时候引擎早就激活好了，只有测试会撞上这一段。
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 敲 nihao
    for (keyval, keycode) in [
        ('n', KEY_N),
        ('i', KEY_I),
        ('h', KEY_H),
        ('a', KEY_A),
        ('o', KEY_O),
    ] {
        let consumed = context
            .process_key_event(keyval as u32, keycode, 0)
            .await
            .expect("按键送达");
        assert!(consumed, "拼音字母该被吃掉：{keyval}");
    }

    // 2026-09-14 这里断言的是 `ShowPreeditText`，因为「带内容的 `UpdatePreeditText` 收不到」，
    // 当时归因成「真实应用的 preedit 由 GTK / Qt 的输入法模块自己渲染，不走客户端订阅这条路」。
    // **归因错了**：真正的原因是这条信号少发了一个参数（见 `ibus/engine.rs` 的 `update_preedit_text`），
    // ibus-daemon 解不出来就整条丢掉。补上之后带内容的 preedit 正常到达，
    // 反倒是 `ShowPreeditText` 不再单独来了——`UpdatePreeditText` 的 `visible=true` 已经把它显示出来，
    // daemon 那边判定「已经可见」就不再转发。
    collected
        .wait_for("UpdatePreeditText", "ni")
        .await
        .unwrap_or_else(|| panic!("preedit 里该有 ni；收到的是 {}", collected.summary()));
    collected
        .wait_for("UpdateLookupTable", "你好")
        .await
        .unwrap_or_else(|| panic!("候选表里该有 你好；收到的是 {}", collected.summary()));

    // 空格上屏
    let consumed = context
        .process_key_event(' ' as u32, KEY_SPACE, 0)
        .await
        .expect("空格送达");
    assert!(consumed, "组句中的空格该被吃掉");

    collected
        .wait_for("CommitText", "你好")
        .await
        .unwrap_or_else(|| panic!("上屏的该是 你好；收到的是 {}", collected.summary()));
}

/// 订阅后立刻起一条任务把信号全收下来，测试再从收到的里面找。
///
/// 不能边断言边拉流：zbus 的信号流是有界广播，连发几次按键期间没人消费就会丢最旧的消息，
/// 症状是「明明发了却收不到」，很容易误判成产品 bug（这个测试写歪过一次就是因为它）。
struct Collected {
    /// 收到的 (信号名, 载荷)，按到达顺序。
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

impl Collected {
    fn spawn(mut signals: MessageStream) -> Self {
        let seen: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        tokio::spawn(async move {
            while let Some(Ok(message)) = signals.next().await {
                let Some(member) = message.header().member().map(|m| m.to_string()) else {
                    continue;
                };
                sink.lock()
                    .expect("信号表")
                    .push((member, payload(&message)));
            }
        });
        Self { seen }
    }

    /// 等到出现一条名字对得上、载荷里含 `needle` 的信号。
    async fn wait_for(&self, name: &str, needle: &str) -> Option<String> {
        let deadline = tokio::time::Instant::now() + SIGNAL_TIMEOUT;
        loop {
            if let Some(payload) = self.find(name, needle) {
                return Some(payload);
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn find(&self, name: &str, needle: &str) -> Option<String> {
        self.seen
            .lock()
            .expect("信号表")
            .iter()
            .find(|(member, payload)| member == name && payload.contains(needle))
            .map(|(_, payload)| payload.clone())
    }

    /// 失败时打出来的现场：收到过哪些信号。
    /// 失败时打出来的东西。**带上载荷**：只列信号名的话，「发了但内容不对」与「压根没发」
    /// 看起来一模一样，排错要从头再来一遍。
    fn summary(&self) -> String {
        let seen = self.seen.lock().expect("信号表");
        let lines: Vec<String> = seen
            .iter()
            .map(|(member, payload)| {
                let payload: String = payload.chars().take(400).collect();
                format!("  {member}: {payload}")
            })
            .collect();
        format!("{} 条：\n{}", lines.len(), lines.join("\n"))
    }
}

/// 把信号载荷 debug 成一个字符串，只做包含判断，不解 IBus 的嵌套变体。
/// 三种信号的参数个数不一样，逐个试。
fn payload(message: &zbus::Message) -> String {
    let body = message.body();
    if let Ok((value, cursor, visible, mode)) = body.deserialize::<(Value<'_>, u32, bool, u32)>() {
        return format!("{value:?} cursor={cursor} visible={visible} mode={mode}");
    }
    if let Ok(value) = body.deserialize::<Value<'_>>() {
        return format!("{value:?}");
    }
    if let Ok((value, cursor, visible)) = body.deserialize::<(Value<'_>, u32, bool)>() {
        return format!("{value:?} cursor={cursor} visible={visible}");
    }
    if let Ok((value, visible)) = body.deserialize::<(Value<'_>, bool)>() {
        return format!("{value:?} visible={visible}");
    }
    String::new()
}
