//! tracing 接到 logcat（标签 `qingjian`），panic 也记一条（安卓上 stderr 看不到）；非 Android 目标（本机测试）什么都不做。

#[cfg(target_os = "android")]
pub fn init() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Ok(layer) = tracing_android::layer("qingjian") {
            // jni crate 每次调用都打 trace，只留 debug 及以上
            let _ = tracing_subscriber::registry()
                .with(tracing_subscriber::filter::LevelFilter::DEBUG)
                .with(layer)
                .try_init();
        }
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            tracing::error!(%info, "panic");
            previous(info);
        }));
    });
}

#[cfg(not(target_os = "android"))]
pub fn init() {}
