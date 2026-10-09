# Android 26 键实验壳

## 范围与架构

`apps/android/native` 是独立产品 `qingjian-android`，版本 `0.1.3-dev`。Java `InputMethodService` 把输入事件交给单线程 JNI 队列；Rust `Bridge` 装配既有 `Engine`、词库、单一英语释义、可选 bigram 模型和词频学习。Core、排序、解析、译词实现均未修改。

JNI 句柄存放在线程内表，创建、查询、译词、学习和销毁必须使用同一工作线程；panic 不跨 JNI 边界。查询限制为 96 个 ASCII 小写拼音字符或分隔符，最多给 Android 壳 32 个候选。先将未注释的候选交给界面，再排队补充英语译词；输入序号、连接和拼音校验阻止旧任务覆盖新输入，译词结果也必须保持候选文字及消耗量一致。`consumed` 通过关闭学习的 probe 引擎执行 Core commit 并读取剩余输入取得，不复制音节对齐逻辑。只有 Android 应用确认 `commitText` 成功后才调用学习；密码框和 `IME_FLAG_NO_PERSONALIZED_LEARNING` 输入框不学习。

显示面暂用 Android Canvas，设置入口使用原生控件；尚未接入 `qingjian-render`。此原型不修改共享渲染器、桌面贴图路径或主题文件，是否采用此接入方式需要维护者确认。没有神经模型、云服务、输入日志、统计页和学习语言选择器。

## 数据与安装

构建需要 `dict.qj`、`glossary-en.qj`，`lm.qj` 可选，来自本仓库生成的产品数据或已按上游摘要校验的产品数据包；来源与许可见 [发版数据流程](release.md) 和 `assets/lexicon/README.md`。构建脚本不下载数据或工具。词库、SO、APK 和签名密钥不入 Git；APK 保留 `src/main/assets/licenses/` 的来源说明和许可证。

首次启动和版本变化时，把随包数据复制到应用私有目录；每个文件先写 `.part`、sync 后原子替换，复制失败不覆盖旧文件。版本标记只在必要文件就绪后更新。用户习惯存放在 `files/user`；应用不申请网络权限，自动备份关闭。

## Linux / WSL 构建

准备 Rust 1.96.0、`aarch64-linux-android` target、JDK 17、Android SDK platform 35 / build-tools 35.0.0、带 Linux 工具链的 NDK r27c、zip。配置示例：

```bash
export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME=/path/to/android-sdk
export QINGJIAN_NDK=/path/to/android-ndk-r27c
export QINGJIAN_ANDROID_DATA_DIR=/path/to/verified/product-data
rustup target add aarch64-linux-android
bash apps/android/build-android.sh
```

默认从 `data/generated` 取数据，生成 `apps/android/build/output/jianci-<版本>-arm64.apk`。`version.properties` 保存 Android 版本与 `versionCode`；默认开发版本带 git 短哈希，脏树加 `+`，可用 `QINGJIAN_ANDROID_VERSION_NAME` / `QINGJIAN_ANDROID_VERSION_CODE` 明确覆盖。

`build-native.sh` 使用 API 26 的 ARM64 clang，链接时设置 16 KiB 页对齐，遵守调用者的 Cargo / Rustup / target 目录配置。`QINGJIAN_ANDROID_NATIVE_OUT` 可覆盖输出目录。只为打包验证重用原生库时，可显式指定 `QINGJIAN_ANDROID_NATIVE_LIB`；调用者需自行确认 ABI 与 ELF 对齐，正常构建不设置此变量。

Java 用 aapt2 / javac / d8 打包，SO 不压缩，zipalign 使用 `-P 16`，随后 apksigner 签名和验证。默认在忽略的 build 目录生成调试密钥；分发签名通过 `QINGJIAN_ANDROID_KEYSTORE`、`QINGJIAN_ANDROID_KEY_ALIAS`、`QINGJIAN_ANDROID_STORE_PASSWORD`、可选 `QINGJIAN_ANDROID_KEY_PASSWORD` 环境变量提供。密钥和密码不写入仓库。

## 触摸与安全区

26 个字母使用 10 列单位宽，第二排偏移半格、第三排一格半；绘制与触摸共用 `KeyboardLayout`。删除键按下删除一次，450ms 后每 70ms 连续删除，松手、移出、取消、多指、隐藏或输入上下文改变即停止。

候选栏在移动事件中跟手更新，松手后交给 `OverScroller` 做惯性滚动；速度使用 px/s，显示坐标转换为 dp。宽度、卡片位置和序号标签按主候选文字缓存；二分查找可见区间，只画可见卡。主候选变化或取消触摸会停止旧滚动，不在新列表上选择旧索引；仅补充译词时保留滚动位置与正在进行的拖动，避免重新测量和布局。

导航栏与屏幕切口的左右 / 底部安全区加在外层容器上，增加窗口高度，不压缩最后一排按键；不把 `ime()` 自身高度当作底部留白。Android 8–10 使用旧系统 inset API，Android 11 起使用按类型的 inset。

## 检查与真机记录

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude qingjian-macos --all-targets --locked -- -D warnings
cargo test --workspace --exclude qingjian-macos --locked
bash -n apps/android/build-native.sh apps/android/build-android.sh apps/android/test-java.sh
bash apps/android/test-java.sh
# 可选：用完整产品数据重跑 JNI 桥接测试。
QINGJIAN_ANDROID_TEST_DATA=/path/to/verified/product-data cargo test --locked -p qingjian-android
```

Rust 默认测试用独立小词库，不要求 CI 下载产品数据；覆盖词库查询、英语释义、部分拼音消耗、选词落盘和异常输入。JVM 检查覆盖数据复制、26 键几何、长按删除、6000 组候选边界；真实 `KeyboardView` 与 Android 假对象共同验证 density 1 / 3、32 / 100 / 1000 候选的拖动和点击路径。每帧只绘制约 4 张可见卡，重绘期间没有重复 `measureText`；假对象不代表 Android 实际帧率或惯性物理。

试用者在荣耀 500、MagicOS 10.0.0.175(C00E170R103P6H1)、QQ 输入框中安装 0.1.3 实验 APK，并确认：输入一段拼音后横向拖动候选条正常；长按删除连续删除且松手停止；三键导航不遮挡底行。此记录来自目标手机使用者，构建环境没有连接手机。提交整理阶段新增的候选与译词分阶段返回通过自动检查，尚未在该手机上复验；横屏、导航模式切换和其他应用兼容性仍待验证。
