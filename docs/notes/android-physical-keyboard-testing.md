# Android 物理键盘验收

构建依赖与安装见 [Android 构建说明](../../apps/android/README.md)。提交前执行 Rust 测试、clippy 和 Android `testDebugUnitTest`；模拟器构建用 `assembleDebug -PandroidAbis=x86_64 -PsideBySide`，ARM64 设备省略 ABI 参数。

## 回归项目

系统启用青简，连接物理键盘并关闭「使用物理键盘时显示屏幕键盘」。保持默认的「上屏后联想」开启，分别在原生编辑框与 WebView 中检查：

1. 冷启动后输入 `nihao`，显示带拼音、序号、高亮与页码的桌面候选窗；`Space` 选词、`Enter` 原样上屏，完成后候选立即收起，一次 `Backspace` 可删除已上屏文字。数字、点击候选和标点上屏同样不残留候选。
2. `1`–`5` 选择当前页候选，`↑` `↓` 移动高亮，`PageUp` / `PageDown` 与箭头点击翻页。`←` `→`、`Home` / `End` 编辑拼音，`Backspace` / `Delete` 删除对应位置，`Esc` 清除。按住退格删空拼音后停止，松开再按交给应用；无组字时编辑键与翻页键交给应用。
3. 单击 `Shift` 切换语言并短暂显示「中 / 英」；快速切换、继续输入及换焦点后没有旧提示残留。`Shift + A` 输入大写且不误切，`Ctrl + A/C/V` 等应用组合键可用。另验单击 `Ctrl`、`Ctrl + Alt + Space` 及交换 `Space` / `Enter` 设置。
4. 候选随输入光标、输入框移动与滚动更新；底部自动上翻、右侧内收。全屏原生应用和 WebView 可输入、选词；候选窗外触摸交给应用，页面不因候选窗缩小。
5. 切换输入框、应用、重启输入连接与旋转后，旧组字不进入新字段。拔出重接键盘、修饰键按下时断开、系统屏幕键盘开关变化后，按键与显示恢复正常。
6. 九宫格设置下物理字母键使用全拼，切回触屏恢复布局；双拼、注音、五笔及英文候选设置各按所选方案检查。密码、邮箱与数字框直接输入；网址框可输入中文搜索词且标点为半角。
7. 外观页连接键盘时显示桌面候选预览，断开后显示触屏键盘，重连后恢复桌面候选；切回触屏选词仍提供上屏后联想。

每轮记录 Android 版本、设备、应用、受测 APK 与执行项目；新增或变更的行为需重新运行对应项目。

## 已执行的功能验收

2026-11-25，Android 16 Google APIs x86_64 模拟器（Emulator 37.3.9，KVM，4 核 / 8 GiB）。使用虚拟 USB 键盘事件，在原生 EditText 与本地 WebView 中验证；横竖屏分别为 1600×1000、1000×1600，180 dpi。

| 范围 | 结果 |
|---|---|
| 默认联想开启时的上屏收窗、单次退格、Shift 提示、连续输入及全屏 | 31 个检查点、190 条断言通过；旧包对照有 18 条失败 |
| 光标与输入框移动、滚动、边缘定位、原生及 WebView 全屏 | 46 条位置与全屏断言通过 |
| 焦点切换、输入连接重启、旋转与窗口生命周期 | 128 条断言通过 |
| 候选外触摸、触屏上屏后联想、连接变化后的外观预览 | 截图与实际输入确认通过 |

用户另确认 Samsung SM-X800（Android 16）实机测试通过。此确认对应下列 ARM64 功能验收包；上述模拟器记录与实机确认均为功能验收快照，提交前最终构建与检查结果另记于 PR。

| 受测包 | SHA256 |
|---|---|
| x86_64 模拟器 | `a2d215c7f8568b0aa19805fdb7a8781ad1a34414dc4e854aa8e963a199857163` |
| ARM64 实机 | `b921d9ca770bb274d4fe582d7c66e6d2a10f9dc44a8f83bd4d64368cc187acd7` |

## 最终分支回归

2026-11-26，在同一 Android 16 模拟器与原生 EditText / WebView 中，以最终整理分支重新构建并安装 APK，冷启动后运行默认联想场景（31 个检查点、190 条断言）及光标跟随 / 全屏场景（46 条断言），全部通过。Android 13 项单元测试与 x86_64 APK 构建通过。

最终 APK SHA256：`942e41a9313329bfb8aa93ebc5f24419cbad3529e2cba339bb29f2bfd98b5935`。

## 截图与录屏

光标跟随与默认设置素材更新为最终分支回归录制；外观预览素材来自前述功能验收，均使用测试文本。

- [光标跟随与全屏录屏](android-physical-keyboard-evidence/anchor-follow/recording.mp4)
- [默认设置下收窗与中英提示录屏](android-physical-keyboard-evidence/default-physical/recording.mp4)
- [全屏底部输入时的候选位置](android-physical-keyboard-evidence/anchor-follow/06-fullscreen-bottom.png)
- [切换中文提示](android-physical-keyboard-evidence/default-physical/14-chinese-hint.png)
- [连接物理键盘时的外观预览](android-physical-keyboard-evidence/theme-hotplug/06-physical.png)
