//! 组句期间的轮询：Linux Server 不推送，本地整句模型的重排结果要前端自己来取。
//!
//! 与 Fcitx5 插件同一个做法（`apps/linux/fcitx5/src/qingjian.cpp` 的 `poll`）：还在组句就每 80 毫秒
//! 发一次 `Poll`，回来的帧版本号比面板上那一版新才重画；不组句了就停。
//!
//! **按键与轮询共用一把锁**，锁住「与 Server 往返 + 比版本号 + 画」整段：不然一次轮询取到的旧帧
//! 可能在按键的新帧之后才画出来，把新帧盖掉。

use std::sync::Arc;
use std::time::Duration;

use qingjian_platform::protocol::Frame;
use tokio::sync::{Mutex, MutexGuard};
use zbus::object_server::SignalEmitter;

use super::engine::present_frame;
use crate::client::Shared;

/// 轮询间隔，与 Fcitx5 插件一致；Server 那边停键 80 毫秒才把整句送去重排。
const INTERVAL: Duration = Duration::from_millis(80);

/// 一个引擎对象的轮询；克隆出来的共用同一份状态。
#[derive(Clone, Default)]
pub struct Poller {
    state: Arc<Mutex<PollState>>,
}

/// 面板上是哪一版帧，以及轮询任务的开停。
#[derive(Default)]
pub struct PollState {
    /// 面板上现在是哪一版帧（Server 的展示版本号）。
    shown: u64,

    /// 轮询任务的代次：每次重新起轮询、或者停掉，都加一；旧任务醒来看到代次变了就退出。
    epoch: u64,

    /// 有没有轮询任务在跑。
    running: bool,
}

impl Poller {
    /// 拿锁。按键等事件在锁里做完往返与重画，再交给 [`Self::follow`]。
    pub async fn lock(&self) -> MutexGuard<'_, PollState> {
        self.state.lock().await
    }

    /// 一次事件的帧画完之后：记下它的版本号，还在组句就确保轮询在跑，不组句了就停。
    pub fn follow(
        &self,
        state: &mut PollState,
        frame: &Frame,
        revision: Option<u64>,
        emitter: &SignalEmitter<'_>,
        client: &Shared,
        context: &str,
    ) {
        if let Some(revision) = revision {
            state.shown = revision;
        }
        if !composing(frame) {
            self.stop(state);
            return;
        }
        if state.running {
            return;
        }
        state.running = true;
        state.epoch += 1;
        tokio::spawn(run(
            Arc::clone(&self.state),
            state.epoch,
            emitter.to_owned(),
            client.clone(),
            context.to_owned(),
        ));
    }

    /// 停掉轮询（不组句了、引擎对象要销毁了）。在跑的任务下一拍醒来看到代次变了就退出。
    pub fn stop(&self, state: &mut PollState) {
        if state.running {
            state.running = false;
            state.epoch += 1;
        }
    }
}

/// 轮询任务：隔 [`INTERVAL`] 问一次，版本号变新了就重画，不组句了或者被新的代次取代就退出。
async fn run(
    state: Arc<Mutex<PollState>>,
    epoch: u64,
    emitter: SignalEmitter<'static>,
    client: Shared,
    context: String,
) {
    loop {
        tokio::time::sleep(INTERVAL).await;
        let mut state = state.lock().await;
        if state.epoch != epoch {
            return;
        }
        let reply =
            match client.with_session(&context, |connection, session| connection.poll(session)) {
                Ok(reply) => reply,
                Err(error) => {
                    // 连接断了由下一次按键重连；轮询不在这里重试，免得每 80 毫秒刷一条失败
                    tracing::debug!(%error, "轮询失败，这一轮组句不再取重排结果");
                    state.running = false;
                    state.epoch += 1;
                    return;
                }
            };
        tracing::trace!(revision = ?reply.revision, shown = state.shown, "轮询");
        let Some(revision) = reply.revision.filter(|revision| *revision > state.shown) else {
            continue;
        };
        state.shown = revision;
        if let Err(error) = present_frame(&emitter, &reply.frame).await {
            tracing::warn!(%error, "重排结果到了，但更新候选窗失败");
        }
        tracing::debug!(revision, "重排结果到了，重画");
        if !composing(&reply.frame) {
            state.running = false;
            state.epoch += 1;
            return;
        }
    }
}

/// 帧里还有组句：拼音行或候选有一样不空。
fn composing(frame: &Frame) -> bool {
    !frame.preedit.is_empty() || !frame.candidates.items.is_empty()
}
