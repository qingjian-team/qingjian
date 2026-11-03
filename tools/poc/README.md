# 命名管道 PoC

复现 Windows 端 IPC 信任边界上的两个问题。都在**普通用户权限**下可跑，不需要管理员。

管道名：0.3.3 起按当前用户的 SID 派生（`\\.\pipe\qingjian-<12 位摘要>`，见
`crates/qingjian-platform/src/pipe.rs`），两个脚本会自动算（`pipe_name.py`，算法与 Rust 侧一致）；
0.3.2 及更早是固定的 `\\.\pipe\qingjian`，用 `--pipe` 指回去。

## `pipe-crosstalk.py`：会话号可猜，别人的组句能读到

会话号是 DLL 的线程号（`tsf/src/com/mod.rs` 里 `SessionId(GetCurrentThreadId())`），4 对齐、范围小，
而 Server 分发时只看消息里带的号码，不校验这条号码是不是开在这条连接上。于是任何本机进程都能拿别人的
号码去问，Server 就把它正在组句的内容回给你。

```bash
# 先开记事本、切成青简、敲几个拼音（别选词），再跑：
python tools/poc/pipe-crosstalk.py --scan
python tools/poc/pipe-crosstalk.py --scan --steal      # 顺手把别人的缓冲区抢过来（破坏性）
python tools/poc/pipe-crosstalk.py --scan --unprivate  # 顺手把别人的私密态关掉（破坏性）
```

预期输出形如：

```
[命中] 会话 14372：正在输入 'nihao'
        候选：你好 尼豪 你 泥 …
```

修好之后：不在本连接 `OpenSession` 过的会话一律丢弃、不回话，每条探测都超时，脚本打出「没命中」。

## `pipe-squat.py`：管道名能抢注

`qingjian-server.exe` 不是开机自启，由 DLL 第一次打字时拉起来，而它建实例时带
`FILE_FLAG_FIRST_PIPE_INSTANCE`：名字已被占，它就退出。DLL 连管道时也不校验对端。

```bash
python tools/poc/pipe-squat.py           # 占位，等一条连接，打印 OpenSession 里的宿主应用名
python tools/poc/pipe-squat.py --hold 30 # 占着 30 秒，这段时间真 Server 起不来（DoS）
```

抢注之后拿到的 `OpenSession` / `Key` / `Surrounding` 都是明文、无任何鉴别，回包还能让任意文本落进
任意 TSF 应用。脚本只读不写：收到消息就打印，不构造回包，读完释放名字，避免把用户的应用卡在 `open()`。

修好之后：DLL 连上先认对端（`GetNamedPipeServerProcessId` + 打开进程比对映像路径与令牌用户），
认不出来就当连不上——`Key` / `Surrounding` 一条都不会发。脚本这头看到的是「连上了、客户端立刻断开」；
抢注能造成的只剩 DoS（真 Server 抢不到名字），不再有按键外泄。

## 相关代码

- 管道名与两端身份校验：`crates/qingjian-platform/src/pipe.rs`
- 管道与 DACL：`apps/windows/server/src/ipc/pipe.rs`
- 会话分发：`apps/windows/server/src/dispatch/`、`crates/qingjian-platform/src/protocol/client.rs`
- DLL 侧连接：`apps/windows/tsf/src/client/pipe.rs`、`apps/windows/tsf/src/com/mod.rs`
- 对照：Linux 端按连接重映射会话号（`apps/linux/server/src/ipc/connection.rs`），没有这个问题

两个问题都只影响 Windows（macOS 是进程内 IMK，没有这条 IPC）。
