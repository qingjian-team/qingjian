# Linux 发行包：deb / rpm（2026-09-23）

`apps/linux/packaging/build.sh` 一条命令打两种包，给别的 Linux 机器装着试。
这一页记两种包各装了什么、为什么这么分、怎么装，以及几处绕不过去的限制。
flatpak 暂不做（2026-10-23）：输入法在沙箱里要靠宿主机上的登记脚本去写服务与 IBus 组件，等于绕开沙箱，出了问题也难查。
用户级的源码安装（`apps/linux/scripts/install.sh`）另见 [linux-fcitx5.md](linux-fcitx5.md) 与 [linux-dual-frontend.md](linux-dual-frontend.md)。

## 打包

```bash
apps/linux/packaging/build.sh               # 两种都打
apps/linux/packaging/build.sh deb rpm       # 只打给出的几种
QINGJIAN_SAMPLE=1 apps/linux/packaging/build.sh   # 没有产品数据时只带样例词库
```

产物在 `target/linux-pkg/out/`。只要 docker（免 sudo），本机不用装 fcitx5 开发包或 rpmbuild：

| 步骤 | 在哪做 | 为什么 |
|---|---|---|
| Rust 的 Server 与 IBus 前端 | Ubuntu 22.04 容器，编一次两种包共用 | 这两个只依赖 libc / libm / libgcc；在 glibc 2.35 上编，实际门槛是 `GLIBC_2.34`。工具链与 crate 缓存直接挂本机的 `~/.cargo` / `~/.rustup`；依赖树里有 `openssl-sys`，编译要 `libssl-dev`（最终二进制并不链 libssl） |
| fcitx5 插件（deb） | Debian 13 容器 | 插件要对着目标发行版自己的 fcitx5 编；`CMakeLists.txt` 要 5.1.8+，Ubuntu 24.04 只有 5.1.7 |
| fcitx5 插件（rpm） | Fedora 42 容器 | 同上 |
| 随包资源 | 本机，`files.py resources` | 与 `install.sh` 同一套产品数据校验（`tools/release/data.lock`）与布局 |

## 两种包各装了什么

deb 与 rpm 都拆成两个包：

- **`qingjian`**：`/usr/bin/qingjian-linux-server`、`/usr/bin/qingjian-linux-ibus`、`/usr/share/qingjian/resources/`（词库等，
  Server 按「可执行文件上一层的 `share/qingjian/resources`」找到它）、`/usr/share/ibus/component/qingjian.xml`、
  `/usr/lib/systemd/user/qingjian-server.service`。装的时候 `systemctl --global enable` 让每个用户登录后自动起 Server。
- **`qingjian-fcitx5`**：fcitx5 插件与两份描述文件，放 fcitx5 自己的插件目录（Debian 是 `/usr/lib/<多架构>/fcitx5/`，Fedora 是 `/usr/lib64/fcitx5/`）。

IBus 前端没有额外依赖，跟 Server 同包；它的组件 XML 在系统目录，IBus 直接认，不用 `install.sh` 那套 environment.d。

## 装法

deb / rpm 装完 Server 就在已登录用户的会话里起来了（装包脚本对正在跑的用户 systemd 实例直接 restart）；
IBus 要重新登录一次（或 `ibus restart`）才看得到新引擎。

### Debian / Ubuntu（deb）

```bash
sudo apt install ./qingjian_<版本>_amd64.deb                 # IBus 用户
sudo apt install ./qingjian_<版本>_amd64.deb ./qingjian-fcitx5_<版本>_amd64.deb   # fcitx5 用户
```

`qingjian` 只要 `libc6 (>= 2.34)`，Debian 12、Ubuntu 22.04 / 24.04 也装得上（IBus）；
`qingjian-fcitx5` 要 `libfcitx5core7 (>= 5.1.12)`，只有 Debian 13、Ubuntu 25.04 及以后。

### Fedora 42 及以后（rpm）

```bash
sudo dnf install ./qingjian-<版本>.x86_64.rpm
sudo dnf install ./qingjian-<版本>.x86_64.rpm ./qingjian-fcitx5-<版本>.x86_64.rpm
```

KDE Plasma（Wayland）上输入法由 KWin 拉起，默认是 fcitx5：「系统设置 → 键盘 → 虚拟键盘」里选 IBus Wayland
（等于 `kwriteconfig6 --file kwinrc --group Wayland --key InputMethod /usr/share/applications/org.freedesktop.IBus.Panel.Wayland.Gtk3.desktop`），
装了 fcitx5 的再把它的自启关掉，重新登录。

### 装完之后

- IBus：「设置 → 键盘 → 输入源」添加「青简」（GNOME），或 `ibus engine qingjian`。
- fcitx5：`fcitx5-configtool` 里取消「仅显示当前语言」，添加「青简」。
- 配置：`~/.config/qingjian/config.toml`，保存后约一秒内生效（`preedit` 与 `input_log` 两项要重启 Server）。
- 自检（不经过输入法框架，直接问 Server）：`qingjian-linux-ibus --check nihao`。
- Server 状态与日志：`systemctl --user status qingjian-server`、`journalctl --user -u qingjian-server`。

## 实测（2026-09-23，容器里装包 + 起 Server + `--check nihao`）

| 包 | 装在 | 结果 |
|---|---|---|
| deb 两个包 | Debian 13、Ubuntu 26.04 | 装上；插件不缺库；`systemctl --global` 自启登记上、卸载后清掉；自检出「你好」等候选 |
| deb `qingjian` | Ubuntu 24.04、22.04 | 同上（不装插件） |
| rpm 两个包 | Fedora 42、Fedora 最新、openSUSE Tumbleweed | 同上 |

