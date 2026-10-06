# 可选 Rime 后端

雾凇方案通过 `Engine::enable_rime(RimeOptions)` 接入用户提供的原生运行库。输入键、方案部署、spelling algebra、translator、filter、Lua、OpenCC 和用户词库由 librime 执行。默认后端仍是青简；中文词库导入是另一条独立路径，不提供配置或 Lua 的执行能力。

## 边界与部署

`crates/qingjian-core/src/rime/` 为私有模块，`Engine` 提供按键、查询、候选选择、翻页、光标与开关接口。最低 API 表对应 librime 1.17.0，需包含 Lua。读取 API 槽位前检查 `data_size` 和空指针，函数指针显式标注 C 签名；不在构建时链接 librime，也不随包分发运行库、词库或脚本。

动态库使用规范化后的绝对路径。Windows 使用 `LoadLibraryExW` 的 DLL 所在目录与系统目录搜索依赖，其他平台使用 `libloading`。运行库在 `finalize` 完成后才卸载。一个进程只允许一组 library/shared/user/modules；兼容的配置共享同一服务；`EngineSession` 可为输入上下文保存独立原生 session；Linux 使用该机制，Windows 与 macOS 沿用切换焦点时清理当前输入的行为。最后一个实例销毁时，全局注册锁和条件变量会阻止新的初始化，直到 `finalize` 完成。调用锁保证 C API 串行执行。

初始化时加载 `default`、`lua` 和配置中指定的额外已注册模块，执行完整维护并等待部署结束。缺少 API、Lua、额外模块、方案或部署失败均返回 `RimeError`，不静默退回青简词库。Rime 自身报告的组件或 Lua 运行错误仍需检查其原生日志。更改 `[rime]` 后重启服务；配置热加载不更换原生全局服务。

`shared_data` 需包含完整雾凇目录与匹配的 OpenCC 数据；`user_data` 存放 custom.yaml、用户 Lua、短语、build 和用户词库。维护过程由用户安装的 Rime 执行，不由青简解析 YAML 或重写 Lua。原生插件必须已注册在所选库内，`modules` 只启用注册模块，不查找或自动下载 DLL。

## 事件、候选与会话

Windows 协议增加 release 和可选 keysym，TSF 将按下与释放交给原生 composer。Linux 保留 fcitx 原始 X11 keysym，避免 ASCII `p` 与 Windows `VK_F1` 的数值冲突。macOS 接收 KeyDown、KeyUp 和 FlagsChanged，转为 X11 keysym 与 Rime 修饰键掩码。三端分别保留 consumed 与 commit，同一次按键可以提交文本，同时将快捷键交给应用。

查询只复制原生候选的当前页，所有字符串都在 `free_context` 前复制到 Rust 内存。`RimeMenu` 保存页码、页大小、最后一页、高亮和预编辑字符光标；总页数未知，界面只显示当前页，并提示是否还有下一页。青简模型、云候选、简繁过滤及词库排序均绕过原生候选。青简翻译注释可以附加，但不改原生顺序、文本、选字标签或注释。

原生候选携带独立的进程内 session token、查询 revision 和绝对候选下标；已过期的候选和来自其他会话的候选均会被拒绝。按键、输入、光标、分页、开关或选词状态变化后，旧 revision 作废。通过 `EngineSession` 切换上下文时保存原生 session；新上下文使用当前原生方案创建 session。取消输入时还需关闭原生 switcher；仅调用 clear_composition 无法关闭方案菜单。

私密输入不向原生发送按键。原生上屏保留青简历史、译词曝光、输入日志与文字统计，日志来源为 `rime`；CLI 默认回放仅计数并跳过该来源，不能用青简模型估计原生排序质量。统计不根据候选类型推算原生中文词数。

## 验证

普通测试不依赖第三方库或词库。真实集成测试 `crates/qingjian-core/tests/rime_runtime.rs` 及两端 Server 的 `tests/rime_loop.rs` 默认 ignored，调用者需将以下环境变量指向隔离的验证目录：

```text
QINGJIAN_TEST_RIME_LIBRARY=<absolute library file>
QINGJIAN_TEST_RIME_SHARED=<complete Rime Ice directory with compatible OpenCC>
QINGJIAN_TEST_RIME_USER=<isolated writable test profile>
```

```sh
cargo test -p qingjian-core --test rime_runtime -- --ignored --nocapture
cargo test -p qingjian-windows-server --test rime_loop -- --ignored
cargo test -p qingjian-linux-server --test rime_loop -- --ignored
```

Core 的测试会在隔离用户目录下写入 `qingjian-custom-integration/` 来验证个人 YAML、Lua 注释和大于九项的原生分页。不可指向日常用户词库。九宫格测试验证数字拼写映射，不能替代仓输入法 / 元书输入法提供的 t9_processor。

发布前还需在对应系统上完成 TSF / fcitx5 / IMK 安装、应用输入、候选窗、鼠标选词、切换上下文、快捷键放行和私密输入验证。编译与 Server 协议测试不替代这些验证。
