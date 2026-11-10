"""dicts-fetch.py 与 models-fetch.py 共用的下载：Gitee 与 GitHub 上的附件一模一样，几个地址同时请求，先回来的用；断了或核对不上再换下一边。"""

from __future__ import annotations

import hashlib
import shutil
import sys
import urllib.parse
import urllib.request
from concurrent.futures import FIRST_COMPLETED, Future, ThreadPoolExecutor, wait
from pathlib import Path

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
                print(f"  {host(response)}: {error}", file=sys.stderr)
    sys.exit(f"all mirrors failed: {', '.join(urls)}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def download(urls: list[str], expected: str, target: Path) -> None:
    """先回来的那边下；断了或 SHA-256 对不上换下一边，都不行就停。"""
    partial = target.with_name(target.name + ".part")
    for response in race(urls):
        with response:
            try:
                with partial.open("wb") as file:
                    shutil.copyfileobj(response, file, 1 << 20)
            except OSError as error:
                print(f"  {host(response)}: {error}", file=sys.stderr)
                continue
        if sha256(partial) == expected:
            partial.replace(target)
            print(f"  {host(response)}")
            return
        print(f"  {host(response)}: SHA-256 mismatch", file=sys.stderr)
    partial.unlink(missing_ok=True)
    sys.exit(f"no mirror served a matching {target.name}")
