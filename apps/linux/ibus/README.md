# 青简 IBus 前端

与 [`../fcitx5`](../fcitx5) 平级的**第二个前端**：同一个 Server、同一套协议，换一个输入法框架。
引擎在 [`../server`](../server) 那个进程里，前端只做两件事——把框架的按键与焦点事件转成协议事件发过去，
把 Server 回的帧交给框架的面板画。

两个前端可以同时装着、同时连着一个 Server：Server 按「连接代次 + 上下文」隔离会话，互不干扰。
用户实际用哪个由 `$XMODIFIERS` 决定（GNOME 上内建的是 IBus）。

## 现在到哪一步

| 部分 | 状态 |
| --- | --- |
| socket 客户端（连接、握手、会话、事件） | 通了，有测试 |
| 按键翻译（keysym + 修饰键 → `KeyEvent`） | 通了，有测试 |
| D-Bus 层（组件注册、按键、焦点、重置、面板三样） | 通了，**对着真 ibus-daemon 验过** |
| 本地整句模型的重排（组句期间每 80 毫秒 `Poll`，版本号变新才重画） | 通了，**对着真 ibus-daemon 验过**：`houxuanshengcheng` 停键约 0.6 秒后候选声称 → 候选生成，上屏后轮询即停 |
| 回报实际显示了哪些释义（`DisplayAcknowledged`） | 没有：Fcitx5 插件每画一帧都回报，Server 据此给生词记曝光；IBus 这边不回报，生词标记不会随看过的次数消失 |
| 密码框 / 私密输入（`SetContentType`） | 代码有，**没验过**：测试里 IBus 一次都没调它 |
| 按应用设置（`[apps]`） | 没有：IBus 这条路上还没找到拿应用标识的办法，`OpenSession` 的 `app` 报的是 `None` |
| 安装脚本认框架、Server 自启 | 通了：`scripts/install.sh --frontend auto` 装能装的那几支，systemd 用户单元随装 |

怎么验这两支见 [`docs/notes/linux-dual-frontend.md`](../../../docs/notes/linux-dual-frontend.md)，真机上的坑记在 [`docs/notes/linux-bringup.md`](../../../docs/notes/linux-bringup.md)。

## 跑起来

Server 要先起着（它 bind `$XDG_RUNTIME_DIR/qingjian.sock`，`QINGJIAN_SOCKET` 可以换路径）。
不碰 IBus 的自检：

```bash
cargo run -p qingjian-linux-server                  # 一个终端
cargo run -p qingjian-linux-ibus -- --check nihao   # 另一个终端
```

打印 Server 回的候选就说明前端这一侧的协议是对的。打 `"nihao "`（带空格）能看到上屏那一条。

## 对着真 ibus-daemon 跑端到端

[`tests/ibus_engine.rs`](tests/ibus_engine.rs) 是唯一能验证「IBus 那套 GVariant 对象拼对了没有」的办法——
签名错一位不报错，只是候选窗静默不出现。**别用桌面会话那条总线**：那会把当前输入法切成青简、
把测试打的字写进真实学习数据。起一条私有的，Server 也用私有数据目录：

```bash
cargo build -p qingjian-linux-ibus -p qingjian-linux-server

export QJ=$(mktemp -d)
mkdir -p "$QJ/component" "$QJ/config" "$QJ/data" "$QJ/state" "$QJ/cache"
target/debug/qingjian-linux-ibus --ibus-xml "$PWD/target/debug/qingjian-linux-ibus" \
    > "$QJ/component/qingjian.xml"

# socket 路径要短：Unix socket 有 107 字节上限，$(mktemp -d) 套太深就 bind 不了
export QINGJIAN_SOCKET=/run/user/$(id -u)/qingjian-test.sock
env XDG_CONFIG_HOME="$QJ/config" XDG_DATA_HOME="$QJ/data" XDG_STATE_HOME="$QJ/state" \
    target/debug/qingjian-linux-server &
env XDG_CONFIG_HOME="$QJ/config" XDG_DATA_HOME="$QJ/data" XDG_CACHE_HOME="$QJ/cache" \
    IBUS_COMPONENT_PATH="$QJ/component" QINGJIAN_SOCKET="$QINGJIAN_SOCKET" \
    ibus-daemon -a "unix:path=$QJ/bus" -p disable -c disable -t refresh -d

QINGJIAN_IBUS_TEST=1 IBUS_ADDRESS="unix:path=$QJ/bus" \
    cargo test -p qingjian-linux-ibus --test ibus_engine
```

四处不这么写就会浪费一下午：

- **`IBUS_COMPONENT_PATH` 才是决定组件从哪读的**，`XDG_DATA_DIRS` 与 `~/.local/share/ibus/component/` 都不管用。
  机器上装过 `.deb` 时 `/usr/share/ibus/component/qingjian.xml` 与开发版**同名**，路径里排在前面的那份生效，
  排错了的症状是「改了代码重新编译，行为一点没变」。
- **Server 的 socket 目录必须是 0700**，否则 Server 拒绝 bind（它自己的安全检查）。
- **引擎进程的日志要自己引出来**：daemon 拉起它，stdout 进了 daemon。排错时把 `<exec>` 指向一个
  `RUST_LOG=debug ... >> engine.log 2>&1` 的小脚本，事件交错（焦点 / 按键谁先谁后）一眼就看见。
- **收尾按 PID 杀**：`pkill -f 'qingjian-linux-server'` 会匹配到你自己那条命令行，把自己也杀掉。

## 改动注意

[`src/key/mapping.rs`](src/key/mapping.rs) 的 keysym → 虚拟键码表**必须与 Fcitx5 插件的
`../fcitx5/src/key/mapping.cpp` 一字不差**：两个前端喂同一个 Server、同一套 `[shortcut]` 配置，
映射不一致的话同一个键在两个框架下行为就不一样。改一边要改另一边。
