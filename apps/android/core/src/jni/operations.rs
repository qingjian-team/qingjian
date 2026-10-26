//! 返回一帧的出口：每个对应 [`Session`] 的一个操作。

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{jboolean, jint, jlong, jstring};

use super::frame;
use crate::session::Session;

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_push<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    code_point: jint,
) -> jstring {
    let Some(c) = u32::try_from(code_point).ok().and_then(char::from_u32) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.push(c))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_backspace<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    frame(env, handle, Session::backspace)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_select<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    index: jint,
) -> jstring {
    let Ok(index) = usize::try_from(index) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.select(index))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_selectTranslation<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    index: jint,
    sense: jint,
) -> jstring {
    let (Ok(index), Ok(sense)) = (usize::try_from(index), usize::try_from(sense)) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| {
        session.select_translation(index, sense)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_forget<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    index: jint,
) -> jstring {
    let Ok(index) = usize::try_from(index) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.forget(index))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_commitRaw<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    frame(env, handle, Session::commit_raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clear<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    frame(env, handle, Session::clear)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setNineKey<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_nine_key(on != 0))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_pin<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    alternative: JString<'local>,
) -> jstring {
    let Ok(alternative) = env.get_string(&alternative).map(String::from) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.pin(&alternative))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setEnglishMode<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_english_mode(on != 0))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_complete<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    before_cursor: JString<'local>,
) -> jstring {
    let Ok(before_cursor) = env.get_string(&before_cursor).map(String::from) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| session.complete(&before_cursor))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setZhuyin<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_zhuyin(on != 0))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_space<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jstring {
    frame(env, handle, Session::space)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setJapanese<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_japanese(on != 0))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setTraditional<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_traditional(on != 0))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setShuangpin<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    scheme: JString<'local>,
    raw_preedit: jboolean,
) -> jstring {
    let Ok(scheme) = env.get_string(&scheme).map(String::from) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| {
        session.set_shuangpin(&scheme, raw_preedit != 0)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setWubi<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    on: jboolean,
    mixed: jboolean,
) -> jstring {
    frame(env, handle, |session| session.set_wubi(on != 0, mixed != 0))
}
