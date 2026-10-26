//! 语音的出口：识别结果上屏（各种识别方式共用），识别配置与测试连接，以及 Rust 侧识别方的录音进出与取结果。
//! 配置是 [`VoiceSettings`] 的 JSON（含解好的密钥），只在内存里过一下，不进日志。

use std::panic::{AssertUnwindSafe, catch_unwind};

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JShortArray, JString};
use ::jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jlong, jstring};

use super::{frame, with_session};
use crate::VoiceSettings;
use crate::frame::VoiceEvent;

/// 识别出的一段文字上屏，回上屏之后的一帧。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_commitVoice<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    text: JString<'local>,
) -> jstring {
    let Ok(text) = env.get_string(&text).map(String::from) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.commit_voice(&text))
}

/// Rust 侧的识别方能不能用。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceAvailable<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jboolean {
    if with_session(handle, |session| session.voice_available()).unwrap_or(false) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceStart<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    sample_rate: jint,
) {
    let Ok(sample_rate) = u32::try_from(sample_rate) else {
        return;
    };
    let _ = with_session(handle, |session| session.voice_start(sample_rate));
}

/// 一截录音：单声道 16 位 PCM。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceFeed<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    samples: JShortArray<'local>,
) {
    let Ok(length) = env.get_array_length(&samples) else {
        return;
    };
    let mut buffer = vec![0i16; usize::try_from(length).unwrap_or(0)];
    if env
        .get_short_array_region(&samples, 0, &mut buffer)
        .is_err()
    {
        return;
    }
    let _ = with_session(handle, |session| session.voice_feed(&buffer));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceFinish<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    let _ = with_session(handle, |session| session.voice_finish());
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceCancel<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    let _ = with_session(handle, |session| session.voice_cancel());
}

/// 取一件识别的事（[`VoiceEvent`] 的 JSON），没有为 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_pollVoice<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    let Some(event) = with_session(handle, |session| session.poll_voice()).flatten() else {
        return std::ptr::null_mut();
    };
    let Ok(json) = serde_json::to_string(&VoiceEvent::from(event)) else {
        return std::ptr::null_mut();
    };
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

fn settings_of(env: &mut JNIEnv<'_>, settings: &JString<'_>) -> Option<VoiceSettings> {
    if settings.is_null() {
        return None;
    }
    let json = env.get_string(settings).map(String::from).ok()?;
    serde_json::from_str(&json).ok()
}

/// 按「语音识别」设置换识别方（`settings` 为 null 是没配）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setVoice<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    settings: JString<'local>,
) {
    let settings = settings_of(&mut env, &settings);
    let _ = with_session(handle, |session| session.set_voice(settings.as_ref()));
}

/// 「语音识别」页的测试连接：阻塞到有结果，Kotlin 在后台线程调。回 `{ok, message}` 的 JSON。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_voiceTest<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    settings: JString<'local>,
) -> jstring {
    super::logging::init();
    let settings = settings_of(&mut env, &settings);
    let outcome = catch_unwind(AssertUnwindSafe(|| match &settings {
        Some(settings) => crate::test_voice(settings),
        None => Err("配置读不出来".to_owned()),
    }))
    .unwrap_or_else(|_| Err("测试时出错了".to_owned()));
    let (ok, message) = match outcome {
        Ok(message) => (true, message),
        Err(message) => (false, message),
    };
    let json = serde_json::json!({ "ok": ok, "message": message }).to_string();
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
