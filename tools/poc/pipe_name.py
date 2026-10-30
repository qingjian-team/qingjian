"""当前用户的命名管道名。

与 `crates/qingjian-platform/src/pipe.rs` 的 `name()` 同一套算法：`\\\\.\\pipe\\qingjian-<用户 SID 的
FNV-1a 摘要>`。0.3.2 及更早的名字是固定的 `\\\\.\\pipe\\qingjian`，脚本里 `--pipe` 可以手动指回去。
"""

from __future__ import annotations

import subprocess

PREFIX = r"\\.\pipe\qingjian-"

# FNV-1a 64 的偏移基与质数
OFFSET = 0xCBF29CE484222325
PRIME = 0x100000001B3
MASK = 0xFFFFFFFFFFFFFFFF


def user_sid() -> str:
    """当前登录用户的 SID（`S-1-5-21-…`）。`whoami /user` 的输出格式：`"host\\user","S-1-5-…"`。

    输出按系统 ANSI 代码页走（中文 Windows 不是 UTF-8），SID 本身是 ASCII，解坏了也不影响。
    """
    output = subprocess.run(
        ["whoami", "/user", "/fo", "csv", "/nh"],
        capture_output=True,
        check=True,
    ).stdout
    return output.decode("mbcs", "replace").strip().rsplit(",", 1)[-1].strip().strip('"')


def digest(text: str) -> int:
    value = OFFSET
    for byte in text.encode("utf-8"):
        value ^= byte
        value = (value * PRIME) & MASK
    return value


def default_pipe_name() -> str:
    return f"{PREFIX}{digest(user_sid()):012x}"


if __name__ == "__main__":
    print(default_pipe_name())
