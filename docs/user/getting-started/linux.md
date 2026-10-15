---
title: Linux
order: 6
description: 在 Linux 上安装青简，支持 Fcitx5 与 IBus 两种输入法框架。
---

青简 Linux 提供预编译包与源码安装，支持 **Fcitx5 与 IBus 两种输入法框架**，候选窗口都用框架自带的面板。
预编译包目前只有 Fcitx5 那份；源码安装时，安装脚本会看这台机器上装了哪个框架，把对应的那份装上。

已验证 Ubuntu 26.04 的 GNOME 桌面（Wayland）上 Fcitx5 5.1.19 的 GTK4、Qt6 应用与 Firefox；
IBus 那份验过按键、候选与上屏的完整链路，各类应用里的表现尚未逐个验证。
其他桌面、其他发行版和旧版应用尚未完成验证。当前支持本地候选、学习和本地整句模型重排，暂不提供云联想与设置窗口。

## 预编译包

预编译包在 Ubuntu 26.04（x86_64）上构建，插件与系统的 Fcitx5 版本绑定；其他发行版请用下一节的源码安装。
先装好 Fcitx5 与 GTK / Qt 输入支持：

```sh
sudo apt install python3 fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

从[下载页](https://qingjian.app/download)下载 `qingjian-<版本>-linux-x86_64.tar.gz`，解压后在解出的目录里执行：

```sh
./install.sh

# 手动启动，保持此终端运行
~/.local/bin/qingjian-linux-server
```

之后的添加输入法、首次输入与源码安装相同，见下一节末尾。更新时下载新包再执行一次 `./install.sh`；卸载用包里的 `./uninstall.sh`。

## 源码安装和首次输入

先装 Rust 1.96、Python 3、pkg-config 与 OpenSSL 开发包；再按你用的框架装那一边的东西。

**用 Fcitx5** 还要 CMake、C++20 编译器与 Fcitx5 的开发包（候选面板插件是 C++ 写的）。Debian / Ubuntu 上是：

```sh
sudo apt install python3 pkg-config libssl-dev cmake g++ \
  libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev \
  fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

**用 IBus**（GNOME 自带的就是它）不需要编译器与开发包：

```sh
sudo apt install python3 pkg-config libssl-dev ibus
```

缺了哪一项，安装脚本开头会直接列出来。X11 应用通常需要设 `GTK_IM_MODULE` / `QT_IM_MODULE` / `XMODIFIERS`
（Fcitx5 填 `fcitx`、IBus 填 `ibus`），环境变化后重新登录。

在源码目录执行：

```sh
# 下载并校验正式词库与本地整句模型
tools/release/data-fetch.sh
apps/linux/scripts/install.sh

# 让青简服务随登录自动启动
systemctl --user daemon-reload
systemctl --user enable --now qingjian-server.service
```

安装脚本会打印这次装了哪几份。两个框架都装着也没关系，它们共用同一个青简服务，互不干扰；
只想装一边就加 `--frontend fcitx5` 或 `--frontend ibus`。加 `--enable-service` 可以让脚本顺手把上面那两条自启命令跑掉。

也可用 `apps/linux/scripts/install.sh --sample --debug` 快速体验少量样例词（例如「你好」），不下载正式词库。
默认安装到 `~/.local`；`--prefix /绝对用户目录` 可更改安装位置。请用相同用户安装、运行，不要使用 sudo。

不想用自启的话，手动启动并保持终端运行：`~/.local/bin/qingjian-linux-server`（关掉终端就结束了，下次登录要再来一次）。

**Fcitx5**：重启 Fcitx5，打开配置工具，取消「仅显示当前语言」，添加「青简」。
**IBus**：**重新登录一次**，然后在系统「设置 → 键盘」里添加「青简」，或 `ibus engine qingjian`。IBus 只在登录时读取输入法列表的位置，装完不重新登录是看不到青简的。如果系统里另外装过青简（比如旧版的 deb 包），重新登录后以这次装的为准；想换回那一份，运行卸载脚本后再重新登录。

切换到青简后输入 `nihao`，空格选中「你好」。单击 `Shift` 切换中英；按键规则见 [按键与快捷键](keys.md#linux)。

## 翻页键

中文组句时，默认用 `[` / `]` 上一页 / 下一页，也可用 `PageUp` / `PageDown`；`Shift + Tab` 上一页，`Tab` 在没有整句补全时下一页，有补全时接受补全。
英文组句中的 `Tab` 是选中高亮候选，不是翻页；完整规则见 [按键与快捷键](keys.md#linux)。

要换成 `-` / `=`，编辑 `~/.config/qingjian/config.toml` 中已有的 `[general]` 分节，然后重启青简服务：

```toml
[general]
page_keys = "-="
```

可选 `"[]"`、`",."`、`"-="`，前一个字符上一页，后一个下一页。不要重复添加 `[general]` 分节。
青简使用这项配置处理候选翻页，修改 Fcitx5 的全局候选翻页热键不会替代它。
选择 `",."` 后，中文组句中的逗号、句号用于翻页；选择 `"-="` 后，中文组句中的 `-` 用于翻页，不再进入英文直输段。
这里匹配的是输入字符，`Shift + -` 的 `_` 和 `Shift + =` 的 `+` 不属于 `"-="`。
已经进入英文直输段时，`-` 原样追加，不用于翻页。

## 配置、隐私和数据

首次运行生成 `~/.config/qingjian/config.toml`，保存后约一秒内生效，不用重启；`[general] preedit` 与 `[general] input_log` 例外，改完要重启青简服务（`systemctl --user restart qingjian-server`）。
`[general] preedit` 可设为 `both`（行内和候选窗口）、`inline`（只在行内）、`window`（只在候选窗口）；应用不支持行内显示时使用候选窗口。
每页候选数、翻页键、学习、日志和辅助语言使用同一配置文件。`learning_language = "off"` 关闭中文候选的辅助语言释义与生词标记。系统面板外观由框架自己的设置控制（Fcitx5 配置工具 / IBus 首选项）。

随附含章·通变与含章·知微两个本地模型：优先用通变处理拼音整句与纠错，没有通变时回退到知微。自己的 `.qjm` 也可放入 `~/.local/share/qingjian/models/` 下对应的模型目录。`[model] enabled = false` 可关闭本地模型，见 [本地整句模型](../input/local-model.md)。

`[general] shift_letter = "compose"` 让 Shift 大写字母参与中文组句，默认 `"passthrough"` 保持临时英文输入。
`scheme = "zhuyin"` 启用大千注音；双拼下 `Shift + V` / `Shift + U` 可进入表达式 / 码点输入。
数字没有对应候选时继续输入，英文直输内容以空格结束时保留空格；英文候选开启后可用数字、翻页键、空格或 Tab 选词。
具体规则见 [按键与快捷键](keys.md#linux)。

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
