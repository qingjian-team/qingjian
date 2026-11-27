//! 物理键盘出口：null 表示事件由应用处理；候选位图复用同一个画笔与像素出口。

use crate::handle::Handle;
use ::jni::{
    JNIEnv,
    objects::{JObject, JString},
    sys::{jboolean, jint, jlong, jstring},
};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_physicalKey<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    action: jint,
    code_point: jint,
    repeat: jboolean,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: 与 jni::frame 相同，句柄只在主线程使用。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let character = u32::try_from(code_point).ok().and_then(char::from_u32);
    let result = catch_unwind(AssertUnwindSafe(|| {
        handle.session.poll_glosses();
        let frame = handle
            .session
            .physical_key(action, character, repeat != 0)?;
        handle.session.note_physical(&frame);
        if let Some(strip) = &mut handle.strip {
            strip.show(&frame);
        }
        Some(frame.to_json())
    }));
    match result {
        Ok(Some(json)) => env
            .new_string(json)
            .map(JString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        Ok(None) => std::ptr::null_mut(),
        Err(_) => {
            tracing::error!("physical key handling panicked");
            std::ptr::null_mut()
        }
    }
}

/// 物理/触屏模式影响上屏后的候选语义，触摸选词同样使用当前模式。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_setPhysicalInput<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    active: jboolean,
) -> jstring {
    super::frame(env, handle, |session| {
        session.set_physical_input(active != 0)
    })
}

/// 候选视图用桌面布局画当前物理候选页，像素仍由 copyStrip 取。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_renderPhysical<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    scale: ::jni::sys::jfloat,
    dark: jboolean,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: 与 jni::frame 相同，句柄只在主线程使用。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let Some(strip) = &mut handle.strip else {
        return std::ptr::null_mut();
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        strip.paint_physical(&handle.session.physical, scale, dark != 0)
    }));
    let meta = match result {
        Ok(Ok(meta)) => meta,
        _ => {
            tracing::error!("physical candidates rendering failed");
            return std::ptr::null_mut();
        }
    };
    handle
        .session
        .note_displayed(&meta.hits.iter().collect::<Vec<_>>());
    serde_json::to_string(&meta)
        .ok()
        .and_then(|json| env.new_string(json).ok())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 中英切换提示的单格位图，像素沿用 copyStrip。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_renderPhysicalMode<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    label: JString<'local>,
    english: jboolean,
    scale: ::jni::sys::jfloat,
    dark: jboolean,
) -> jstring {
    if handle == 0 {
        return std::ptr::null_mut();
    }
    let Ok(label) = env.get_string(&label).map(String::from) else {
        return std::ptr::null_mut();
    };
    // SAFETY: 句柄和画笔仅在调用方主线程使用。
    let handle = unsafe { &mut *(handle as *mut Handle) };
    let Some(strip) = &mut handle.strip else {
        return std::ptr::null_mut();
    };
    let painted = catch_unwind(AssertUnwindSafe(|| {
        strip.paint_mode(&label, english != 0, scale, dark != 0)
    }));
    let Ok(Ok(meta)) = painted else {
        tracing::error!("physical mode rendering failed");
        return std::ptr::null_mut();
    };
    serde_json::to_string(&meta)
        .ok()
        .and_then(|json| env.new_string(json).ok())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
