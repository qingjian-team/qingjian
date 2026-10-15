# Linux 从零到能日用（2026-09-14）

> **这一页记的是走 IBus 那次，不是现行方案。** Linux 后来改成 Rust Server + Fcitx5 默认面板
> （见 [linux-fcitx5.md](linux-fcitx5.md)），IBus 壳与 deb / rpm / flatpak 打包都已从 main 移出，
> 完整代码在合并前的分支末端 `26ece65`（`git show 26ece65:apps/linux/ime/src/ibus/service.rs` 这样取）；
> D-Bus 那层与按键翻译后来移植进了 `apps/linux/ibus`（改成连 Server 的前端，第二个框架）。留着这一页是因为**坑与验收数字仍然有效**：
> 下面五个坑（总线地址 / Shift+数字 / GVariant 嵌套 / 把 `/` 当仓库根 / 锁毒化）与 IBus 面板的限制，
> 换成瘦客户端接 Server 之后一样会碰到。当时的方案取舍文档 `plan/linux_plan.md` 已随路线一起删掉。

## 起因

Phase 5 只剩 Linux 没开工。这一天把它从「一行代码没有」推到「装在开发机上当会话输入法用」：
`apps/linux` 建起来、走 IBus 接上 Core、打成三个 `.deb` 装机、切成系统输入法日常打字。
各模块的实现要点在 [crate-notes.md](crate-notes.md)，这一页只记**过程里踩的坑和验出来的数字**。

## 怎么分层

```text
ibus/        D-Bus 那层：GVariant 序列化、工厂、引擎对象、总线地址
  ↑
dispatch/    按键 → 帧。不认 IBus 也不认 Wayland，所以能脱离输入法框架整段测
  ↑
assembly/    装 Engine
  ↑
Core
```

`dispatch` 这一刀切得值：80 个单元测试里绝大多数在这一层，不用起 daemon、不用 D-Bus。
按键规则是照 macOS 的 `handle_text` / `handle_command` 与 Windows 的 `apply_key` 手工对齐的——
**这是第三份孪生逻辑**，`assembly` 也是第三份。（现行方案把这一份收进了 Linux Server，前端只转事件。）

比 Windows 少了会话分派：IBus 一个引擎实例服务当前焦点，没有「一个 Server 服务多个应用进程」那回事，
所以 Router 只有一份组句状态，形状更接近 macOS 的进程级单例。

## 五个只有真机才暴露的坑

### 1. ibus-daemon 不给引擎进程设 `IBUS_ADDRESS`

本来以为 daemon 拉起引擎时会把总线地址放进环境变量，写完发现引擎起来就退出：找不到地址。
要自己读 `~/.config/ibus/bus/` 下的地址文件。

**而且不能扫目录随便挑**：那个目录里常年躺着别的输入法或上次会话留下的陈旧文件——
开发机上就有 fcitx 留的 `<机器id>-unix-0`，按字典序还排在当前会话的 `<机器id>-unix-wayland-0` 前面，
挑中它连上去一个信号都收不到。改成按 ibus 自己的规矩算文件名（`<机器 id>-<主机>-<显示标识>`），
算出来的不在才退回扫目录，且只认 `IBUS_DAEMON_PID` 还活着的。

### 2. Shift + 数字的 keysym 是 `!` 不是 `1`

缺省的删候选快捷键正是 Shift + 数字。照 macOS 那样按字符认数字，这条快捷键在 Linux 上**整个失效**，
而且失效得很安静——键被当成普通标点吃掉了。得同时用硬件键码按**键位**认（evdev 的 `KEY_1` + X11 的 8 偏移 = 10）。
Windows 用虚拟键码干的是同一件事，只是那边字符与键位天然分开，不容易想到 Linux 也要。

### 3. IBus 的嵌套 GVariant 要包变体

`IBusText` 的签名是 `(sa{sv}sv)`，第四项那个 `v` 得把属性表**包成变体**再塞；
直接塞结构体会内联成 `(sa{sv}av)`。签名错了**不报错**，只是候选窗静默不出现。
`av` 的元素同理。现在三种对象的签名由单元测试钉死，改错了先红的是测试。

### 4. `bundled_root` 把 `/` 当成了仓库根

装完第一次跑 `--check` 就炸出来：随包数据根解析成了 `/`，回落到 148 条的样例词库。
原因是开发布局的回退写成「exe 往上数三层」（`exe → {debug,release} → target → 仓库根`），
装到 `/usr/bin` 之后往上三层正好是 `/`，而这台机器根目录下有个 `/data`，`has_resources` 就认了。

症状是**装好了、能启动、就是一个词都打不出**——最难查的那种。改成先确认 exe 真在
`target/{debug,release}/` 里才认开发布局。这个函数 **Windows 也在用**，所以算顺手修了 Windows。

### 5. `Mutex` 毒化 = 崩一次就再也打不了字

