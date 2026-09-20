---
title: Linux
order: 6
description: 在 Linux 上安装青简，使用 Fcitx5 默认候选面板。
---

青简 Linux 目前提供源码安装，使用 Fcitx5 默认候选面板。已验证 Ubuntu 26.04 上 Fcitx5 5.1.19、GTK4 和 Qt6 应用的 X11 输入；
其他桌面、旧版应用和原生 Wayland 尚未完成验证。当前支持本地候选和学习，暂不提供云联想、神经模型、设置窗口或自动启动。

## 安装和首次输入

先安装 Rust 1.96、CMake、C++20 编译器、Fcitx5、其 GTK / Qt 输入支持，以及 Fcitx5 Core/Config/Utils 与 nlohmann-json 开发包。
按发行版指引启用 Fcitx5；X11 应用通常需要 `GTK_IM_MODULE=fcitx`、`QT_IM_MODULE=fcitx`、`XMODIFIERS=@im=fcitx`，环境变化后重新登录。

在源码目录执行：

```sh
# 下载并校验正式词库
tools/release/data-fetch.sh
apps/linux/scripts/install.sh

# 手动启动，保持此终端运行
~/.local/bin/qingjian-linux-server
```

也可用 `apps/linux/scripts/install.sh --sample --debug` 快速体验少量样例词（例如「你好」），不下载正式词库。
默认安装到 `~/.local`；`--prefix /绝对用户目录` 可更改安装位置。请用相同用户安装、运行，不要使用 sudo。

重启 Fcitx5，打开 Fcitx5 配置工具，取消「仅显示当前语言」，添加「青简」。切换到青简后输入 `nihao`，空格选中「你好」。
单击 `Shift` 切换中英；按键规则见 [按键与快捷键](keys.md#linuxfcitx5)。关闭手动启动的终端会结束青简服务；下次登录后需再次启动。

## 配置、隐私和数据

首次运行生成 `~/.config/qingjian/config.toml`，修改后重启青简服务。
`[general] preedit` 可设为 `both`（行内和候选窗口）、`inline`（只在行内）、`window`（只在候选窗口）；应用不支持行内显示时使用候选窗口。
每页候选数、翻页键、学习、日志和辅助语言使用同一配置文件。`learning_language = "off"` 关闭中文候选的辅助语言释义与生词标记。系统面板外观由 Fcitx5 设置控制。

`[general] shift_letter = "compose"` 让 Shift 大写字母参与中文组句，默认 `"passthrough"` 保持临时英文输入。
`scheme = "zhuyin"` 启用大千注音；双拼下 `Shift + V` / `Shift + U` 可进入表达式 / 码点输入。
数字没有对应候选时继续输入，英文直输内容以空格结束时保留空格；英文候选开启后可用数字、翻页键、空格或 Tab 选词。
具体规则见 [按键与快捷键](keys.md#linuxfcitx5)。

Fcitx5 识别为敏感输入时，可以组句但不会保存输入文本或学习；识别为密码框或禁用输入法的输入框时直接交还应用。
保护依赖应用和其输入支持正确传递标记；本机已验证 Qt6 的敏感标记，GTK4 的动态 PRIVATE 提示尚未传递为敏感输入标记。
普通输入的学习与日志遵循配置。学习数据保存在 `~/.local/share/qingjian`，运行日志保存在 `~/.local/state/qingjian/logs`。
设置了 `XDG_CONFIG_HOME`、`XDG_DATA_HOME`、`XDG_STATE_HOME` 时分别使用对应目录下的 `qingjian`。

## 更新和卸载

结束手动启动的青简服务，重复执行安装命令，然后重启 Fcitx5 和青简。安装会检查文件归属；目标文件被手动修改时会提示保留，请先备份处理。

```sh
apps/linux/scripts/uninstall.sh
# 自定义安装位置时：
apps/linux/scripts/uninstall.sh --prefix /安装时的绝对目录
```

卸载后手动结束青简服务并重启 Fcitx5。卸载保留配置、个人词库、学习数据、已修改的安装文件和其他输入法。
