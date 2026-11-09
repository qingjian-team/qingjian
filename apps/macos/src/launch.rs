//! 进程入口：注册输入源、画主题预览这类命令行用法直接做完退出，否则起日志、注册控制器类、建 IMKServer、跑主循环。

use objc2::{AnyThread, ClassType, MainThreadMarker};
use objc2_app_kit::NSApplication;
use objc2_foundation::NSString;
use objc2_input_method_kit::IMKServer;

use crate::{app, candidates, host, imk};

pub(crate) fn run() {
    // candle 的 CPU 线程池算完每一步先空转再睡，线程一多打字时 CPU 冲到 170%；只有没 Metal 退回 CPU 时才用得上。
    // 池子第一次用时读这个变量，得在起任何线程之前设
    if std::env::var_os("CANDLE_NUM_THREADS").is_none() {
        // SAFETY：进程刚启动，还没有别的线程读写环境变量
        unsafe { std::env::set_var("CANDLE_NUM_THREADS", "2") };
    }
    // `--register` 起的子进程：在干净的 TIS 缓存里回读启用状态并切成当前（见 input_source.rs）
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some(app::input_source::FINISH_FLAG) {
        let source_id = arguments.next().unwrap_or_default();
        std::process::exit(app::input_source::finish_register(&source_id));
    }
    // 主题投稿用：把主题目录画成预览图写进它的 preview/，不起 IMK（主题仓库的 tools/preview.py 调它）
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some(qingjian_render::preview::FLAG) {
        let Some(dir) = arguments.next() else {
            eprintln!("用法：{} <主题目录>", qingjian_render::preview::FLAG);
            std::process::exit(2);
        };
        std::process::exit(qingjian_render::preview::run(
            std::path::Path::new(&dir),
            candidates::family_files,
        ));
    }
    // 安装器的 postinstall 以登录用户身份调 `--register`：注册、启用并切成当前输入源后直接退出，不起 IMK
    if std::env::args().any(|argument| argument == "--register") {
        match app::input_source::register_main_bundle() {
            Ok(true) => println!("青简输入源已注册、启用并切成当前输入源"),
            Ok(false) => println!(
                "青简输入源已注册并启用，但没能切成当前输入源；输入法菜单里没有「青简」的话，注销后重新登录（系统在登录时才刷新输入法列表）"
            ),
            Err(error) => {
                eprintln!("青简输入源注册失败：{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let _log_guard = app::logging::init();
    // panic 的位置与 backtrace 记进日志（拦截在 imk::catch_panic，这里只记不碰状态）；
    // 后台线程（云联想）的 panic 也经过这里，否则只会打到没人看的 stderr。
    // 不记正文：切片越界之类的消息会带上用户打的字
    std::panic::set_hook(Box::new(|info| {
        let location = info.location();
        tracing::error!(
            file = location.map(|location| location.file()),
            line = location.map(|location| location.line()),
            backtrace = %std::backtrace::Backtrace::force_capture(),
            "panic; payload omitted"
        );
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
