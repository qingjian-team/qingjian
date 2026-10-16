# Linux 双前端怎么验（Fcitx5 与 IBus，2026-09-21）

Linux 上一个 Server、两个前端：`apps/linux/fcitx5`（C++ 插件）与 `apps/linux/ibus`（Rust，D-Bus）。
两支说同一套协议、连同一个 Server，用户实际用哪个由会话的 `$XMODIFIERS` 决定。
这一页只回答一件事：**怎么证明两支都真的能用**。

判据一律写成「看到什么算过」，因为这条链路上**大多数故障是静默的**——
IBus 的 GVariant 签名错一位不报错，只是候选窗不出现；组件 XML 挑错一份，症状是「改了代码行为一点没变」。

分三层：能自动跑的、只能在真会话里手测的、现在根本验不了的。

## 一、自动化：不碰真实会话，可重复

### IBus 那支端到端

完整命令在 [`apps/linux/ibus/README.md`](../../apps/linux/ibus/README.md)「对着真 ibus-daemon 跑端到端」，
要点是私有 daemon + 私有 XDG 目录 + 短 socket 路径（Unix socket 有 107 字节上限）。

```bash
QINGJIAN_IBUS_TEST=1 IBUS_ADDRESS="unix:path=$QJ/bus" \
    cargo test -p qingjian-linux-ibus --test ibus_engine
```

**过**：敲 `nihao` 之后 preedit 出 `ni`、候选表出 `你好`、空格 `CommitText` 出 `你好`。
这是验证「IBus 那套 GVariant 对象拼对了没有」的唯一办法，单元测试只能钉住签名字符串、钉不住 daemon 认不认。
2026-09-21 在开发机上跑通。

### fcitx5 那支的 CTest

`apps/linux/fcitx5/tests/` 九个测试，其中 `real-server` / `privacy-*` / `reentry-*` 会拉起真 Server：

```bash
cmake -S apps/linux/fcitx5 -B target/fcitx5-test -DBUILD_TESTING=ON \
  -DQINGJIAN_SERVER_EXECUTABLE=$PWD/target/debug/qingjian-linux-server
cmake --build target/fcitx5-test --parallel
ctest --test-dir target/fcitx5-test --output-on-failure
```

**`privacy-*` 那几条要特别看**：它们验的正是 IBus 侧现在空着的隐私链路（能力变化时学习与输入日志遵守最新能力）。
两支的隐私行为应当一致，所以 fcitx5 这边全绿能说明 **Server 侧**的逻辑没问题——
但**替代不了 IBus 侧的 `SetContentType` 验证**，那是前端各自的翻译层（见第三节）。

### 安装脚本

装到临时 prefix、私有 XDG，别污染真实环境：

```bash
P=$(mktemp -d)/inst
XDG_DATA_HOME=$P/data XDG_CONFIG_HOME=$P/config apps/linux/scripts/install.sh --prefix $P --sample --debug
XDG_DATA_HOME=$P/data XDG_CONFIG_HOME=$P/config apps/linux/scripts/install.sh --prefix $P --sample --debug
XDG_DATA_HOME=$P/data XDG_CONFIG_HOME=$P/config apps/linux/scripts/uninstall.sh --prefix $P
```

**过**：第一次打印装了哪几支、跳过哪支及原因；第二次不报「目标已存在且不属于本次安装」（幂等）；
卸载后 `find $P -type f | grep -v resources` 为空（bin、IBus 组件 XML、systemd 单元都清掉）。
再加两条负路径：`--frontend fcitx5` 在缺开发包的机器上要报错退出并给出 apt 行，`--frontend xx` 要被拒。

## 二、真会话手测：只有真人能做

先确认现在用的是哪个：`echo $XMODIFIERS`（`@im=ibus` / `@im=fcitx`）。GNOME 会话里内建的是 IBus。

三个观察手段（都验过可用）：