容器里没有图形会话，**真会话里切到青简打字没验**：这一步要在别的机器上装了之后手测，判据见 [linux-dual-frontend.md](linux-dual-frontend.md) 第二节。

### 装包自检：`smoke/run.sh`

```bash
apps/linux/packaging/build.sh deb rpm
apps/linux/packaging/smoke/run.sh                 # 五个发行版都跑；给镜像名只跑那几个
```

把 `target/linux-pkg/out/` 的 deb / rpm 拿到 Debian 13、Ubuntu 26.04、Fedora 42、最新 Fedora、openSUSE Tumbleweed 的干净容器里：
装两个包，看 `systemctl --global` 自启登记、IBus 组件的 `<exec>` 指向的文件在不在、fcitx5 插件缺不缺库，
起 Server 用包里的 `qingjian-linux-ibus --check nihao` 打一串键（走真前端那套客户端代码，不另抄协议），
等本地整句模型加载完，卸载后看自启登记与组件清掉没有；哪一项不对退出码非 0。每个容器要装发行版的依赖，跑一遍五六分钟。
2026-10-14 五个都过（data-v7）：候选「你好 👋 (＾▽＾)ノ 你好好 你好像…」，含章·通变加载并预热。

### fcitx5 的依赖与切换框架

`apt install ./…` / `dnf install ./…` 会把 fcitx5 与它的 GTK / Qt 输入法模块、配置工具一起装上（2026-09-23 在干净的
Debian 13、Ubuntu 26.04、Fedora 42 容器里看过；`dpkg -i` 不补依赖，要再跑一次 `sudo apt -f install`）。
**不会替你把输入法框架从 IBus 切到 fcitx5**，GNOME 默认是 IBus：

- Debian / Ubuntu：`im-config -n fcitx5`，重新登录；
- Fedora：装 `fcitx5-autostart`（`qingjian-fcitx5` 已经 Recommends 它，dnf 缺省会一起装），重新登录。

## 几处限制与坑

- **`systemctl --global enable` 不够**：它只对之后起来的用户 systemd 实例生效，而注销再登录得快时实例根本不重起。
  2026-09-23 在 Ubuntu 24.04 上踩过：10:51 装包、10:52 重新登录，用户实例是 10:46 起的一直没换，Server 一次都没起，
  切到青简打字没有候选。所以 postinst / `%post` 对 `loginctl list-users` 里的每个用户 `systemctl --user -M 用户@ restart`，
  prerm / `%preun` 同样 stop。
- **与用户级安装冲突**：`install.sh` 装在 `~/.local` 的那一份也叫 `qingjian-server.service`，组件名也相同。
  用户目录的单元优先于系统目录的，两份同时在时生效的是用户目录那份；装发行包之前先跑 `apps/linux/scripts/uninstall.sh`。
  2026-09-23 本机踩过：装了 deb 没卸 `~/.local` 那份，IBus 拉起的是 deb 的新前端（协议 7），跑着的却是用户目录单元起的旧 Server
  （协议 6），开会话就被断开、按键全部放行，切到青简打不出字；前端的输出进了 `/dev/null`，`--check` 按 PATH 又先找到 `~/.local/bin`
  的旧前端、看上去是通的。现在两处会提醒：装包时 postinst / `%post` 发现有用户的 `~/.config/systemd/user/qingjian-server.service`
  就打印卸法；Server 碰到版本不一致记 `前端开会话带的协议版本与 Server 不一致` 的警告（`journalctl --user -u qingjian-server`）。
  确认跑的是哪一份：`systemctl --user show qingjian-server -p FragmentPath`。
- **IBus 的注册表缓存不跟 `IBUS_COMPONENT_PATH` 走**：ibus-daemon 把组件缓存在 `~/.cache/ibus/bus/registry`，
  下次启动只看缓存里记着的那几个目录变没变，路径变量换了它不管。2026-09-23 在 Arch（KDE Plasma）上踩过：先登记了一份青简、
  后 `pacman -S ibus`，登记时 `/usr/share/ibus/component` 还不存在，脚本退到了 `/usr/local/...`；会话里的 daemon 按这个路径写了缓存，
  里面没有系统组件目录，之后每次启动都找不到 dconf 配置组件，报 `Can not execute default config program` 退出，
  界面上是「IBus 守护进程 5 秒内无法启动」，重新登录、改对变量都没用，删掉缓存才好。现在 `install.sh` 只在
  `/usr/local` 下确实有 IBus 时才换目录，装与卸都删掉这份缓存。
- **挑 IBus 地址不能只看 socket 文件在不在**：ibus-daemon 退出不删 socket，`~/.cache/ibus` 下常年一堆死的；
  陈旧的 `…-unix-0` 按字典序排在 `…-unix-wayland-0` 前面，指着一个死 socket，前端连它失败，切到青简 `SetGlobalEngine` 超时。
  现在真 connect 一下，连得上才认。
- **`--global` 对登录界面也生效**：gdm 这类系统账户也有自己的用户会话（`loginctl list-users` 里能看到 `gdm`），
  不拦的话 greeter 里也会起一个 Server、白白加载词库与模型。服务单元写 `ConditionUser=!@system` 跳过系统账户（systemd 237+）。
  装包脚本取用户名用 shell 的 `read`，不靠 awk（openSUSE 最小容器里没有 awk）。
