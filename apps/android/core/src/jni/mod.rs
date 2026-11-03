//! JNI 出口，对应 Kotlin 的 `app.qingjian.inputmethod.core.Native`。
//!
//! 句柄是 `Box<Handle>`（会话加候选条画笔）的裸指针，Kotlin 侧只在主线程调；每个出口都拦 panic，
//! 穿出 JNI 边界进程就没了。这里是句柄的生命周期，各个操作在 `operations`，候选条位图在 `strip`。

mod catalog;
mod clipboard;
mod cloud;
mod expression;
mod logging;
mod operations;
mod options;
mod shuangpin;
mod stats;
mod strip;
mod swipe;
mod theme;
mod voice;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{jlong, jstring};

use crate::frame::Frame;
use crate::handle::Handle;
use crate::session::Session;

/// 装配 Engine，返回句柄；失败返回 0，原因在 logcat。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_create<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    data_dir: JString<'local>,
    user_dir: JString<'local>,
) -> jlong {
    logging::init();
    let (Ok(data_dir), Ok(user_dir)) = (env.get_string(&data_dir), env.get_string(&user_dir))
    else {
        return 0;
    };
    let (data_dir, user_dir) = (String::from(data_dir), String::from(user_dir));
    let created = catch_unwind(|| {
        crate::assemble(Path::new(&data_dir), Path::new(&user_dir)).map(|engine| {
            Handle::new(Session::new(
                engine,
                Path::new(&data_dir).join("wubi86.tsv"),
                PathBuf::from(&data_dir),
                PathBuf::from(&user_dir),
            ))
        })
    });
    match created {
        Ok(Ok(handle)) => Box::into_raw(Box::new(handle)) as jlong,
        Ok(Err(error)) => {
            tracing::error!(%error, "Engine 装配失败");
            0
        }
        Err(_) => {
            tracing::error!("Engine 装配时 panic");
            0
        }
    }
}

/// 「青简翻译」用的句柄：轻量 Engine（[`crate::assemble_translation`]），只做翻译（`setCloud` / `translate` / `pollTranslation`），用完 `destroy`。
/// `language` 是学习语言的代码（`off` 译成英语）；失败返回 0。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_createTranslation<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    data_dir: JString<'local>,
    user_dir: JString<'local>,
    language: JString<'local>,
) -> jlong {
    logging::init();
    let (Ok(data_dir), Ok(user_dir), Ok(language)) = (
        env.get_string(&data_dir).map(String::from),
        env.get_string(&user_dir).map(String::from),
        env.get_string(&language).map(String::from),
    ) else {
        return 0;
    };
    let created = catch_unwind(|| {
        let (data_dir, user_dir) = (PathBuf::from(data_dir), PathBuf::from(user_dir));
        let language = crate::options::learning_language(&language);
        crate::assemble_translation(&data_dir, &user_dir, language).map(|engine| {
            Handle::new(Session::new(
                engine,
                data_dir.join("wubi86.tsv"),
                data_dir,
                user_dir,
            ))
        })
    });
    match created {
        Ok(Ok(handle)) => Box::into_raw(Box::new(handle)) as jlong,
        Ok(Err(error)) => {
            tracing::error!(%error, "翻译用的 Engine 装配失败");
            0
        }
        Err(_) => {
            tracing::error!("翻译用的 Engine 装配时 panic");
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_destroy<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    // SAFETY: 句柄来自 `create` 的 Box::into_raw，Kotlin 侧销毁后不再使用。
    let mut handle = unsafe { Box::from_raw(handle as *mut Handle) };
    let _ = catch_unwind(AssertUnwindSafe(|| handle.session.reset()));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_reset<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    // SAFETY: 同 `frame`。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let _ = catch_unwind(AssertUnwindSafe(|| handle.session.reset()));
}

/// 跑一个操作并把帧序列化成 Java 字符串；句柄无效或 panic 返回 null，壳当作空帧。
fn frame<'local>(
    env: JNIEnv<'local>,
    handle: jlong,
    operate: impl FnOnce(&mut Session) -> Frame,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: 句柄来自 `create`，Kotlin 侧只在主线程用、销毁后置 0。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let json = match catch_unwind(AssertUnwindSafe(|| handle.operate(operate).to_json())) {
        Ok(json) => json,
        Err(_) => {
            tracing::error!("按键处理 panic，清掉组句");
            let _ = catch_unwind(AssertUnwindSafe(|| handle.session.reset()));
            return std::ptr::null_mut();
        }
    };
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 拿会话跑一个不出帧的操作（拆词、重挂常用语）；句柄无效或 panic 返回 `None`。
fn with_session<T>(handle: jlong, operate: impl FnOnce(&mut Session) -> T) -> Option<T> {
    if handle == 0 {
        return None;
    }
    // SAFETY: 同 `frame`。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    catch_unwind(AssertUnwindSafe(|| operate(&mut handle.session))).ok()
}
