//! 青简 macOS 输入法壳（IMK）。
//!
//! 按键进 [`qingjian_core::Engine`]，候选画在自绘 NSPanel 里。数字选词，空格上首选，
//! 回车上屏拼音本身，退格删一个，Esc 清空，上下键移动高亮。
//!
//! 无法 `cargo run`：必须打包成 `.app` 装到 `~/Library/Input Methods/`，见 `scripts/bundle.sh`。

mod app;
mod candidates;
mod error;
mod host;
mod imk;
mod menubar;
mod preferences;

use objc2::{AnyThread, ClassType, MainThreadMarker};
use objc2_app_kit::NSApplication;
use objc2_foundation::NSString;
use objc2_input_method_kit::IMKServer;

fn main() {
    // `--register` 起的子进程：在干净的 TIS 缓存里回读启用状态并切成当前（见 input_source.rs）
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some(app::input_source::FINISH_FLAG) {
        let source_id = arguments.next().unwrap_or_default();
        std::process::exit(app::input_source::finish_register(&source_id));
    }
    // 安装器的 postinstall 以登录用户身份调 `--register`：注册、启用并切成当前输入源后直接退出，不起 IMK
    if std::env::args().any(|argument| argument == "--register") {
        match app::input_source::register_main_bundle() {
            Ok(true) => println!("青简输入源已注册、启用并切成当前输入源"),
            Ok(false) => println!("青简输入源已注册并启用，请在输入法菜单里选择「青简」"),
            Err(error) => {
                eprintln!("青简输入源注册失败：{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let _log_guard = app::logging::init();
    // panic 的位置与 backtrace 记进日志（拦截在 imk::catch_panic，这里只记不碰状态）；
    // 后台线程（云联想）的 panic 也经过这里，否则只会打到没人看的 stderr
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(%info, %backtrace, "panic");
    }));
    let info = app::BundleInfo::from_main_bundle();
    tracing::info!(connection = %info.connection_name, bundle = %info.identifier, "启动");

    // define_class! 的类在首次调用 class() 时才注册到 ObjC 运行时，而 IMKServer 初始化时就会按
    // Info.plist 里的类名查找；找不到会静默退回基类 IMKInputController，表现为按键全部透传。
    // 所以必须先注册类，再建 server。
    let controller_class = imk::QingjianInputController::class();
    tracing::info!(class = %controller_class.name().to_string_lossy(), "控制器类已注册");

    let mtm = MainThreadMarker::new().expect("输入法入口必须在主线程");
    if let Err(error) = host::init(mtm, &info) {
        tracing::error!(%error, "Engine 初始化失败");
        std::process::exit(1);
    }

    // IMKServer 需要在主线程创建，并在整个进程生命周期内活着
    let server = unsafe {
        IMKServer::initWithName_bundleIdentifier(
            IMKServer::alloc(),
            Some(&NSString::from_str(&info.connection_name)),
            Some(&NSString::from_str(&info.identifier)),
        )
    };
    let Some(_server) = server else {
        tracing::error!(
            "IMKServer 创建失败：检查 Info.plist 的 InputMethodConnectionName 与沙盒设置"
        );
        std::process::exit(1);
    };
    NSApplication::sharedApplication(mtm).run();
}
