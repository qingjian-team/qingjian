#!/usr/bin/env python3
"""随包词库从词库仓库（qingjian-team/qingjian-dicts）取：按 tools/release/dicts.lock 下载 .qjz、核对 SHA-256、解成 .qj 放进 data/generated/。

    tools/release/dicts-fetch.py            # 下载 + 核对 + 解开；缓存里已有且核对得上的不重下
    tools/release/dicts-fetch.py --verify   # 不联网，只核对缓存的 .qjz 与 data/generated/ 里解开的 .qj
    tools/release/dicts-fetch.py --update   # 按词库仓库 index.json 的 bundled 重写 dicts.lock（换随包词库或版本时用）

Gitee 与 GitHub 两边同时请求，先回来的用（mirror.py）。
锁文件一行一本：`<标签> = <.qjz 的 SHA-256> <装成的文件>`，文件相对 data/generated/，壳按这个名字找。
data-fetch.sh 最后会调它，CI 与各平台打包脚本都不用另外跑。
"""

from __future__ import annotations

import gzip
import json
import shutil
import sys
from pathlib import Path

from mirror import download, fetch_bytes, sha256

ROOT = Path(__file__).resolve().parent.parent.parent
LOCK = ROOT / "tools/release/dicts.lock"
CACHE = ROOT / "target/release-data/dicts"
GENERATED = ROOT / "data/generated"
RELEASE_HOSTS = (
    "https://gitee.com/qingjian-team/qingjian-dicts/releases/download",
    "https://github.com/qingjian-team/qingjian-dicts/releases/download",
)
RAW_HOSTS = (
    "https://gitee.com/qingjian-team/qingjian-dicts/raw/main",
    "https://raw.githubusercontent.com/qingjian-team/qingjian-dicts/main",
)
# 壳认得的 .qj 格式版本；词库仓库升了格式要先改壳再改这里
QJ_FORMAT = 1
# 装成的文件名缺省是 `<id>.qj`，附加词库放 dicts/、形码方案放 schemes/（id 就是方案 id）、辅码码表放 codes/；基础词库沿用壳一直找的 dict.qj，成语词库沿用配置 `[dictionaries] domains` 里的 idioms
NAMES = {"base": "dict.qj", "domain-idioms": "idioms.qj"}


def read_lock() -> list[tuple[str, str, str]]:
    entries = []
    for line in LOCK.read_text("utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        tag, _, rest = line.partition("=")
        sha, installed = rest.split()
        entries.append((tag.strip(), sha, installed))
    return entries


def dict_id(tag: str) -> str:
    return tag.rsplit("-v", 1)[0]


def unpacked_matches(packed: Path, installed: Path) -> bool:
    if not installed.is_file():
        return False
    with gzip.open(packed, "rb") as source, installed.open("rb") as current:
        while True:
            left, right = source.read(1 << 20), current.read(1 << 20)
            if left != right:
                return False
            if not left:
                return True


def fetch(verify_only: bool) -> None:
    CACHE.mkdir(parents=True, exist_ok=True)
    for tag, sha, installed in read_lock():
        name = f"{dict_id(tag)}.qjz"
        packed = CACHE / f"{tag}.qjz"
        target = GENERATED / installed
        if not (packed.is_file() and sha256(packed) == sha):
            if verify_only:
                sys.exit(f"{packed} missing or differs from dicts.lock; run tools/release/dicts-fetch.py")
            print(f"downloading {tag}")
            download([f"{host}/{tag}/{name}" for host in RELEASE_HOSTS], sha, packed)
        if unpacked_matches(packed, target):
            print(f"{installed}  {tag}")
            continue
        if verify_only:
            sys.exit(f"data/generated/{installed} differs from {tag}; run tools/release/dicts-fetch.py")
        target.parent.mkdir(parents=True, exist_ok=True)
        partial = target.with_name(target.name + ".part")
        with gzip.open(packed, "rb") as source, partial.open("wb") as file:
            shutil.copyfileobj(source, file, 1 << 20)
        partial.replace(target)
        print(f"{installed}  {tag} (unpacked)")
    if not verify_only:
        locked = {f"{tag}.qjz" for tag, _, _ in read_lock()}
        for stale in CACHE.glob("*.qjz"):
            if stale.name not in locked:
                stale.unlink()


def update() -> None:
    index = json.loads(fetch_bytes([f"{host}/index.json" for host in RAW_HOSTS]))
    lines = ["# 随包词库：tools/release/dicts-fetch.py --update 按词库仓库 index.json 的 bundled 写，不要手改", "# 标签 = .qjz 的 SHA-256 装成的文件（相对 data/generated/）"]
    for entry in index["bundled"]:
        if entry["qj_format"] != QJ_FORMAT:
            sys.exit(f"{entry['tag']} 是 .qj 格式 {entry['qj_format']}，壳只认 {QJ_FORMAT}")
        manifest = json.loads(fetch_bytes([f"{host}/{entry['tag']}/manifest.json" for host in RELEASE_HOSTS]))
        part = next(part for part in manifest["parts"] if part["name"] == f"{entry['id']}.qjz")
        installed = NAMES.get(entry["id"], f"{entry['id']}.qj")
        if entry["kind"] == "extra":
            installed = f"dicts/{installed}"
        elif entry["kind"] == "scheme":
            installed = f"schemes/{installed}"
        elif entry["kind"] == "codes":
            installed = f"codes/{installed}"
        lines.append(f"{entry['tag']} = {part['sha256']} {installed}")
        print(lines[-1])
    # 按字节写：write_text 在 Windows 上会把换行写成 CRLF，整个锁文件都算改了
    LOCK.write_bytes(("\n".join(lines) + "\n").encode("utf-8"))
    print("dicts.lock 已更新（记得提交）")


def main(args: list[str]) -> int:
    if args == ["--update"]:
        update()
    elif args in ([], ["--verify"]):
        fetch(verify_only=bool(args))
    else:
        sys.exit(__doc__)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
