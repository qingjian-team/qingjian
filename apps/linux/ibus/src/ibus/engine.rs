//! `org.freedesktop.IBus.Engine`：ibus-daemon 每开一个输入上下文就叫工厂造一个这个对象。
//!
//! 这一层只做两件事：把 D-Bus 来的事件转成协议事件发给 Server，把 Server 回的帧经 [`View`]
//! 折成 IBus 面板认的三样（preedit / 候选表 / 辅助行）再发信号出去。
//! **不做任何排序、查词或文本变换**——那些在 Server 进程的 Engine 里。
//!
//! 与 Server 的收发是阻塞的（一问一答，200 毫秒截止），就在 async 处理函数里直接调：
//! 输入法本来就按用户的按键串行，Fcitx5 插件也是在它的事件循环里同步收发。

use qingjian_platform::protocol::linux::Capabilities;
use qingjian_platform::protocol::{KeyOutcome, SessionId};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

use super::poller::Poller;
use super::variant;
use super::view::View;
use crate::client::{Connection, Reply, Shared};
use crate::error::IbusError;
use crate::key;

/// `UpdatePreeditText` 的 `mode`：焦点移走时把 preedit 清掉（ibus 的 `IBUS_ENGINE_PREEDIT_CLEAR`）。
/// 青简在 `FocusOut` 里自己把缓冲原样上屏，不靠 ibus 替它提交，所以是 CLEAR 不是 COMMIT。
const PREEDIT_CLEAR: u32 = 0;

/// `SetContentType` 的 purpose：密码框与 PIN（`IBusInputPurpose`）。
const PURPOSE_PASSWORD: u32 = 8;
const PURPOSE_PIN: u32 = 9;

/// `SetContentType` 的 hints：应用声明这是私密输入（`IBUS_INPUT_HINT_PRIVATE`）。
const HINT_PRIVATE: u32 = 1 << 10;

/// 把一帧发给面板：preedit、候选表、辅助行各发一次，空的就发隐藏。
pub(crate) async fn present_frame(
    emitter: &SignalEmitter<'_>,
    frame: &qingjian_platform::protocol::Frame,
) -> zbus::Result<()> {
    let view = View::from_frame(frame);

    if view.preedit.is_empty() {
        IBusEngine::hide_preedit_text(emitter).await?;
        IBusEngine::update_preedit_text(emitter, variant::text(""), 0, false, PREEDIT_CLEAR)
            .await?;
    } else {
        let text = variant::segmented_text(&view.preedit);
        IBusEngine::update_preedit_text(emitter, text, view.cursor, true, PREEDIT_CLEAR).await?;
        IBusEngine::show_preedit_text(emitter).await?;
    }

    if view.has_candidates() {
        let table = variant::lookup_table(&view.candidates, view.cursor_index, true);
        IBusEngine::update_lookup_table(emitter, table, true).await?;
        IBusEngine::show_lookup_table(emitter).await?;
    } else {
        IBusEngine::update_lookup_table(emitter, variant::lookup_table(&[], 0, false), false)
            .await?;
        IBusEngine::hide_lookup_table(emitter).await?;
    }

    match &view.auxiliary {
        Some(line) => {
            IBusEngine::update_auxiliary_text(emitter, variant::text(line), true).await?;
            IBusEngine::show_auxiliary_text(emitter).await?;
        }
        None => {
            IBusEngine::update_auxiliary_text(emitter, variant::text(""), false).await?;
            IBusEngine::hide_auxiliary_text(emitter).await?;
        }
    }
    Ok(())
}

/// 一个输入上下文的引擎对象。
pub struct IBusEngine {
    /// 与 Server 的共用连接；会话按 [`Self::context`] 认。
    client: Shared,

    /// 自己的对象路径，同时当 Server 那边的上下文标识（要求非空、不超过 64 字节、同连接内不重复）。
    context: String,

    /// 组句期间取重排结果的轮询；它的锁也把每次事件的「往返 + 重画」串起来。
    poller: Poller,
}

impl IBusEngine {
    pub fn new(client: Shared, context: String) -> Self {
        Self {
            client,
            context,
            poller: Poller::default(),
        }
    }

    /// 发一个事件给 Server，把回来的处置应用到面板上（上屏 + 重画），返回这次按键吃没吃掉。
    /// 画完还在组句就起轮询，等本地整句模型的重排结果（见 [`Poller`]）。
    ///
    /// Server 连不上时**一律放行**：输入法连不上引擎是坏了，但不能连累用户连字母都打不出来。
    async fn apply(
        &self,
        emitter: &SignalEmitter<'_>,
        what: &str,
        action: impl FnOnce(&mut Connection, SessionId) -> Result<Reply, IbusError>,
    ) -> bool {
        let mut polling = self.poller.lock().await;
        let reply = match self.client.with_session(&self.context, action) {
            Ok(reply) => reply,
            Err(error) => {
                tracing::warn!(%error, what, "与 Server 的一次往返失败，这个事件放行");
                return false;
            }
        };
        if let Some(text) = reply.commit.filter(|text| !text.is_empty())
            && let Err(error) = Self::commit_text(emitter, variant::text(&text)).await
        {
            tracing::warn!(%error, "上屏失败");
        }
        if let Err(error) = present_frame(emitter, &reply.frame).await {
            tracing::warn!(%error, "更新候选窗失败");
        }
        self.poller.follow(
            &mut polling,
            &reply.frame,
            reply.revision,
            emitter,
            &self.client,
            &self.context,
        );
        reply.outcome == KeyOutcome::Consumed
    }
}