D-Bus 那层要求接口对象 `Send + Sync`，而 `Engine` 因为几个 `RefCell` 缓存不是 `Sync`。
一开始按「另开一条工人线程持有它」设计，写之前测了一下发现 **`Router` 是 `Send` 的**，
套个 `Mutex` 就够，工人线程省了（`dispatch::tests` 里有一条编译期断言钉着）。

但满地 `lock().expect(...)` 埋了个雷：`Mutex` 一旦被 panic 毒化，之后每次 `lock()` 都返回错误。
按「崩溃不丢」那条，崩一次之后输入法该照常服务，而不是永久瘫掉。现在统一走 `Shared::lock`，
毒化了就把状态取回来接着用；按键外面再套一层 `catch_unwind`，拦下后清组句、这个键让给应用。

## 「带内容的 preedit 收不到」——当天归因错了

写了个 D-Bus 客户端当「应用」跑端到端，候选表和上屏都收得到，**唯独带内容的 `UpdatePreeditText` 收不到**。
一路怀疑到序列化、面板、能力位，试了半天：起面板一样收不到，同一个 `IBusText` 在另外两条路上完全正常。
当天的结论是「真实应用的 preedit 由 GTK / Qt 的输入法模块自己渲染，合成客户端本来就验不到」，
测试于是退而断言 `ShowPreeditText`。

**这个归因是错的。** 接本地整句模型时顺手看了一眼 ibus-daemon 自己的日志，里面一直在刷：

```text
GLib-CRITICAL: the GVariant format string '(vubu)' has a type of '(vubu)' but the given value has a type of '(vub)'
IBUS-CRITICAL: bus_engine_proxy_g_signal: assertion 'arg0 != NULL' failed
```

`UpdatePreeditText` 的参数是 `(vubu)`——最后那个 `mode`（`IBUS_ENGINE_PREEDIT_CLEAR` / `COMMIT`）**不能省**。
我们只发了 `(vub)`，daemon 解不出来，`g_return_if_fail` 打一条 CRITICAL 就把整条信号丢掉了。
补上那个参数之后，合成客户端**立刻就收到了带内容的 preedit**，反倒是 `ShowPreeditText` 不再单独来——
`visible=true` 已经把它显示出来，daemon 判定「已经可见」就不再转发。端到端的断言跟着改了过来。

**两条经验**：

1. **错在「我们这边一点错都看不到」**：信号发出去了、zbus 没报错、引擎日志干干净净，
   出错的是收信号那一头，而它的抱怨只写在 ibus-daemon 自己的 stderr 里。
   验 IBus 这类「对面是个 C 程序」的协议，**得去看对面的日志**，这是当天漏掉的一步。
2. 合成客户端能验的东西比当天以为的多。它验不了的是排版与观感，协议对不对它验得了。

## 验收

装机后（GNOME / Wayland，真产品数据：22 万词库 + 42 MB 语言模型 + 24 万词释义表）：

| 项 | 数字 |
|---|---|
| 每键耗时 | 最慢 **3.34 ms**、平均 **857 µs**（标准 10 ms 以内） |
| 整句转换 | `zhonghuarenmingongheguowansui` → 中华人民共和国万岁 |
| 引擎常驻 | 约 104 MB |
| 测试 | 84 个单元测试 + 1 个对着真 ibus-daemon 的端到端（没有 ibus 就跳过，CI 不受影响） |

日常打字验过的：词级、整句、简拼、拼写纠错（`zhonoghua` / `zonghua` 都救回了 中华）、
中英混输（`clone` 原样直通）、学习六张表落盘。

**X11 也能用**：方案 C 压根不碰显示服务器，候选窗的绘制与定位全是 IBus 面板的活。
`GDK_BACKEND=x11` 起的 XWayland 客户端里验过。计划里「只做 Wayland」说的是将来那一步自绘，不是整个 Linux 支持。

## 打包与安装

四个 `.deb`（当时的 `apps/linux/packaging/build-deb.sh`，已随路线删除）：程序 3.5 MB、设置界面 1.4 MB、数据 27 MB、模型 49 MB。
数据与模型分开是因为发行版不收一百多兆的单包，而且它们的更新节奏跟代码不一样；
程序包不依赖这两个：缺语言模型退化成一元词频、缺模型不重排，Core 本来就支持。
**设置界面单独一个包**是后来拆的：输入法本体只链 libc / libm / libgcc（IBus 走纯 Rust 的 zbus），
设置界面是这一套里唯一要 GTK4 的东西，合在一起等于让 KDE / Qt 环境装个输入法就拖进整棵 GTK 树。

装到 `/usr/bin/qingjian-linux` + `/usr/share/qingjian/{data,assets}`，组件 XML 落 `/usr/share/ibus/component/`，
postinst / postrm 调 `ibus write-cache --system` 让 ibus 重扫。

**依赖没走 `dh_shlibdeps`，所以两头都自己算。** 漏了它们的后果不是装不上而是
**装得上、跑不起来**：dpkg 放行，启动才报错。

