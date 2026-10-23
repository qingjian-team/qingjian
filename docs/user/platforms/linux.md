---
title: Linux
order: 2
description: 在 Linux（以及 FreeBSD）上安装青简：Fcitx5 预编译包、Fcitx5 与 Fcitx4 源码安装、首次输入、更新与卸载，以及与电脑版的不同之处。
---

青简 Linux 版作为 Fcitx5 或 Fcitx4 的一个输入法运行，候选使用 Fcitx 自己的候选面板。目前需要在终端中安装，并在每次登录后手动启动后台服务。

- Fcitx5 提供 Ubuntu 26.04 预编译包，其他发行版用源码安装。已验证 Ubuntu 26.04 的 GNOME 桌面（Wayland）上 GTK4、Qt6 应用与 Firefox。
- Fcitx4 提供 Ubuntu 22.04 源码安装，已验证 GTK3 文本框输入。
- FreeBSD 15.1 上用 Fcitx5 源码安装验证过 XFCE 与 KDE Plasma 的 X11 会话，以及 sway 下的 Wayland 应用。

支持本机候选、学习、本地整句模型与可选的 AI 服务，附带青简设置程序。与 macOS / Windows 的差别见 [平台差异](differences.md)。

## Fcitx5 预编译包（Ubuntu 26.04）

预编译包在 Ubuntu 26.04（x86_64）上构建，与系统的 Fcitx5 版本绑定；其他发行版请用下一节的源码安装。先装好 Fcitx5 与应用的输入支持：

```sh
sudo apt install python3 fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

从[下载页](https://qingjian.app/download)下载名字以 `linux-x86_64.tar.gz` 结尾的包，解压后在解出的目录里执行：

```sh
./install.sh

# 启动青简，保持此终端开着
~/.local/bin/qingjian-linux-server
```

之后按下文[首次输入](#首次输入)添加输入法。

## Fcitx5 源码安装

先安装 Rust 1.96 或更新版本，以及编译所需的软件包。Debian / Ubuntu 上是：

```sh
sudo apt install cmake g++ pkg-config python3 libssl-dev \
  libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev \
  fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji \
  fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

三个字体包不能省：没有 `fonts-noto-core` 设置程序打不开，缺了另外两个界面上的中文与表情显示不出来。缺了哪一项，安装脚本开头会列出来。

FreeBSD 上是：

```sh
pkg install bash cmake pkgconf python3 rustup-init nlohmann-json \
  fcitx5 fcitx5-gtk3 fcitx5-qt6 fcitx5-configtool
rustup-init   # 安装 Rust
```

按发行版的说明启用 Fcitx5；X11 应用通常需要设置 `GTK_IM_MODULE=fcitx`、`QT_IM_MODULE=fcitx`、`XMODIFIERS=@im=fcitx`，改完重新登录。

在源码目录执行：

```sh
# 下载并校验正式词库与本地整句模型
tools/release/data-fetch.sh
apps/linux/scripts/install.sh

# 启动青简，保持此终端开着
~/.local/bin/qingjian-linux-server
```

- 只想先试一下安装流程，可用 `apps/linux/scripts/install.sh --sample --debug`，不下载正式词库，只有少量样例词（例如「你好」）。
- 缺省安装到 `~/.local`；`--prefix /绝对路径` 可改安装位置。
- 用自己的账户安装、运行，不要加 `sudo`。

## Fcitx4 源码安装（Ubuntu 22.04）

已在使用 Fcitx4 时，Qt5 应用用 `fcitx-frontend-qt5` 即可。先安装 Rust 1.96 或更新版本，以及：

```sh
sudo apt install cmake g++ pkg-config python3 libssl-dev fcitx-libs-dev libx11-dev libxres-dev nlohmann-json3-dev \
  fonts-noto-core fonts-noto-cjk fonts-noto-color-emoji \
  fcitx fcitx-frontend-gtk3 fcitx-frontend-qt5 fcitx-config-gtk
```

在源码目录执行：

```sh
tools/release/data-fetch.sh
apps/linux/scripts/install.sh --fcitx4
~/.local/bin/qingjian-linux-server
```

最后一条命令需要保持终端开着。执行 `fcitx -r` 重启 Fcitx4，在配置工具中把「青简」加入输入法列表。
只想先试安装流程，可用 `apps/linux/scripts/install.sh --fcitx4 --sample --debug`。
切到青简时，托盘中的输入法图标显示青简标志。

## 首次输入

1. 重启 Fcitx，或注销后重新登录。
2. 打开 Fcitx 配置工具，取消「仅显示当前语言」，添加「青简」。
3. 切换到青简后输入 `nihao`，按空格得到「你好」。

单击 `Shift` 切换中英，按键见 [按键速查](../getting-started/keys.md)。设置在应用菜单的「青简设置」里。
关闭启动青简的那个终端会结束青简；下次登录后需再次执行 `~/.local/bin/qingjian-linux-server`。

## 与电脑版不同之处

- **候选面板**：使用 Fcitx 的候选面板，外观由 Fcitx 的主题决定，青简的主题、字体、横排展开、键位提示不适用。面板上只显示第一条译词，带词性与日语注音。
- **中英模式**：每个窗口各记一份，新窗口从中文开始，可设为从英文开始，见 [中英文](../input/english-mode.md#默认模式与按应用记忆)。Fcitx5 的左 `Shift` 热键需要关掉，见同一页。
- **Tab**：没在打字时交给应用；打拼音时接受整句补全，没有补全时下一页；英文候选里选高亮候选。
- **数字**：当前页没有对应的候选时，数字接在拼音后面，`hao6` 回车原样输出。
- **英文直输**：进入后数字和标点继续追加，以空格结束时保留空格（`gpt-6` 后按空格得到 `gpt-6 `）。
- **整句补全**显示在候选面板下方的提示行，按 `Tab` 接受。
- **私密输入**：Fcitx5 识别为私密的输入框里可以打字，但不记录、不学习、不发送；密码框直接交还应用。这依赖应用正确标记，GTK4 应用的私密提示目前传不过来。Fcitx4 只认密码框标记，其他私密输入框认不出；不想留下记录时，可关掉学习与输入日志，见 [直接编辑配置文件](../advanced/config-file.md#Linux)。
- **暂不支持**：五笔与自制方案、辅码、符号面板、额外翻页键与高亮移动键、标点快捷键、中西文自动空格、拼音带声调、自动补充译词与生词本整句翻译、AI 用量统计、手动联想。

## 更新与卸载

更新：关掉运行青简的终端，重新执行对应的安装命令（预编译包下载新包后再执行 `./install.sh`），然后重启 Fcitx 并重新启动青简。
安装时会检查文件；你改过的安装文件会提示保留，请先备份。

卸载：

```sh
apps/linux/scripts/uninstall.sh
# 自定义过安装位置时：
apps/linux/scripts/uninstall.sh --prefix /安装时的绝对路径
```

预编译包用包里的 `./uninstall.sh`。卸载后手动关掉运行青简的终端，并重启 Fcitx。卸载保留设置、个人词库、学习数据与其他输入法。