#[zbus::interface(name = "org.freedesktop.IBus.Engine")]
impl IBusEngine {
    /// 一次按键。回 `true` 表示吃掉，`false` 让应用自己处理。
    ///
    /// 抬键也要报上去：中 / 英切换认的是 Shift 的**抬起**（`[shortcut] switch_mode`），
    /// 在这里就地放行的话那个键永远不会生效。吃不吃掉仍由 Server 说了算。
    async fn process_key_event(
        &mut self,
        keyval: u32,
        _keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let release = key::is_release(state);
        let event = key::to_key_event(keyval, state);
        let consumed = self
            .apply(&emitter, "按键", |connection, session| {
                connection.key(session, event, release)
            })
            .await;
        // 排错时要看的是**事件之间的交错**（焦点、能力、按键谁先谁后），少了这一条就只能靠猜
        tracing::debug!(keyval, state, release, consumed, "按键");
        consumed
    }

    /// 拿到焦点。
    async fn focus_in(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        tracing::debug!(context = %self.context, "拿到焦点");
        self.apply(&emitter, "焦点进入", |connection, session| {
            connection.focus(session, true)
        })
        .await;
    }

    /// 焦点离开：缓冲原样上屏，与 macOS 的 `commitComposition`、Windows 的 `Commit` 一致。
    ///
    /// `client_preedit` 报 `false`：报 `true` 的话 Server 会把缓冲丢掉不上屏（那是给「应用自己画
    /// preedit」的场合留的），而走 IBus 这条路时缓冲里的字母该原样交给应用——
    /// 这是 IBus 那版真机验过的行为，换成 `true` 之前要在真机上确认。
    async fn focus_out(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        tracing::debug!(context = %self.context, "焦点离开，结束组句");
        self.apply(&emitter, "焦点离开", |connection, session| {
            connection.deactivate(session, true, false, false)
        })
        .await;
    }

    /// IBus 1.5 之后带对象路径与客户端名的那一对，行为同上。
    async fn focus_in2(
        &mut self,
        _object_path: String,
        client: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        tracing::debug!(%client, "拿到焦点");
        self.focus_in(emitter).await;
    }

    async fn focus_out2(
        &mut self,
        _object_path: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.focus_out(emitter).await;
    }

    /// 应用要求重置：丢掉组句不上屏。
    async fn reset(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.apply(&emitter, "重置", |connection, session| {
            connection.reset(session)
        })
        .await;
    }

    /// 应用说明这个输入框是干什么的（IBus 的 `IBusInputPurpose` / `IBusInputHints`）。
    ///
    /// **这是 IBus 这条路上唯一能知道「这是密码框」的地方**（fcitx5 那边是框架的能力位）。
    /// Server 在收到能力之前不接受按键，所以会话一开就报一次缺省值，这里只报变化。
    async fn set_content_type(&mut self, purpose: u32, hints: u32) {
        let capabilities = Capabilities {
            sensitive: hints & HINT_PRIVATE != 0,
            password: purpose == PURPOSE_PASSWORD || purpose == PURPOSE_PIN,
            disabled: false,
        };
        match self.client.set_capabilities(&self.context, capabilities) {
            Ok(true) => tracing::debug!(?capabilities, "输入框类型变了，已报给 Server"),
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, "报输入框类型失败"),
        }
    }

    /// 输入法被启用 / 停用。
    async fn enable(&mut self) {
        tracing::debug!("启用");
    }

    async fn disable(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.apply(&emitter, "停用", |connection, session| {
            connection.reset(session)
        })
        .await;
        tracing::debug!("停用");
    }

    /// 下面这些 IBus 会调但我们不处理：翻页由 Core 的候选布局算（面板自己翻会和它打架），
    /// 光标位置用不着（面板自己定位），属性栏还没做。声明出来是因为
    /// 不实现的话 ibus-daemon 每次调用都会记一条错误。
    async fn set_capabilities(&mut self, _capabilities: u32) {}

    async fn set_cursor_location(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    async fn page_up(&mut self) {}

    async fn page_down(&mut self) {}

    async fn cursor_up(&mut self) {}

    async fn cursor_down(&mut self) {}

    async fn candidate_clicked(&mut self, _index: u32, _button: u32, _state: u32) {}

    async fn property_activate(&mut self, _name: String, _state: u32) {}

    async fn destroy(&mut self) {
        // 先停轮询：不然它下一拍会给已经销毁的上下文重开一个会话
        let mut polling = self.poller.lock().await;
        self.poller.stop(&mut polling);
        self.client.close(&self.context);
        tracing::debug!(context = %self.context, "引擎对象销毁");
    }

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    /// 参数是 `(vubu)`，**最后那个 `mode` 不能省**：ibus-daemon 按 `(vubu)` 解这条信号，
    /// 少一个参数它解不出来，只在自己的日志里留一条 `arg0 != NULL` 的 CRITICAL，
    /// 然后把整条信号丢掉——症状就是拼音行不显示，而且这边一点错都看不到。
    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
