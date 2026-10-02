//! JNI 生命周期与异常边界；引擎句柄只在创建它的单一 Java 工作线程中有效。

mod bridge;

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use serde_json::json;

use bridge::Bridge;

thread_local! {
    static ENGINES: RefCell<HashMap<i64, Bridge>> = RefCell::new(HashMap::new());
}

static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_qingjian_android_NativeEngine_create(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    data_dir: JString<'_>,
    user_dir: JString<'_>,
) -> jlong {
    catch_unwind(AssertUnwindSafe(|| {
        let data: String = env.get_string(&data_dir).ok()?.into();
        let user: String = env.get_string(&user_dir).ok()?.into();
        let engine = Bridge::open(Path::new(&data), Path::new(&user)).ok()?;
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        ENGINES.with(|engines| engines.borrow_mut().insert(handle, engine));
        Some(handle)
    }))
    .ok()
    .flatten()
    .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_qingjian_android_NativeEngine_query(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    input: JString<'_>,
) -> jstring {
    let response = catch_unwind(AssertUnwindSafe(|| {
        let input: String = match env.get_string(&input) {
            Ok(input) => input.into(),
            Err(_) => return json!({"candidates": [], "error": "读取拼音失败"}),
        };
        ENGINES.with(|engines| match engines.borrow_mut().get_mut(&handle) {
            Some(engine) => engine.query(&input),
            None => json!({"candidates": [], "error": "引擎尚未就绪"}),
        })
    }))
    .unwrap_or_else(|_| json!({"candidates": [], "error": "查询失败"}));
    env.new_string(response.to_string())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_qingjian_android_NativeEngine_learn(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    text: JString<'_>,
    input: JString<'_>,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Ok(text) = env.get_string(&text) else {
            return;
        };
        let text: String = text.into();
        let Ok(input) = env.get_string(&input) else {
            return;
        };
        let input: String = input.into();
        ENGINES.with(|engines| {
            if let Some(engine) = engines.borrow_mut().get_mut(&handle) {
                engine.learn(&text, &input);
            }
        });
    }));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_qingjian_android_NativeEngine_annotate(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
    input: JString<'_>,
) -> jstring {
    let response = catch_unwind(AssertUnwindSafe(|| {
        let input: String = env.get_string(&input).ok()?.into();
        Some(ENGINES.with(|engines| {
            engines
                .borrow_mut()
                .get_mut(&handle)
                .map(|engine| engine.annotate(&input))
                .unwrap_or_else(|| json!({"candidates": [], "error": null}))
        }))
    }))
    .ok()
    .flatten()
    .unwrap_or_else(|| json!({"candidates": [], "error": null}));
    env.new_string(response.to_string())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_qingjian_android_NativeEngine_destroy(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    handle: jlong,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        ENGINES.with(|engines| engines.borrow_mut().remove(&handle));
    }));
}

#[cfg(test)]
mod tests;
