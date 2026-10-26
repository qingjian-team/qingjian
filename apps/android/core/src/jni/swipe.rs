//! 滑行输入的出口：壳交 `{points: [{x, y}], keys: [{letter, x, y, width, height}]}` 的 JSON（像素，键位是各字母键的中心与大小）。

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{jlong, jstring};
use qingjian_core::{SwipeKey, SwipePoint};
use serde::Deserialize;

use super::{frame, with_session};

#[derive(Deserialize)]
struct SwipeInput {
    points: Vec<SwipePoint>,

    keys: Vec<SwipeKey>,
}

fn input_of(env: &mut JNIEnv<'_>, input: &JString<'_>) -> Option<SwipeInput> {
    let json = env.get_string(input).map(String::from).ok()?;
    serde_json::from_str(&json).ok()
}

/// 滑完一笔：解出的字母进组句，回之后的一帧；JSON 读不出来回 null。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_swipe<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    input: JString<'local>,
) -> jstring {
    let Some(input) = input_of(&mut env, &input) else {
        return std::ptr::null_mut();
    };
    frame(env, handle, |session| {
        session.swipe(&input.points, &input.keys)
    })
}

/// 只解码不进组句（字母直接上屏的英文键盘用）；解不出或出错回空串。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_decodeSwipe<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    input: JString<'local>,
) -> jstring {
    let letters = input_of(&mut env, &input)
        .and_then(|input| {
            with_session(handle, |session| {
                session.decode_swipe(&input.points, &input.keys)
            })
        })
        .unwrap_or_default();
    env.new_string(letters)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
