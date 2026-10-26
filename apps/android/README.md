# 青简 Android

Kotlin 的 `InputMethodService` 壳 + Rust 的 `libqingjian_android.so`。Android 的输入法跑在自己的进程里，
没有 Server / 薄壳之分，Engine 直接装在 `.so` 里，经 JNI 调用。

功能怎么用见 [docs/user/platforms/android.md](../../docs/user/platforms/android.md)，各功能怎么接到 Core、取舍与踩坑见 [docs/notes/android.md](../../docs/notes/android.md)。

## 目录

- `core/`：Rust crate `qingjian-android`（workspace 成员）。`assembly` 装配 Engine；`session/` 是 `Session`，把软键盘操作翻成 Engine 调用
  （`keys` 打字选词与补全、`scheme` 切方案与语言、`options` 键盘设置、学习语言与私密输入、`dictionaries` 附加词库重挂、`display` 候选条上看得见的候选、`cloud` 快捷翻译与释义兜底、`prediction` 云联想与整句补全、`clipboard` 拆词与常用语重挂、`voice` 语音结果上屏、按设置换识别方与录音进出、`swipe` 滑行解出的字母进组句）；`frame/` 是回给壳的一帧（JSON，另有语音识别的事 `VoiceEvent`）；
  `store/` 是不经会话的数据读写（`catalog` 符号 / 表情面板目录、`clipboard` / `phrases` 剪贴板与常用语、`code` 代码显示、`dictionaries` 词库页的列出导入删除、`learning` 高级页导出与清空的文件清单、`shuangpin` 方案与键印、`stats` / `vocabulary` App 的统计与生词本）；
  `cloud/` 是云端配置与测试连接；`appearance/` 是外观（内置主题列表、按主题推出的键盘配色、设置页的主题预览、字号档）；`strip/` 把帧里的候选画成候选条位图（`StripPainter`），`Handle` 是句柄背后的会话 + 画笔；
  `jni/` 是出口（`mod.rs` 句柄生命周期，`operations.rs` 各个操作，`strip.rs` 候选条位图：`renderStrip` 画、`copyStrip` 拷像素，`theme.rs` 主题列表、键盘配色、预览与候选条换主题，`voice.rs` 语音与识别配置、测试连接，`swipe.rs` 滑行）。