- `libc6`：取二进制里最高的强符号版本（`pidfd_spawnp` 这类 Rust 标准库的弱引用不算），当前是 2.34，
  也就是 Ubuntu 22.04 / Debian 12 / RHEL 9 起。
- **动态库**：`readelf` 取 NEEDED、`ldd` 找路径、`dpkg -S` 问包名。
  一定要现查而不能手写——接上云联想之后 `libssl` / `libcrypto` 就进了 NEEDED（reqwest 带的 OpenSSL），
  手写的那份不会自己跟上。包名还有 Debian time_t 转换那一层（`libssl3t64` vs `libssl3`），
  所以 `*t64` 的自动写成「或」依赖，两边发行版都装得上。

`qingjian-data` 进 Recommends 同理：缺了它只有几十条样例词库，症状又是「装好了打不出字」。

## 后来补上的

- **本地整句模型**（`dispatch/rescore/`）：当天打了 `qingjian-model` 这个包，但壳根本没接
  `qingjian-neural`，装了也不起作用——mac 的 `host/model.rs` 与 Windows 的 `dispatch/rescore/` 都有，
  Linux 是第三份。接的时候顺带改了主循环：原先是固定一秒的 `tokio::time::interval`，
  而重排要停键 80 ms 请求、20 ms 轮询，所以节拍改成由 `Router::next_tick()` 说了算，按键之后用
  `Notify` 叫醒主循环重算。
- **按键之外怎么重画**：这是接模型时才发现的一个洞。zbus 把信号发出者（`SignalEmitter`）交在方法参数里，
  主循环手上没有，所以原先 `tick()` 返回的那一帧**被丢掉了**——云联想的结果其实也从来没画出去过。
  现在引擎对象拿到焦点时把自己的对象路径记进 `ibus/active.rs`，主循环照着它
  `SignalEmitter::from_parts` 造一个再发。tick 也改成只有帧真变了才回，空转的一秒不往 D-Bus 上发信号。

- **云联想**：与本地模型同一个毛病——`assembly` 从没调过 `with_predictor`，`[predict]` 读进来只在 `--check` 里打印一行。
  接法照 Windows 的 `attach_cloud`（第三份）。`CloudPredictor` 自己起工人线程、线程里自建 current-thread 运行时，
  所以不占主循环那个多线程运行时，也不需要壳这边配合。
  **Linux 特有的一处**：密钥不能指望环境变量——引擎进程是 ibus-daemon 拉起来的，用户在终端 export 的东西它看不到，
  所以启动时用 dotenvy 读一遍 `~/.config/qingjian/.env`（设置界面写密钥也写在那里）。

- **Flatpak**（2026-09-16）：给非 Debian 系发行版的一条路，但**它不能像别的应用那样一装就好**。
  动手前先做了一个实验：把组件 XML 分别放进 `$XDG_DATA_HOME/ibus/component` 与 `XDG_DATA_DIRS` 里的
  `ibus/component`，起一条私有 daemon，**两份都没被发现**，daemon 只认得 `/usr/share/ibus/component/` 里
  系统装的那份。也就是说 **ibus 不按 XDG 数据目录找引擎**，而 Flatpak 暴露文件恰恰只靠 exports 进
  `XDG_DATA_DIRS`——这条把「Flatpak 装完就能用」堵死了，只能由用户在宿主机上放一个
  `<exec>flatpak run …</exec>` 的组件 XML 把两边接起来。
  另外三处是沙箱的常规代价，都能用 `finish-args` 解决：总线 socket 与地址文件在宿主机上（给只读权限，
  而且 `bus_dir` 要往 `$HOME/.config` 回退一次，因为沙箱改掉了 `XDG_CONFIG_HOME`）、
  随包数据落在 `/app/share/qingjian`（`resources` 的系统布局里加了这条）、
  用户数据进 `~/.var/app/<id>/`，与 `.deb` 装的那份不互通。

## 还没做的

- ~~**`.rpm`**~~：当天卡在「没有能验的环境」，2026-09-16 用 docker 解决了——构建与验证都在 fedora
  容器里跑（`build-rpm.sh --verify` 会 `dnf install` 装上再 `--check`）。
  rpm 的自动依赖比 deb 省事得多（自己扫 ELF），但它不分强弱符号，glibc 门槛因此比 deb 严一档。
- **L3 / L4**：自绘要先把 `renderer-spike` 分支合进 main（那份代码不在这台机器上），
  而 L4 的 `input-method-v2` 只有 wlroots 系实现，GNOME / KDE 验不了。
- ~~**设置界面**~~：当天之后补上了，GTK4 七页（当时的 `apps/linux/settings`，已随路线删除）；也仍可直接编辑
  `~/.config/qingjian/config.toml`，热加载即时生效。
- **`[apps]` 按应用配置**：Wayland 拿不到前台应用标识，X11 可行但只覆盖一半平台，不做。
