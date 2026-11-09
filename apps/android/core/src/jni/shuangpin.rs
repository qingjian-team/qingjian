//! 双拼方案列表与键帽的出口：不要句柄，壳在设置页与建键盘时取。

use std::panic::catch_unwind;

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::jstring;

/// [`crate::ShuangpinChoice`] 列表的 JSON。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_shuangpinChoices<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jstring {
    let Ok(choices) = catch_unwind(crate::shuangpin_choices) else {
        return std::ptr::null_mut();
    };
    let json = serde_json::to_string(&choices).expect("方案列表总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// `scheme` 的 [`crate::ShuangpinKeycap`] 列表 JSON；认不出的方案是空数组。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_shuangpinKeycaps<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    scheme: JString<'local>,
) -> jstring {
    let Ok(scheme) = env.get_string(&scheme).map(String::from) else {
        return std::ptr::null_mut();
    };
    let Ok(keycaps) = catch_unwind(|| crate::shuangpin_keycaps(&scheme)) else {
        return std::ptr::null_mut();
    };
    let json = serde_json::to_string(&keycaps).expect("键帽总能序列化");
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
