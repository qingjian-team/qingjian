//! 剪贴板同步的出口。服务进程里只有一份；事件由 Kotlin 的后台线程用 `clipboardSyncPoll` 取，取到的是 JSON。

use std::panic::catch_unwind;
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jstring};
use qingjian_clipboard_sync::default_name::default_device_name;
use qingjian_clipboard_sync::discovery::Found;
use qingjian_clipboard_sync::{
    Identity, PeerStatus, RecordKind, Replica, SyncEvent, SyncOptions, SyncService,
};
use serde_json::json;

use crate::HistoryReplica;

struct Running {
    service: SyncService,

    /// 单独一份，事件和历史都从这里读：同步服务里的那份已交给服务。
    reader: HistoryReplica,

    events: Arc<Mutex<Receiver<SyncEvent>>>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

fn running() -> MutexGuard<'static, Option<Running>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn to_java(env: &JNIEnv, text: &str) -> jstring {
    env.new_string(text)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn identity_path(user_dir: &str) -> std::path::PathBuf {
    Path::new(user_dir)
        .join("clipboard-sync")
        .join("identity.key")
}

/// 没起名时显示的默认名。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncDefaultName<
    'local,
>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    chinese: jboolean,
) -> jstring {
    let Ok(user_dir) = env.get_string(&user_dir).map(String::from) else {
        return std::ptr::null_mut();
    };
    match Identity::load_or_create(&identity_path(&user_dir)) {
        Ok(identity) => to_java(
            &env,
            &default_device_name(&identity.public, chinese != JNI_FALSE),
        ),
        Err(error) => {
            tracing::warn!(%error, "clipboard sync identity failed");
            std::ptr::null_mut()
        }
    }
}

/// 开始同步（已经开着就先停掉重开），成功回真。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncStart<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    name: JString<'local>,
    model: JString<'local>,
    limit: jint,
) -> jboolean {
    let (Ok(user_dir), Ok(name), Ok(model)) = (
        env.get_string(&user_dir).map(String::from),
        env.get_string(&name).map(String::from),
        env.get_string(&model).map(String::from),
    ) else {
        return JNI_FALSE;
    };
    let started = catch_unwind(|| {
        *running() = None;
        let directory = Path::new(&user_dir);
        let limit = usize::try_from(limit).unwrap_or(1).max(1);
        let (sender, events) = channel();
        let service = SyncService::start(
            SyncOptions {
                directory: directory.join("clipboard-sync"),
                name,
                model,
                own_discovery: false,
            },
            Box::new(HistoryReplica::new(directory, limit)),
            sender,
        )?;
        *running() = Some(Running {
            service,
            reader: HistoryReplica::new(directory, limit),
            events: Arc::new(Mutex::new(events)),
        });
        Ok::<(), qingjian_clipboard_sync::SyncError>(())
    });
    match started {
        Ok(Ok(())) => JNI_TRUE,
        Ok(Err(error)) => {
            tracing::warn!(%error, "clipboard sync did not start");
            JNI_FALSE
        }
        Err(_) => JNI_FALSE,
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncStop<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
) {
    let _ = catch_unwind(|| *running() = None);
}

/// 壳注册 NSD 要用的：设备 id 和监听端口（JSON）。没在同步时回 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncInfo<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let info = catch_unwind(|| {
        let guard = running();
        let service = &guard.as_ref()?.service;
        Some(json!({"id": service.device_id(), "port": service.port()}).to_string())
    });
    match info {
        Ok(Some(json)) => to_java(&env, &json),
        _ => std::ptr::null_mut(),
    }
}

/// NSD 发现了一台设备。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncFound<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    device_id: JString<'local>,
    name: JString<'local>,
    model: JString<'local>,
    address: JString<'local>,
) {
    let (Ok(device_id), Ok(name), Ok(model), Ok(address)) = (
        env.get_string(&device_id).map(String::from),
        env.get_string(&name).map(String::from),
        env.get_string(&model).map(String::from),
        env.get_string(&address).map(String::from),
    ) else {
        return;
    };
    let Ok(address) = address.parse() else {
        return;
    };
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref() {
            running.service.report(Found::Seen {
                device_id,
                name,
                model,
                address,
            });
        }
    });
}

/// NSD 说一台设备下线了。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncGone<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    device_id: JString<'local>,
) {
    let Ok(device_id) = env.get_string(&device_id).map(String::from) else {
        return;
    };
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref() {
            running.service.report(Found::Gone { device_id });
        }
    });
}

/// 本机内容变了。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncChanged<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
) {
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref() {
            running.service.notify_changed();
        }
    });
}

