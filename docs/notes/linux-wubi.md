# Linux 五笔验证记录（2026-11-05）

设计见 [linux-wubi.md](../design/linux-wubi.md)，执行清单见 [实现计划](../plan/linux-wubi-implementation.md)。

## 实现边界

三个前端共用 Linux Server，不新增协议。Server 接入 `SchemeSlot`、Core 自动上屏与顶字，安装和打包共用资源清单携带 `assets/schemes` 及许可证。设置程序开放已有方案入口；不包含辅码、键位提示、字根图、新五笔版本或排序调整。

有效方案、规则或已启用码表改变时，所有上下文取消未提交输入，候选展示身份、翻译、云请求与重排状态作废。无关配置或未选方案变化不取消输入。坏 TOML 保留上次有效配置，资源更新仍按该配置的禁用词库列表应用。码表不可用时复用共用层的拼音回退。

## 自动化入口

```bash
CARGO_BUILD_JOBS=3 cargo test -p qingjian-linux-server --locked
CARGO_BUILD_JOBS=3 cargo test -p qingjian-settings --locked
python3 apps/linux/tests/installation.py
cmake -S apps/linux/fcitx5 -B target/fcitx5-wubi -DBUILD_TESTING=ON
cmake --build target/fcitx5-wubi -j3
env -u DBUS_SESSION_BUS_ADDRESS ctest --test-dir target/fcitx5-wubi --output-on-failure
cmake -S apps/linux/fcitx4 -B target/fcitx4-wubi -DBUILD_TESTING=ON
cmake --build target/fcitx4-wubi -j3
ctest --test-dir target/fcitx4-wubi --output-on-failure
```

Server 的 38 项单元测试通过，覆盖纯形码 / 混输、满码唯一与重码、顶当前高亮、退格 / 光标 / 数字 / 标点 / 空码、规则关闭、配置热加载、个人方案覆盖、码表失效回退、自制双拼、导入词库更新 / 禁用 / 删除、多上下文清理与展示身份失效。自制方案的 `table` 指向子目录时，也检测码表修改、删除与恢复，不要求重新保存描述文件。安装测试检查源码安装及共用发行资源输出里的码表、描述、README 和许可证，并确认卸载保留个人方案。设置程序另用独立进程与临时资源目录验证五笔发现、许可证与码表加载。

Fcitx5 `code-server` 使用真实插件与真实 Server，断言「开一乙开」及顶字提交与下一段 `g` 预编辑共存。Fcitx4 的 CTest 只有共用 surrounding-text 测试，不把它说成五笔端到端验证。

## 真实 GTK3 输入

在 Ubuntu 24.04.4 x86_64 上运行，GTK 3.24.41、Fcitx5 Core 5.1.12、Fcitx4 4.2.9.9、IBus 1.5.29-rc2。三个前端均在真实 GTK3 Entry 中通过：输入 `ga ` 得到「开」，`ggll` 自动上屏「一」，`aaaa` 后向下移动高亮再敲 `g` 顶「乙」，随后 `a ` 上屏「开」，最终文本严格等于「开一乙开」。

使用 Xvfb 与私有 D-Bus，独立 HOME、XDG 配置 / 数据 / 学习目录和 Unix socket；不是在用户正在使用的桌面输入框里测试。Fcitx5 58 项 CTest、Fcitx4 1 项 CTest 通过。未切换用户当前输入法，也未重新安装开发版青简。

```bash
CARGO_BUILD_JOBS=3 cargo build -p qingjian-linux-server -p qingjian-linux-ibus --locked
apps/linux/tests/wubi/session.sh fcitx5
RUN_IBUS_PROTOCOL=0 apps/linux/tests/wubi/session.sh ibus
FCITX4_ROOT="$PWD/target/fcitx4-runtime" apps/linux/tests/wubi/session.sh fcitx4
```

脚本需要 `dbus-run-session`、Xvfb、xdotool、系统 `/usr/bin/python3` 的 PyGObject / GTK3，以及相应框架与 GTK 输入模块。`KEEP_TEST_DATA=1` 保留临时日志；默认退出清理进程与测试目录。GTK 主循环不可被同步自动化阻塞；激活输入法后等待焦点生效，并先发 Escape 建立上下文，再输入测试编码。

Fcitx4 运行时可从发行版 deb 解包到 `target/fcitx4-runtime`，含 `fcitx-bin`、`fcitx-data`、`fcitx-modules`、`fcitx-module-dbus`、`fcitx-module-x11`、`fcitx-ui-classic`；共享库依赖需已可用。脚本将插件和运行时路径写入临时 addon 描述，不安装到系统。Ubuntu 的 `fcitx-libs-dev` 会依赖与 `fcitx5` 冲突的 `fcitx-bin`，不要为验证直接安装；本次曾触发该冲突，已恢复 Fcitx5 软件包，并从仍在运行的进程恢复原主程序与 table / pinyinhelper 插件，逐个核对 SHA256 一致。原桌面 Fcitx5 与青简服务仍在运行。

## IBus 协议测试的环境限制

`tests/ibus_engine/code.rs` 另有真实 daemon 信号测试，通过 `QINGJIAN_IBUS_WUBI_TEST=1` 显式启用；要求上述微型五笔夹具，与原拼音测试的开关分开，不会把未启动 daemon 当作通过。单次测试限时 45 秒，脚本外层再限时 90 秒。

本机 IBus 1.5.29-rc2 与锁定的 zbus 5.19.0 组合在接收消息时出现 `Invalid field reconstruction: InvalidName("Invalid unique name...")`，栈在 zbus `FieldPos::read` / `QuickFields::sender` / `SocketReader`。本次未修改 IBus 或第三方依赖，不宣称该 Rust 信号测试通过；用 `RUN_IBUS_PROTOCOL=0` 单独验证 GTK3，真实输入成功。默认脚本仍执行协议测试，失败会返回非零，不静默绕过。仓库原 IBus 工程记录使用的 1.5.34 环境未在本机重验。

## 最终构建与门禁

以下命令在最终修改上通过（非 macOS workspace 的既有忽略项保持原样）：

```bash
CARGO_BUILD_JOBS=3 cargo build -p qingjian-linux-server -p qingjian-linux-ibus -p qingjian-settings --locked
CARGO_BUILD_JOBS=3 cargo test --workspace --exclude qingjian-macos --locked
CARGO_BUILD_JOBS=3 cargo clippy --workspace --exclude qingjian-macos --all-targets --locked -- -D warnings
cargo fmt --all --check
shellcheck -S warning apps/linux/tests/wubi/session.sh
bash -n apps/linux/tests/wubi/session.sh
python3 -m py_compile apps/linux/tests/wubi/widget.py
git diff --check
```

最后一轮 Fcitx5 CTest 与三种前端 GTK3 输入使用更新后的 Server 二进制重跑，均通过；这些结果不包括上述显式启用的 IBus Rust 信号测试。

## 未覆盖范围

- 未在本次验证 native Wayland、GTK4、Qt、浏览器、FreeBSD、Windows 或 macOS；历史验证不能充当本次五笔实测。
- 未构建 / 安装 deb 与 rpm，也未运行发行版容器矩阵；本机没有容器运行时与 rpmbuild。共用 `resources` 输出检查不能替代这些验证。
- 不自动提交、发布、创建 PR 或更新用户当前青简安装；字根图与辅码仍保持待实现。