- `app/`：Android 应用模块，包名 `app.qingjian.inputmethod`，按职责分包，同一职能的再分子包：
  - `core/`：JNI 封装（`Native` / `Engine`，「青简翻译」用的轻量句柄 `TranslationEngine`）与首次启动解随包数据（`BundledData`）；`frame/` 帧的数据类，`strip/` 画好的一帧候选条（位图 + 分界 + 可点区域），
    `store/` 不要句柄的数据（面板目录、剪贴板与常用语、双拼方案、主题与预览图、统计、生词本）。
  - `service/`：`QingjianInputMethodService` 只管生命周期、设置与视图，`InputPanel` 拼出整块视图；
    `input/` 是输入逻辑：`Composer`（键 → Engine 操作 → 输入框）、`Language` 输入语言、`EditorKind` 哪些输入框不过引擎、`EnterLabel` 回车键文字。
  - `keyboard/`：自绘键盘。`KeyboardView` 管触摸，`SideColumn` 九宫格拼音列，`BottomActionsView` 全面屏底部操作条，`Haptics` 震动，`SwipeTrail` 一笔滑行的判定、轨迹与交 Rust 的 JSON；
    `layout/` 键位与排版（`Key`、`KeyboardLayout` 键位表、`KeyboardMetrics`），`paint/` 绘制（`KeyPainter`、`KeyIcons`）。
  - `theme/`：配色。`Palette` 键盘与面板（由 Rust 按设置里的主题推出来），`Appearance` 深浅色，`TextSize` 候选字号档，`AppColors` App 页面（不跟主题）。
  - `panel/`：盖住候选条与键盘的面板。`FunctionPanel` / `KeyboardPickerPanel` / `CatalogPanel`（符号与表情）各一种，`PanelKind` 面板种类，`PanelActions` 是面板能做的事（服务实现）；
    `base/` 是共用的底子（`PanelView` 表头与滚动、`Tabs`、`ScrollState`、`Ink` 画笔、`PanelIcons`），`clipboard/` 剪贴板与常用语（`ClipboardPanel`、拆词 `PiecesPanel`），
    `editor/` 浮在键盘上方接管打字的编辑条（`FloatingEditor`、常用语编辑框 `PhraseEditor`）。
    `VoicePanel` 是说话时盖上的语音面板。
  - `voice/`：语音输入。`VoiceInput` 是唯一入口（长按空格、工具栏话筒、底栏语音键），`VoiceRecognizer` 识别方式的接口（占位 `UnavailableRecognizer`、Rust 侧识别方 `EngineRecognizer`），`MicrophonePermissionActivity` 代输入法申请录音权限，`VoiceConfig` 是「语音识别」页的配置（识别服务、计费方式、密钥密文，与云端大模型分开存）。
  - `translate/`：快捷翻译，翻译条 `TranslateBar` 与流程 `QuickTranslate`；系统文字选择菜单的「青简翻译」`ProcessTextActivity`；`cloud/` 是云端配置、密钥（Android Keystore）与当前网络（`Network`，「只在 Wi-Fi 下使用」）。
  - `clipboard/`：系统剪贴板的监听与图片的 content:// 出口。
  - `candidates/`：键盘上方那一条。`CandidateBarView` 在三种样子之间切：`CandidateStrip` 候选（贴 `StripSource` 给的位图，拼音行钉住、候选随滚动）、
    `Toolbar` 工具栏（浮在键盘上的圆角卡片，面板表头同样是浮起卡片）、`ResizeHandle` 调高度手柄。
  - `popup/`：`ChoicePopup` 长按气泡，候选条（译词）与数字键（字母 / 数字）共用。
  - `settings/`：`Settings` 设置快照与 `BottomAction`；App 各页分三块：`home/` 启动页（`MainActivity` + 三个 `AppPage`：`HomePage` / `VocabularyPage` / `MePage`），
    `page/` 二级页（`KeyboardSettingsActivity` / `AppearanceActivity` / `DictionariesActivity` / `StatsActivity` / `AdvancedActivity` / `AboutActivity` / `VoiceSettingsActivity` 继承 `PageActivity`），`widget/` 控件积木（`SettingsWidgets`、`TabBar`、`Wash` 渐变底、`BarChart` 柱状图、`ThemePreview` 主题预览）。

## 构建

需要 Android SDK（platform 36、NDK 29.0.13599879）、JDK 17 以上、Rust 目标与 cargo-ndk；不需要 Android Studio。

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
echo "sdk.dir=$HOME/Library/Android/sdk" > apps/android/local.properties

cd apps/android
./gradlew installDebug        # 编 .so、放随包数据、打包、装到连着的设备
adb logcat -s qingjian        # Rust 侧日志
```

`preBuild` 之前会跑两个任务：`cargoNdk`（`cargo ndk` 编 arm64-v8a 的 release `.so`）与 `stageData`
（`data/generated/` 有 `dict.qj` 就带产品数据，没有退回 `assets/sample/` 样例）。只编 arm64。

装好后打开「青简」，按页面上两步启用并切换输入法。

`./gradlew assembleRelease` 出 release 包；版本号取自 `core/Cargo.toml`，签名用的环境变量与发版流程见 [docs/notes/release.md](../../docs/notes/release.md)。

Rust 侧在本机就能测：`cargo test -p qingjian-android`。

## 约定

- JNI 句柄只在主线程用；每个出口拦 panic，出事返回 null，壳当空帧处理。
- 随包数据首次启动从 assets 解到 `files/data/`（`.qj` 要 mmap，领域词库在 `dicts/` 下），装了新包才重解；学习数据在 `files/user/`，导入的词库在 `files/user/dicts/`。
- 解数据和装配在后台线程，没装好之前字母原样上屏。
- 密码、网址、邮箱、非文本输入框不过引擎。
- 不依赖 `qingjian-platform`：它带进的 native-tls / OpenSSL 在 Android 交叉编译过不了；云端直接引 `qingjian-predict` 并关缺省 feature、开 `rustls`。
