# iOS 键盘扩展 spike 设计（2026-10-04）

## 目标

在 `apps/ios` 建第四个壳：容器 App + 键盘扩展，Rust Core 链进扩展，26 键全拼出候选、能上屏，装到真机。
spike 要回答一个问题：**含章模型能不能装进 iOS 键盘扩展的内存上限**（jetsam 按 `phys_footprint` 杀，社区实测 48–60 MB）。

不做：Android、设置页、云联想、双拼 / 注音 / 五笔 / 英文模式、分页滑动、主题切换。模型装不下也是合格结果，这版不裁模型。

## 约束

- Core 一行不动。iOS 壳只做两件事：把触摸翻译成 Core 的按键，把渲染器出的位图贴到候选条。遵守 `docs/contributing.md` 的架构约束。
- 只有全拼。FFI 不暴露 `set_shuangpin` / `set_zhuyin_mode` / `set_code_table` / 英文模式；bundle 不带 `wubi86.tsv`、`english.tsv`、emoji 表。
- 接口手写 C ABI，不用 UniFFI。

## 目录与构建

```
apps/ios/
  core/                    Rust crate qingjian-ios，crate-type = ["staticlib"]
    src/lib.rs             只做 mod 声明
    src/ffi/               extern "C" 函数：engine.rs / keys.rs / frame.rs / model.rs
    src/assembly/          照 Linux Server 的 AssemblySpec 组装 Engine
    src/handle.rs          不透明句柄：Engine + Renderer + 当前帧 + 最近一次位图
    include/qingjian.h     手写 C 头文件，与 ffi/ 一一对应
  Qingjian.xcodeproj
  Qingjian/                容器 App（Swift）：一页，说明怎么开启键盘，显示版本与许可
  Keyboard/                键盘扩展 target（Swift）
  scripts/build-core.sh    cargo build aarch64-apple-ios 与 -sim 两个切片，lipo 进 xcframework，并把数据文件拷进扩展 Resources；Xcode Run Script 阶段调它
  README.md
```

- `qingjian-ios` 进 workspace，`version = "0.1.0-dev"` 写死；发版标签 `ios-v<版本>`。
- 钩子在 Mac 上 `cargo check` 这个 crate 要能过，不依赖 iOS 目标。

## Rust FFI

一个不透明指针 `QjEngine*`，所有函数单线程，Swift 在主线程调。

| 函数 | 作用 |
| --- | --- |
| `qj_engine_new(resources_dir, user_dir) -> QjEngine*` | 组装 Engine：词库必需，释义表 / 语言模型缺哪个少哪个功能；失败返回 NULL 并写日志 |
| `qj_engine_free(engine)` | 释放 |
| `qj_key(engine, QjKey) -> QjOutcome` | 字母、退格、空格、回车、数字选词、Esc；返回 吞掉 / 上屏 / 透传 |
| `qj_select(engine, index) -> QjOutcome` | 点候选 |
| `qj_take_commit(engine, buf, cap) -> len` | 取本次要上屏的 UTF-8 文本 |
| `qj_render(engine, width_pt, scale, QjBitmap*) -> bool` | 渲染器画当前帧（横排，`columns = 0`，内置默认主题）；位图预乘 RGBA，内存归 Rust，下次调用前有效 |
| `qj_hit_test(engine, x, y) -> index` | 点坐标落在哪个候选，没有返回 -1 |
| `qj_flush(engine)` | 落学习数据；扩展失活时调 |
| `qj_load_model(engine, path, kind) -> bool` / `qj_unload_model(engine)` | spike 专用：加载通变或知微装上重打分，或卸掉 |

`QjKey = { kind: u8, ch: u32 }`。字体库 `FontLibrary::system("zh-CN")`；iOS 上 fontdb 能否找到 PingFang 要验，找不到就列死字体文件路径。

## Swift 壳

键盘扩展 `KeyboardViewController`：

- 键盘面：UIKit 自画 26 键 + 退格 + 空格 + 回车 + 123 键 + 地球键（`advanceToNextInputMode`）。`UIButton` + `UIStackView`，不追求外观。
- 候选条：键盘面上方一条 `UIImageView`，每次按键后 `qj_render` 贴图，点击走 `qj_hit_test`。
- 上屏：`textDocumentProxy.insertText`。拼音行只画在候选条里，不往宿主写 marked text。
- 用户数据：App Group `group.app.qingjian` 容器目录作 `user_dir`。
- 数据文件：`dict.qj`、`glossary-en.qj`、`lm.qj`、两份 `.qjm` 在扩展 bundle 的 Resources。
- 调试面板（仅 Debug 构建）：长按地球键弹出，三个按钮加载通变 / 知微 / 卸载，显示 `os_proc_available_memory()` 与 `task_vm_info.phys_footprint`。

容器 App 一页，不做设置。

## 内存实测

口径 `phys_footprint`。四组，每组记启动后与敲 20 字后两个数：

1. 空键盘，不建引擎
2. 引擎 + 词库 + 释义表 + 语言模型
3. 2 + 通变（44 MB）
4. 2 + 知微（53 MB）

结论连真机型号、系统版本写 `docs/notes/ios-memory-spike.md`。预期：mmap 的词库 / 语言模型是 clean 页不计足迹；candle 若把 f16 权重拷进 Tensor，dirty 页约等于文件大小，加引擎大概率超限，那就记录下来，裁剪 / 量化另起项目。

## 测试与验证

- Rust：组装层与 hit test 单元测试在 Mac 跑；一个集成测试按 C 签名把建、按键、渲染、释放调一遍。
- 真机：备忘录里敲 `nihao` 出候选、点候选上屏、退格、切回系统键盘。四步过算 spike 通。
- 模拟器只用来跑通 UI，不测内存。

## 文档同步

- `CLAUDE.md` 目录地图加 `apps/ios` 一行；`docs/plan/roadmap.md` Phase 5 加 iOS 条目。
- 发布流程、隐私清单、审核要求（4.4.1）留到 TestFlight 版再写。
