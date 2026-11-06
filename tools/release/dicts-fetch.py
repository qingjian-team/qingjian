#!/usr/bin/env python3
"""随包词库从词库仓库（qingjian-team/qingjian-dicts）取：按 tools/release/dicts.lock 下载 .qjz、核对 SHA-256、解成 .qj 放进 data/generated/。

    tools/release/dicts-fetch.py            # 下载 + 核对 + 解开；缓存里已有且核对得上的不重下
    tools/release/dicts-fetch.py --verify   # 不联网，只核对缓存的 .qjz 与 data/generated/ 里解开的 .qj
    tools/release/dicts-fetch.py --update   # 按词库仓库 index.json 的 bundled 重写 dicts.lock（换随包词库或版本时用）

Gitee 与 GitHub 上的附件一模一样，两边同时请求，先回来的用；那边下坏了或核对不上再换另一边。
锁文件一行一本：`<标签> = <.qjz 的 SHA-256> <装成的文件>`，文件相对 data/generated/，壳按这个名字找。
data-fetch.sh 最后会调它，CI 与各平台打包脚本都不用另外跑。
"""

from __future__ import annotations

import gzip
import hashlib
import json
import shutil
import sys
import urllib.parse
import urllib.request
from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from pathlib import Path

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
# 装成的文件名缺省是 `<id>.qj`，附加词库放 dicts/；基础词库沿用壳一直找的 dict.qj，成语词库沿用配置 `[dictionaries] domains` 里的 idioms
NAMES = {"base": "dict.qj", "domain-idioms": "idioms.qj"}
# 单次读写的超时：连不上的那边（GitHub 在大陆常这样）拖着线程，脚本退出前要等它超时
TIMEOUT = 20


def open_url(url: str):
    request = urllib.request.Request(url, headers={"User-Agent": "qingjian-build"})
    return urllib.request.urlopen(request, timeout=TIMEOUT)


def host(response) -> str:
    """跳转后的下载地址带一长串签名，日志里只写主机。"""
    return urllib.parse.urlsplit(response.url).netloc


def race(urls: list[str]):
    """同时打开几个地址，按响应先后逐个交出；调用方用上一个就不再往下取，没用上的都关掉。"""
    pool = ThreadPoolExecutor(len(urls))
    pending: set[Future] = {pool.submit(open_url, url) for url in urls}
    ready: list[Future] = []
    try:
        while pending or ready:
            if not ready:
                done, pending = wait(pending, return_when=FIRST_COMPLETED)
                ready.extend(done)
            future = ready.pop(0)
            try:
                yield future.result()
            except OSError as error:
                print(f"  {error}", file=sys.stderr)
    finally:
        for future in [*ready, *pending]:
            future.add_done_callback(lambda f: f.exception() is None and f.result().close())
        pool.shutdown(wait=False)


def fetch_bytes(urls: list[str]) -> bytes:
    for response in race(urls):
        with response:
            try:
                return response.read()
            except OSError as error:
                print(f"  {host(response)}：{error}", file=sys.stderr)
    sys.exit(f"都下不下来：{'、'.join(urls)}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def download(tag: str, name: str, expected: str, target: Path) -> None:
    """先回来的那边下；断了或核对不上换下一边，都不行就停。"""
    partial = target.with_name(target.name + ".part")
    for response in race([f"{host}/{tag}/{name}" for host in RELEASE_HOSTS]):
        with response:
            try:
                with partial.open("wb") as file:
                    shutil.copyfileobj(response, file, 1 << 20)
            except OSError as error:
                print(f"  {host(response)}：{error}", file=sys.stderr)
                continue
        if sha256(partial) == expected:
            partial.replace(target)
            print(f"  {host(response)}")
            return
        print(f"  {host(response)} 下的与 dicts.lock 对不上", file=sys.stderr)
    partial.unlink(missing_ok=True)
    sys.exit(f"{tag} 的 {name} 两边都没下到核对得上的")


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
                sys.exit(f"{packed} 没有或与 dicts.lock 不符，先跑 tools/release/dicts-fetch.py")
            print(f"下载 {tag}")
            download(tag, name, sha, packed)
        if unpacked_matches(packed, target):
            print(f"{installed}  {tag}")
            continue
        if verify_only:
            sys.exit(f"data/generated/{installed} 与 {tag} 不符，先跑 tools/release/dicts-fetch.py")
        target.parent.mkdir(parents=True, exist_ok=True)
        partial = target.with_name(target.name + ".part")
        with gzip.open(packed, "rb") as source, partial.open("wb") as file:
            shutil.copyfileobj(source, file, 1 << 20)
        partial.replace(target)
        print(f"{installed}  {tag}（已解开）")
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
        lines.append(f"{entry['tag']} = {part['sha256']} {installed}")
        print(lines[-1])
    LOCK.write_text("\n".join(lines) + "\n", "utf-8")
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
