//! 设置里影响候选的几项、设置页的选项清单、私密输入与中文标点键的出口。

use std::panic::catch_unwind;

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{JNI_FALSE, jboolean, jchar, jlong, jstring};

use super::{frame, with_session};
use crate::options::InputOptions;

/// `options` 是 [`InputOptions`] 的 JSON；解不开返回 null（设置保持原样）。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setOptions<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    options: JString<'local>,
) -> jstring {
    let Ok(json) = env.get_string(&options).map(String::from) else {
        return std::ptr::null_mut();
    };
    let options: InputOptions = match serde_json::from_str(&json) {
        Ok(options) => options,
        Err(error) => {
            tracing::warn!(%error, "键盘选项解不开");
            return std::ptr::null_mut();
        }
    };
    frame(env, handle, |session| session.set_options(&options))
}

/// 中文标点键：返回要上屏的文字；句柄无效时原样返回这个字符。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_punctuate<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    c: jchar,
) -> jstring {
    let c = char::from_u32(u32::from(c)).unwrap_or('?');
    let text =
        with_session(handle, |session| session.punctuate(c)).unwrap_or_else(|| c.to_string());
    env.new_string(text)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 私密输入开关：壳换输入框时按输入框交。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setPrivate<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    private: jboolean,
) {
    let _ = with_session(handle, |session| session.set_private(private != JNI_FALSE));
}

/// [`crate::SettingChoices`] 的 JSON：键盘设置页的选项清单与缺省值，不要句柄。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_settingChoices<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let Ok(choices) = catch_unwind(crate::setting_choices) else {
        return std::ptr::null_mut();
    };
    let json = serde_json::to_string(&choices).expect("选项清单总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
