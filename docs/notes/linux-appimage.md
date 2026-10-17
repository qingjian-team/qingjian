# Linux AppImage

一个文件装下 Server、设置程序、Fcitx5 插件与产品数据，下载、加执行权限、运行一次就能用，不用编译也不用 root。
脚本在 `apps/linux/appimage/`：`build.sh` 打包，`AppRun` 是包的入口，`test/test.sh` 在各发行版的容器里实测。用户怎么装见 [user/getting-started/linux.md](../user/getting-started/linux.md#appimage)。

## 输入法为什么不能「双击就跑」

AppImage 的思路是程序留在包里、运行时挂载，什么都不装。输入法的两半不一样：

- **Server 与设置程序**是独立进程，从挂载点里跑没问题（资源按 exe 上两级的 `share/qingjian/resources` 找，包里照 `usr/` 布局放，代码不用改）。
- **Fcitx5 插件**要被宿主的 Fcitx5 进程 `dlopen`，而 AppImage 每次挂载的位置都不同、退出就卸掉。所以必须拷到磁盘上。

于是 `AppRun install` 做四件事，全在用户目录，并把写过的文件记进 `~/.local/share/qingjian-appimage/installed`，`uninstall` 照单删：

| 文件 | 作用 |
|---|---|
| `~/.local/share/qingjian-appimage/qingjian.so` | 插件的拷贝；不放 `~/.local/share/qingjian`，那是学习数据 |
| `~/.local/share/fcitx5/{addon,inputmethod}/qingjian.conf` | `Library=` 指向上面那份拷贝 |
| `~/.config/autostart/qingjian-server.desktop` | 登录时 `"<AppImage 路径>" server`；XDG 自启，各桌面都认，不依赖 systemd 用户会话 |
| `~/.local/share/applications/qingjian-settings.desktop` | 菜单里的「青简设置」，`"<AppImage 路径>" settings` |

另拷 hicolor 图标（同名的已经在、又不是上次自己装的就不动）。装完立刻在后台起 Server，等它的进程真出现了才返回（不等的话紧接着的 `uninstall` 会漏停它），不用重新登录。

- **挪动 AppImage 文件后要再 `install` 一次**：自启项与菜单项写的是它的绝对路径。重装时上次记下的文件（含图标）仍算自己的，照样覆盖并记下，否则卸载会把图标留下、之后的源码安装拒绝覆盖（虚拟机实测抓到的）。
- **同一路径换成新版本**：`server` 每次启动先比对包里的插件与拷出去的那份，不同就换掉，并提示重启 Fcitx5。
- **与其他装法共存**：上面几个文件第一行写着标记；同名文件没有标记（源码安装、发行包装的）就拒装，并提示先卸载那一份。
  只认「exe 旁边有 `AppRun`」的 Server 进程才会被停，源码装的那份不碰。
- Fcitx5 的 `addon` 配置是 INI，`#` 开头的行是注释，标记行不影响解析。

## 构建环境与能跑在哪

插件用到 Fcitx5 5.1.8 的接口（#90 定的下限），宿主 Fcitx5 低于它就加载不了，AppImage 改变不了这一点。所以构建环境选「Fcitx5 ≥ 5.1.8 的发行版里 glibc 最老的」：**Debian 13**（Fcitx5 5.1.12、glibc 2.41），见 `apps/linux/appimage/Dockerfile`。

| 要求 | 来自 | 不满足时 |
|---|---|---|
| glibc ≥ 2.41 | 在 Debian 13 上编 | `install` 报「要 glibc 2.41 及以上」（Ubuntu 22.04 实测） |
| Fcitx5 ≥ 5.1.8 | 插件的 API 下限 | `install` 报 Fcitx5 版本太旧（Ubuntu 24.04 的 5.1.7 实测） |
| OpenSSL 3（`libssl.so.3`） | 云联想的 TLS | 用宿主的，不打进包：CA 证书与配置的路径各发行版不同 |
| FUSE（`fusermount3`） | type2 runtime | 没有就 `APPIMAGE_EXTRACT_AND_RUN=1`（解到临时目录再跑，实测可用） |

按这两条，Debian 13、Ubuntu 25.04 及以后、Fedora 42 及以后、Arch 都满足；实测过的是 Debian 13、Ubuntu 26.04、Fedora 44 与 Arch（见下）。
插件只依赖宿主的 `libFcitx5Core.so.7` / `libFcitx5Utils.so.2` / `libFcitx5Config.so.6` 与 libstdc++，这些都不打进包（打进去反而会和 Fcitx5 进程里已加载的那份冲突）。

runtime 用 AppImage 官方的 type2 runtime（静态链接，宿主不用装 libfuse2），版本与 SHA256 钉在 `build.sh` 里；
换版本时下载 `runtime-x86_64`、`.sig` 与 `signing-pubkey.asc`，`gpg --verify` 过了再改（主密钥指纹 `570C 77AC EA40 C0F1 B758 902C BF96 CCA5 6490 F695`）。
squashfs 由 `mksquashfs`（zstd）直接打，再接在 runtime 后面，不依赖只有 continuous 版的 appimagetool。产品数据照源码安装那样经 `files.py` 按 `data.lock` 校验后放进包。

## 实测

`apps/linux/appimage/test/test.sh` 每个发行版一个容器，装真的 Fcitx5、Mousepad（GTK3）、Xvfb 与 FUSE，以普通用户跑 `test/guest.sh`：

1. 把 AppImage 放到带空格与中文的路径（`~/Applications/青简 测试/`），`--version`、`install`；
2. 检查装出的文件、`Library=` 指的插件在、两个 `.desktop` 过 `desktop-file-validate`；
3. 确认 `install` 起了 Server，停掉后**照自启项的 `Exec=` 原样再起一次**（引号与转义错了登录时就起不来）；
4. Xvfb + `dbus-run-session` 里起 Fcitx5（profile 里只放键盘与青简）、开 Mousepad，xdotool 打字，全选复制读剪贴板比对；
5. 设置程序 `--png` 截图（只验能从 AppImage 起来、画得出页面）；
6. 重装一次再 `uninstall`，确认文件（含图标）删了、Server 停了、用户数据还在。

真桌面的整套场景（重启后靠登录自启打字、插件同步、卸载后再重启）由 `tools/verify/linux/appimage.sh` 在 Linux 虚拟机里跑，见 [testing.md](testing.md)。

拒装的两组（Ubuntu 24.04、22.04）只验提示对不对。

在 Ubuntu 宿主上跑容器测试有个坑：宿主的 AppArmor 按路径给 `/usr/bin/fusermount3` 套配置，容器里别家发行版的 fusermount3 也被套上；
Debian 13 的 fusermount3 3.17 要 `dac_override`，被拦下后普通用户挂不上 AppImage（内核日志里是 `apparmor="DENIED" … capname="dac_override"`）。
真机上不会遇到（别家没有这份配置，Ubuntu 自己的 fusermount3 与配置配套），测试镜像里把它挪个位置、原处留符号链接绕开。

### 2026-10-17 的结果

容器矩阵（`test/test.sh`，AppImage 0.2.2-dev，148 MB）：

| 发行版 | Fcitx5 | glibc | 结果 |
|---|---|---|---|
| Debian 13 | 5.1.12 | 2.41 | 15 项全过：打字 4/4、设置程序截图 28 张 |
| Ubuntu 26.04 | 5.1.19 | 2.43 | 同上 |
| Fedora 44 | 5.1.23 | 2.43 | 同上（设置程序要带上 Fedora 字体路径的修复，见下） |
| Arch | 5.1.23 | 2.44 | 同上 |
| Debian 13，不用 FUSE | 5.1.12 | 2.41 | 同上（`APPIMAGE_EXTRACT_AND_RUN=1`） |
| Ubuntu 24.04 | 5.1.7 | 2.39 | 按预期拒装，提示 Fcitx5 太旧 |
| Ubuntu 22.04 | 5.0.14 | 2.35 | 按预期拒装，提示 glibc 太旧 |

打字用例：`nihao` 空格 → 你好、`nihao` 回车 → nihao、`jintiantianqihenhao` 空格 → 今天天气很好、`firefoxliulanqi` 空格 → Firefox浏览器（后两条要正式词库与含章模型）。
设置程序的截图脚本在 Linux 上会停在没有的统计页（脚本自身的问题，#682 修），这里只看它能从 AppImage 起来、画得出页面。

Fedora 桌面默认装的是 Noto 可变字体，设置程序原来的字体清单一个都对不上，画不出任何页面；中文可变字体还是 CFF2 轮廓，要一并认。
这与 AppImage 无关（Fedora 上源码安装一样），单独修在 bamboo-slip 的字体清单里。

真桌面（QEMU/KVM 虚拟机，Debian 13.7 官方 generic 镜像 + XFCE 4.20 + LightDM 自动登录，Fcitx5 5.1.12 由 im-config 设为输入法）：

1. AppImage 放在 `~/Applications/青简 测试/`，`install` 后起了 Server，把青简加进 Fcitx5 后在 Mousepad 里打字 4/4，候选窗是 Fcitx5 的默认面板（带译词注记）；
2. 照「青简设置」菜单项（`gio launch`）打开设置窗口，界面正常；
3. **重启虚拟机**：自动登录后 Server 由 XFCE 的自启项拉起（进程在 `/tmp/.mount_*` 下），不做任何操作直接打字 4/4；
4. 把拷出去的插件改坏一个字节再起 `server`：提示「插件已换成这个 AppImage 里的版本」，校验值复原，重启 Fcitx5 后打字照常；
5. `uninstall` 后再重启：没有 Server 进程，自启项与 addon 配置都没了，配置与学习数据还在。

没验：GNOME、KDE 这类桌面的真会话（XDG 自启是通用规范，没逐个验）；Wayland 会话；Qt 应用与浏览器（输入链路与源码安装相同，那边验过）。
