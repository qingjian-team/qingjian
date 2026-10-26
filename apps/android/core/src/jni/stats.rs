//! App 页面的数据出口（首页统计、统计详情、生词本、词库、高级页的导出与清空）：不要句柄，读数据目录里的文件，每次进页面时读一次。

use std::panic::{UnwindSafe, catch_unwind};
use std::path::Path;

use ::jni::JNIEnv;
use ::jni::objects::{JObject, JString};
use ::jni::sys::{jboolean, jint, jstring};

/// [`crate::StatsSummary`] 的 JSON；出错返回 null，首页把统计卡显示成 0。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_usageStats<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
) -> jstring {
    let Ok(user_dir) = env.get_string(&user_dir).map(String::from) else {
        return std::ptr::null_mut();
    };
    respond(&mut env, "读输入统计", || {
        crate::store::stats::stats_json(Path::new(&user_dir))
    })
}

/// [`crate::StatsDetail`] 的 JSON（`language` 是学习语言的代码）；出错返回 null，详情页显示成 0。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_statsDetail<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    data_dir: JString<'local>,
    user_dir: JString<'local>,
    language: JString<'local>,
) -> jstring {
    let Some([data_dir, user_dir, language]) = strings(&mut env, [&data_dir, &user_dir, &language])
    else {
        return std::ptr::null_mut();
    };
    respond(&mut env, "读统计详情", || {
        crate::store::stats::stats_detail_json(
            Path::new(&data_dir),
            Path::new(&user_dir),
            &language,
        )
    })
}

/// 按 [`crate::VocabularyRequest`]（JSON）先改再列，回 [`crate::VocabularyPage`] 的 JSON；出错返回 null，生词本页显示成空。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_vocabulary<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    data_dir: JString<'local>,
    user_dir: JString<'local>,
    request: JString<'local>,
) -> jstring {
    let Some([data_dir, user_dir, request]) = strings(&mut env, [&data_dir, &user_dir, &request])
    else {
        return std::ptr::null_mut();
    };
    respond(&mut env, "读生词本", || {
        crate::store::vocabulary::vocabulary_json(
            Path::new(&data_dir),
            Path::new(&user_dir),
            &request,
        )
    })
}

fn strings<const N: usize>(env: &mut JNIEnv, values: [&JString; N]) -> Option<[String; N]> {
    let mut out = Vec::with_capacity(N);
    for value in values {
        out.push(env.get_string(value).ok()?.into());
    }
    out.try_into().ok()
}

/// 拦 panic，结果转成 Java 字符串；`what` 写进日志。
fn respond(env: &mut JNIEnv, what: &str, read: impl FnOnce() -> String + UnwindSafe) -> jstring {
    let Ok(json) = catch_unwind(read) else {
        tracing::error!("{what}时 panic");
        return std::ptr::null_mut();
    };
    env.new_string(json)
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// 按 [`crate::DictionaryRequest`]（JSON）先导入或删除再列，回 [`crate::DictionaryPage`] 的 JSON；出错返回 null，词库页显示成空。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_dictionaries<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    data_dir: JString<'local>,
    user_dir: JString<'local>,
    request: JString<'local>,
) -> jstring {
    let Some([data_dir, user_dir, request]) = strings(&mut env, [&data_dir, &user_dir, &request])
    else {
        return std::ptr::null_mut();
    };
    respond(&mut env, "读词库", || {
        crate::store::dictionaries::dictionaries_json(
            Path::new(&data_dir),
            Path::new(&user_dir),
            &request,
        )
    })
}

/// 导出数据要打包的文件（[`crate::export_files`]）的 JSON；出错返回 null，壳提示导出失败。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_learningFiles<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
) -> jstring {
    let Some([user_dir]) = strings(&mut env, [&user_dir]) else {
        return std::ptr::null_mut();
    };
    respond(&mut env, "列导出文件", || {
        crate::store::learning::export_files_json(Path::new(&user_dir))
    })
}

/// 清空学习数据（[`crate::clear_learning`]），返回删掉的文件数；出错返回 -1。壳调之前要先关掉开着的引擎。
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_qingjian_inputmethod_core_Native_clearLearning<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    user_dir: JString<'local>,
    habits: jboolean,
    vocabulary: jboolean,
) -> jint {
    let Some([user_dir]) = strings(&mut env, [&user_dir]) else {
        return -1;
    };
    catch_unwind(|| crate::clear_learning(Path::new(&user_dir), habits != 0, vocabulary != 0))
        .map_or(-1, |removed| jint::try_from(removed).unwrap_or(jint::MAX))
}
