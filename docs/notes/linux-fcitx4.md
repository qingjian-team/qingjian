# Linux Fcitx4 适配

Fcitx4 插件与 Fcitx5 插件并列，复用同一个 Rust Server、Linux IPC 协议和 socket 传输代码。Fcitx4 插件在 C++ 壳中注册 `FcitxIMClass`，把按键、候选点击、翻页、焦点和预编辑映射到 Server；输入与排序逻辑仍在 Core。

## 构建与安装

X11 选区备用读取还需要 `libx11-dev` 和 `libxres-dev`。

Ubuntu 22.04 使用 `fcitx-libs-dev`（Fcitx4 4.2.9.8）、`nlohmann-json3-dev`、OpenSSL 开发包及 Rust 1.96；无需 Fcitx5 或 Qt6。`apps/linux/scripts/install.sh --fcitx4` 编译 Server 和插件，装到用户目录。插件库放在 `<prefix>/lib/fcitx/`，配置放在 `$XDG_CONFIG_HOME/fcitx/addon/`，配置中使用插件绝对路径。输入法图标同时放在 `$XDG_CONFIG_HOME/fcitx/imicon/qingjian.png` 和 `$XDG_DATA_HOME/icons/hicolor/48x48/apps/fcitx-qingjian.png`，分别供经典面板和托盘组件查找；只安装 `qingjian.png` 应用图标时，托盘组件会回退到 Fcitx 图标。默认 Fcitx5 安装流程不变；卸载继续按安装清单清理。

`python3 apps/linux/scripts/files.py` 的哈希读取按块处理，兼容 Ubuntu 22.04 自带的 Python 3.10。安装后需在 Fcitx4 配置中启用「青简」，并手动启动 Server。

## 框架差异

- Fcitx4 通过每个输入上下文的数据槽保存 Server 会话编号；候选列表由框架拥有并释放，因此候选的文本、注记和私有点击数据都交给 Fcitx4 管理。
- 候选分页使用 `FcitxCandidateWordSetOverridePaging` 转发给 Server；候选展示回执也发给 Server。组句期间每 80 ms 查询一次本地模型重排结果。
- Fcitx4 只有 `CAPACITY_PASSWORD`，没有 Fcitx5 的 `Sensitive` 和 `Disable` 对应标记。普通敏感字段无法可靠识别；用户文档给出了关闭输入日志与学习的办法。
- 云联想由共用 Linux Server 提供；Fcitx4 默认面板从 Poll 接收云端候选和整句，在候选注记与辅助行加 ☁。密码框不发云请求，未标记的其他敏感输入框仍有上述识别限制。
- 前后文只在 `CAPACITY_SURROUNDING_TEXT` 可用时读取；`FcitxInstanceGetSurroundingText` 返回的字符串由插件释放，cursor/anchor 是 Unicode 字符位置。共用截取器跳过选区，只取两侧最多 80 字，再经 `LinuxEvent::Key` 发给 Server。
- 选区翻译由 Server 按 `[shortcut] translate_selection` 判断按键，返回 `RequestSelection` 后插件读取最多 500 字选区并回复 `LinuxEvent::Selection`。评审时默认面板显示「翻译中…」或译文，轮询直到有结果；提交前再次核对选区文字和位置。
- Chrome 等没有 `CAPACITY_SURROUNDING_TEXT` 的 X11 应用改读 PRIMARY；XRes 只接受与当前焦点窗口同属一个 X11 客户端的选区，等待最多 100 ms，UTF-8 格式和长度不合规时拒绝。提交前会重新读取并比对选中文字，应用能提供光标位置时还会比对位置。
- `OnClose(CET_LostFocus)` 表示框架已处理客户端预编辑；插件把这个事实告诉 Server，防止重复上屏。

## 本机验证

Ubuntu 22.04、Fcitx4 4.2.9.8、GTK3：在隔离的 Xvfb 和私有 D-Bus 会话中加载插件，`fcitx-remote -m qingjian` 返回 `fcitx-qingjian`；GTK3 文本框切换到青简，输入 `nihao`、空格，得到「你好」。同一文本框设为密码模式时输入原样上屏为 `nihao `。真实桌面上的 Chrome 输入框选中「你好」后按 `Ctrl + Super + T`，出现「翻译中…」，译文返回后按 Enter 成功替换选区。Qt5 等其他应用尚未验证。
