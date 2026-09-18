# Linux Fcitx5 默认面板

第一阶段保留 Rust Server、薄插件和用户目录安装。输入模式、组句、选词、翻页、隐私和失焦提交均在 Server；
插件负责 Fcitx 事件与默认预编辑 / 候选 API。长期自绘方向不变，后续由 Server 渲染并管理 X11 窗口，或向 GNOME 扩展发送位图；本阶段不带相关源码、补丁、渲染 FFI、服务管理或 Debian 包。

## 构建与测试

需要 Rust 1.96、CMake 3.16、C++20 编译器、Fcitx5 Core/Config/Utils 开发包和 nlohmann-json。
构建最低 API 为 Fcitx5 5.1.8，CI 使用 Ubuntu 26.04 的 Fcitx5 5.1.19；Ubuntu 24.04 自带的 5.1.7 不满足要求。
桌面运行已验证的范围见下文，构建版本门槛不代表全部桌面已验收。

```sh
cargo test -p qingjian-linux-server --locked
cargo fmt --all --check
cargo clippy --workspace --exclude qingjian-macos --all-targets --locked -- -D warnings
cargo test --workspace --exclude qingjian-macos --locked
cmake -S apps/linux/fcitx5 -B target/fcitx5-stage1 -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON
cmake --build target/fcitx5-stage1 --parallel 2
env -u DBUS_SESSION_BUS_ADDRESS ctest --test-dir target/fcitx5-stage1 --output-on-failure
python3 apps/linux/tests/installation.py
```

CTest 的 `real-server` 实际运行 Rust Server 并通过 InputContext 输入与点击，默认二进制位置 `target/debug/qingjian-linux-server`。
自定义 Cargo target 时传 `-DQINGJIAN_SERVER_EXECUTABLE=/绝对路径/qingjian-linux-server`。
测试实例关闭所有 Fcitx addon，不依赖桌面总线；插件初始化本身也不连接 D-Bus。真实桌面测试才需要 `fcitx5-modules`、GTK / Qt 输入模块及 `dbus-run-session`。

## 协议与状态

同用户 Unix socket，长度前缀为 4 字节小端，单消息上限 16 MiB；每次插件收发共用 200 ms 截止时间。
连接先 `OpenSession` 校验共享协议版本（6），再强制协商 `LinuxHello` v2，返回既有预编辑设置。
Linux 的首个回包仍是含 `linux_ui` 的 `Update`，不额外发送 Windows DLL 使用的 `SessionOpened`；旧版插件的协议 5 连接会被拒绝，升级时需同时更新 Server 与插件。
旧版本或损坏响应关闭连接、清空显示并放行当前输入；重连创建全新输入状态，旧提交不会重放。
显示 API 同步触发能力变化、失焦或 Reset 时，插件在最终上屏前撤销旧生命周期响应，但保持已接受按键的 Consumed 结果；纯显示失败仍交付有效提交。

同一上下文停用后保留连接和中英模式；Reset 只丢弃输入，销毁上下文或断线才关闭会话。

LinuxEvent 转发能力、按下 / 释放、焦点、停用原因、客户端预编辑事实、带帧身份的候选点击和翻页。
空能力是普通输入；Sensitive 可组句但不学习、不记输入文本；Password / Disable 优先禁用并丢弃输入。
能力先由 Server 确认，再清理可能同步重入的预编辑；Reset / 焦点变化不会丢掉能力通知，清理中重入的新按键已受新隐私状态约束。
能力变化清理组句、暂存透传、学习链与补全；能力即将改变时的 deactivate 原因也触发清理，即使框架此刻仍返回旧能力。
FocusOut 的行内预编辑由框架或声明 ClientUnfocusCommit 的客户端提交，Server 不重复返回它；仅窗口预编辑才返回原样组句。

每次候选响应绑定 generation/context/revision；过期点击不选词。默认面板只展示第一条释义，完成面板更新后报告当前页对应的 `(候选槽位, 0)`。
隐藏、失焦、私密、无候选与过期回报不产生有效展示记录；Server 只凭已生成帧不记展示。

## 路径和排错

用户安装与手动启动见 [Linux 用户说明](../user/getting-started/linux.md)。安装只登记实际绝对插件库路径，不修改系统 Fcitx5 搜索规则。
`QINGJIAN_SOCKET` 可指定绝对 socket 路径；缺省为 `$XDG_RUNTIME_DIR/qingjian.sock`，无 runtime 时用 `/tmp/qingjian-<uid>/qingjian.sock`。
父目录须归当前用户且不可被其他用户写入；socket 权限为 0600。`QINGJIAN_RESOURCES` 覆盖资源根，`QINGJIAN_DICT` 可指定测试词库。
默认数据从可执行文件旁 `../share/qingjian/resources` 找，支持源码开发目录回退。

未出候选时先确认手动 Server 在运行，再查看 `$XDG_STATE_HOME/qingjian/logs/server.*.log`（缺省 `~/.local/state`）。
输入日志由 `[general] input_log` 控制；用户配置和学习文件始终保存在 XDG 用户目录，卸载不删除。

## 验证范围（2026-09-18）

Ubuntu 26.04，系统 Fcitx5 5.1.19-1 / Core7 / modules 经 `dpkg -V` 确认未修改。
隔离 X11（Xvfb）和私有 D-Bus 下，GTK4 4.22.4、Qt6 6.10.2 各覆盖键盘选词、默认面板点击、双输入框切换、Server 中途退出、组句中切密码框；十项均通过。
失焦只提交一次预编辑（框架可能保留拼音分节撇号），另一框独立输入；断线后仅透传新输入并隐藏旧候选。
Qt6 动态 `ImhSensitiveData` 已实测：可组句，实际输入日志与学习文件均不含敏感文本。
本机 GTK4 动态 `PRIVATE` 提示未由前端上报为 Fcitx5 Sensitive 能力，因此不计 GTK4 的该项隐私验收；GTK4 / Qt6 密码框能力转换均已通过。
用户目录安装、重复安装、手动启动、绝对路径加载、卸载保留用户数据和其他输入法均已实测。正式 data-v1 数据包的默认安装也已按 data.lock 校验通过。

native Wayland、其他 GTK / Qt 版本和桌面组合尚未验收；Windows 三个 crate（Server / TSF / Settings）的 x86_64-pc-windows-gnu 全目标交叉检查已通过；Windows / macOS 真机回归交对应环境与 runner，Linux 本地 Router 测试不等于 Windows 真机验收。
原始日志和截图不进仓库。