fn event_json(event: SyncEvent, reader: Option<&HistoryReplica>) -> serde_json::Value {
    match event {
        SyncEvent::NearbyChanged => json!({"type": "nearby"}),
        SyncEvent::Paired { name } => json!({"type": "paired", "name": name}),
        SyncEvent::PairFailed => json!({"type": "pair_failed"}),
        SyncEvent::ReceiveStarted { .. } => json!({"type": "started"}),
        SyncEvent::ReceiveProgress {
            bytes_per_second, ..
        } => json!({"type": "progress", "speed": bytes_per_second}),
        SyncEvent::ReceiveFinished { newest, .. } => {
            // 文字直接带给壳放上系统剪贴板
            let text = newest
                .filter(|item| item.kind == RecordKind::Text)
                .and_then(|item| reader?.record(&item.id).ok()?)
                .map(|record| record.text);
            json!({"type": "finished", "text": text})
        }
        SyncEvent::ReceiveFailed { .. } | SyncEvent::SendFailed { .. } => {
            json!({"type": "failed"})
        }
    }
}

/// 等下一个事件，超时或没在同步时回 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncPoll<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    timeout_millis: jint,
) -> jstring {
    let wait = Duration::from_millis(u64::try_from(timeout_millis).unwrap_or(0));
    let polled = catch_unwind(|| {
        // 等的时候不占着总锁，别的出口才进得来
        let events = running().as_ref().map(|running| running.events.clone())?;
        let event = events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv_timeout(wait);
        match event {
            Ok(event) => {
                let guard = running();
                let reader = guard.as_ref().map(|running| &running.reader);
                Some(event_json(event, reader).to_string())
            }
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    });
    match polled {
        Ok(Some(json)) => to_java(&env, &json),
        _ => std::ptr::null_mut(),
    }
}

/// 设备页要显示的：已配对和附近的设备。没在同步时回 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncDevices<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let devices = catch_unwind(|| {
        let guard = running();
        let service = &guard.as_ref()?.service;
        let nearby = service.nearby();
        let peers: Vec<_> = service
            .peer_states()
            .into_iter()
            .map(|state| {
                let status = match state.status {
                    PeerStatus::Connected => "connected",
                    PeerStatus::Connecting => "connecting",
                    PeerStatus::Unreachable => "unreachable",
                };
                json!({
                    "id": state.peer.device_id,
                    "name": state.peer.name,
                    "status": status,
                    "last_seen": state.peer.last_seen,
                })
            })
            .collect();
        let nearby: Vec<_> = nearby
            .iter()
            .map(|device| {
                json!({
                    "id": device.device_id,
                    "name": device.name,
                    "model": device.model,
                    "address": device.address.to_string(),
                })
            })
            .collect();
        Some(json!({"peers": peers, "nearby": nearby}).to_string())
    });
    match devices {
        Ok(Some(json)) => to_java(&env, &json),
        _ => std::ptr::null_mut(),
    }
}

/// 设备页开了或关了：开着时状态刷新勤一些，并扫一遍同网段。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncWatch<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    watching: jboolean,
) {
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref() {
            running.service.set_watching(watching != JNI_FALSE);
        }
    });
}

/// 这台显示配对码。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncBeginPairing<
    'local,
>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let code = catch_unwind(|| running().as_ref()?.service.begin_pairing().ok());
    match code {
        Ok(Some(code)) => to_java(&env, &code),
        _ => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncCancelPairing<
    'local,
>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
) {
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref() {
            running.service.cancel_pairing();
        }
    });
}

/// 这台输入配对码，结果走事件。地址写错回假。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncJoin<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    address: JString<'local>,
    code: JString<'local>,
) -> jboolean {
    let (Ok(address), Ok(code)) = (
        env.get_string(&address).map(String::from),
        env.get_string(&code).map(String::from),
    ) else {
        return JNI_FALSE;
    };
    let Ok(address) = address.trim().parse() else {
        return JNI_FALSE;
    };
    let joined = catch_unwind(|| {
        let guard = running();
        let running = guard.as_ref()?;
        running.service.join_pairing(address, code);
        Some(())
    });
    if matches!(joined, Ok(Some(()))) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clipboardSyncUnpair<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    device_id: JString<'local>,
) {
    let Ok(device_id) = env.get_string(&device_id).map(String::from) else {
        return;
    };
    let _ = catch_unwind(|| {
        if let Some(running) = running().as_ref()
            && let Err(error) = running.service.unpair(&device_id)
        {
            tracing::warn!(%error, "cannot unpair");
        }
    });
}
