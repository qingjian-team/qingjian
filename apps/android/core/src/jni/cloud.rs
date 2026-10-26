//! 快捷翻译、释义兜底、云联想、云端配置与工具框统计开关的出口。配置是 [`CloudSettings`] 的 JSON（含解好的密钥），只在内存里过一下，不进日志。

use std::panic::{AssertUnwindSafe, catch_unwind};

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jlong, jstring};
use serde::Serialize;

use super::with_session;
use crate::CloudSettings;

/// 「测试连接」的结果。
#[derive(Serialize)]
struct TestOutcome {
    ok: bool,

    /// 给用户看的一句话。
    message: String,
}

fn settings_of(env: &mut JNIEnv<'_>, settings: &JString<'_>) -> Option<CloudSettings> {
    if settings.is_null() {
        return None;
    }
    let json = env.get_string(settings).map(String::from).ok()?;
    serde_json::from_str(&json).ok()
}

fn to_jboolean(value: bool) -> jboolean {
    if value { JNI_TRUE } else { JNI_FALSE }
}

/// 接上（`settings` 非 null）或断开云端；返回接没接上。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setCloud<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    settings: JString<'local>,
) -> jboolean {
    let settings = settings_of(&mut env, &settings);
    let connected = with_session(handle, |session| session.set_cloud(settings.as_ref()));
    to_jboolean(connected.unwrap_or(false))
}

/// 开（`settings` 非 null）或关释义兜底；返回开没开上。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setGlossFill<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    settings: JString<'local>,
) -> jboolean {
    let settings = settings_of(&mut env, &settings);
    let enabled = with_session(handle, |session| session.set_gloss_fill(settings.as_ref()));
    to_jboolean(enabled.unwrap_or(false))
}

/// 发一条翻译请求；没接云端或文字为空返回 false。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_translate<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    text: JString<'local>,
) -> jboolean {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return JNI_FALSE;
    };
    to_jboolean(with_session(handle, |session| session.translate(&text)).unwrap_or(false))
}

/// 取译文：null 是还没到，空串是模型没给出内容。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_pollTranslation<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    match with_session(handle, |session| session.poll_translation()).flatten() {
        Some(text) => env
            .new_string(text)
            .map(JString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_cancelTranslation<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    let _ = with_session(handle, |session| session.cancel_translation());
}

/// 工具框（快捷翻译、常用语编辑）开着时关输入统计。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setMetered<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    metered: jboolean,
) {
    let _ = with_session(handle, |session| session.set_metered(metered != JNI_FALSE));
}

/// 测试连接：阻塞到有结果（请求有超时），Kotlin 在后台线程调。回 `{ok, message}` 的 JSON。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_cloudTest<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    settings: JString<'local>,
) -> jstring {
    super::logging::init();
    let settings = settings_of(&mut env, &settings);
    let outcome = catch_unwind(AssertUnwindSafe(|| match &settings {
        Some(settings) => crate::test_connection(settings),
        None => Err("配置读不出来".to_owned()),
    }))
    .unwrap_or_else(|_| Err("测试时出错了".to_owned()));
    let outcome = match outcome {
        Ok(message) => TestOutcome { ok: true, message },
        Err(message) => TestOutcome { ok: false, message },
    };
    let json = serde_json::to_string(&outcome).expect("两个字段总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 这段组句的光标前后文（云联想用），壳在每段组句开头交一次。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setSurrounding<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    before: JString<'local>,
    after: JString<'local>,
) {
    let (Ok(before), Ok(after)) = (
        env.get_string(&before).map(String::from),
        env.get_string(&after).map(String::from),
    ) else {
        return;
    };
    with_session(handle, |session| session.set_surrounding(before, after));
}

/// 取回云联想结果：插进候选后的一帧 JSON；没有新结果为 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_pollPrediction<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: 同 `super::frame`：句柄来自 `create`，Kotlin 侧只在主线程用、销毁后置 0。
    let handle = unsafe { &mut *(handle as *mut crate::handle::Handle) };
    let polled = catch_unwind(AssertUnwindSafe(|| {
        handle.poll_prediction().map(|frame| frame.to_json())
    }));
    match polled {
        Ok(Some(json)) => env
            .new_string(json)
            .map(JString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        Ok(None) => std::ptr::null_mut(),
        Err(_) => {
            tracing::error!("取云联想结果时 panic");
            std::ptr::null_mut()
        }
    }
}