```bash
ss -x | grep qingjian.sock                                          # 每个连着的前端一条 ESTAB
ls -l /proc/$(pgrep -f qingjian-linux-server)/fd | grep -c socket   # 1 = 只有监听，2 = 一个前端，3 = 两个
journalctl --user -u qingjian-server -f                             # 走 systemd 时的 Server 日志
```

Server 目前**不按会话记日志**，所以「前端连上没有」只能这么看。

| # | 做什么 | 过的判据 |
| --- | --- | --- |
| A | 每个框架各切到青简打一次 `nihao` | 候选出来，空格上屏「你好」 |
| B | 组句打一半切窗口（Alt+Tab 走） | 缓冲里的字母**原样上屏**而不是消失 |
| C | 密码框（`sudo` 图形提示 / 浏览器密码框 / 锁屏）里打几个字母 | 不出候选；之后 `~/.local/share/qingjian/` 下的用户词与输入日志里**没有**刚打的内容 |
| D | 两个框架都装上，各切过去用一次 | `ss` 里只有在用的那个前端连着；切另一个之后两边各自组句、互不串词 |
| E | `[general] preedit` 三档各试一次 | `both` / `inline` / `window` 在 IBus 面板下都正常显示 |
| F | `systemctl --user enable --now qingjian-server` 之后重新登录 | 不用手动起，切到青简直接能打 |

B 与 C 是代码里标着「没验过」的两条，优先做：

- **B 验的是 `focus_out` 的 `client_preedit` 报 `false` 对不对**。报 `true` 的话 Server 会把缓冲丢掉不上屏
  （那是留给「应用自己画 preedit」的场合），现在一律报 `false`，沿用 IBus 老版本真机验过的行为。
- **C 验的是 `SetContentType` 那条隐私链路**，见下一节。

## 三、现在验不了的

- **`[apps]` 按应用设置在 IBus 下不生效**：IBus 这条路上拿不到宿主应用标识，`OpenSession` 的 `app` 报 `None`。
  这不是「待验」而是**已知不支持**；要验只能在 fcitx5 那侧（它有 `InputContext::program()`）。
- **IBus 到底会不会调 `SetContentType`**：2026-09-21 那轮端到端里它**一次都没调**（引擎进程的 debug 日志里没有），
  所以密码框映射（purpose 8/9 → `password`，hint 1024 → `sensitive`）目前是纯推演。
  上面 C 就是验它；**要是密码框里候选照出**，说明这条信号在 GNOME 上没来，
  得另找隐私信号的来源（IBus 的 `SetCapabilities` 位，或别的 purpose / hints 传递路径）。

## 四个坑，都踩过

- **IBus 不读用户目录下的组件，同名时看路径顺序**。IBus 只认 `IBUS_COMPONENT_PATH`（没设时只有系统目录），
  `~/.local/share/ibus/component/` 放了也白放——安装脚本第一版就栽在这里，端到端测试手动指定了路径所以没测出来。
  deb 装的旧版与开发版组件名都是 `org.freedesktop.IBus.Qingjian`，两个目录都在路径里时**排在前面的生效**
  （用 `ibus read-cache` 看注册表、用 `ibus engine qingjian` 看实际拉起的进程，两种顺序各试过）。
  安装脚本因此用 environment.d 把用户目录放在最前面；验它时组件路径要从 systemd 的 environment.d 生成器取，别手写。
- **别用真实数据目录**。自动化那层一律私有 XDG：测试打的字会进真实学习数据。
- **`pkill -f 'qingjian…'` 会把自己杀掉**（模式串匹配到自己那条命令行），按 PID 杀。
- **引擎进程的日志进了 daemon**。ibus-daemon 拉起引擎进程，它的 stdout 跟着 daemon 走；
  排错时把组件 XML 的 `<exec>` 指向一个 `RUST_LOG=debug … >> engine.log 2>&1` 的小脚本，
  事件交错（焦点 / 按键谁先谁后）一眼就看见——查「第一个字母被 FocusOut 当原样上屏交出去」那次就是靠它。
