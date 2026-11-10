#!/usr/bin/env python3
"""随包模型从模型仓库（qingjian-team/qingjian-models）取：按 tools/release/models.lock 下载分块、拼回压缩包核对 SHA-256，解出模型文件放到锁文件写的位置。

    tools/release/models-fetch.py            # 下载 + 核对 + 解开；缓存里已有且核对得上的不重下
    tools/release/models-fetch.py --verify   # 不联网，只核对缓存的压缩包与解开的文件

锁文件一行一个：`<标签> = <压缩包的 SHA-256> <装成的文件>`，文件相对仓库根。压缩包里是 `<模型目录>/<文件>` 加 LICENSE，按文件名取那一个。
data-fetch.sh 会调它，CI 与各平台打包脚本都不用另外跑。
"""

from __future__ import annotations

import json
import shutil
import sys
import tarfile
from pathlib import Path

from mirror import download, fetch_bytes, sha256

ROOT = Path(__file__).resolve().parent.parent.parent
LOCK = ROOT / "tools/release/models.lock"
CACHE = ROOT / "target/release-data/models"
RELEASE_HOSTS = (
    "https://gitee.com/qingjian-team/qingjian-models/releases/download",
    "https://github.com/qingjian-team/qingjian-models/releases/download",
)


def read_lock() -> list[tuple[str, str, str]]:
    entries = []
    for line in LOCK.read_text("utf-8").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        tag, _, rest = line.partition("=")
        sha, installed = rest.split()
        entries.append((tag.strip(), sha, installed))
    return entries


def member(archive: tarfile.TarFile, name: str) -> tarfile.TarInfo:
    for info in archive.getmembers():
        if info.isfile() and Path(info.name).name == name:
            return info
    sys.exit(f"{archive.name} has no {name}")


def unpacked_matches(packed: Path, installed: Path) -> bool:
    if not installed.is_file():
        return False
    with tarfile.open(packed) as archive, installed.open("rb") as current:
        source = archive.extractfile(member(archive, installed.name))
        while True:
            left, right = source.read(1 << 20), current.read(1 << 20)
            if left != right:
                return False
            if not left:
                return True


def download_archive(tag: str, expected: str, target: Path) -> None:
    """分块逐个按 manifest.json 核对，拼好再按锁文件核对整个压缩包。"""
    manifest = json.loads(fetch_bytes([f"{host}/{tag}/manifest.json" for host in RELEASE_HOSTS]))
    if manifest["archive"]["sha256"] != expected:
        sys.exit(f"{tag}: manifest.json archive SHA-256 differs from models.lock")
    partial = target.with_name(target.name + ".part")
    with partial.open("wb") as joined:
        for part in manifest["parts"]:
            piece = target.with_name(part["name"])
            download([f"{host}/{tag}/{part['name']}" for host in RELEASE_HOSTS], part["sha256"], piece)
            with piece.open("rb") as file:
                shutil.copyfileobj(file, joined, 1 << 20)
            piece.unlink()
    if sha256(partial) != expected:
        partial.unlink()
        sys.exit(f"{tag}: joined archive SHA-256 differs from models.lock")
    partial.replace(target)


def fetch(verify_only: bool) -> None:
    CACHE.mkdir(parents=True, exist_ok=True)
    for tag, sha, installed in read_lock():
        packed = CACHE / f"{tag}.tar.gz"
        target = ROOT / installed
        if not (packed.is_file() and sha256(packed) == sha):
            if verify_only:
                sys.exit(f"{packed} missing or differs from models.lock; run tools/release/models-fetch.py")
            print(f"downloading {tag}")
            download_archive(tag, sha, packed)
        if unpacked_matches(packed, target):
            print(f"{installed}  {tag}")
            continue
        if verify_only:
            sys.exit(f"{installed} differs from {tag}; run tools/release/models-fetch.py")
        target.parent.mkdir(parents=True, exist_ok=True)
        partial = target.with_name(target.name + ".part")
        with tarfile.open(packed) as archive, partial.open("wb") as file:
            shutil.copyfileobj(archive.extractfile(member(archive, target.name)), file, 1 << 20)
        partial.replace(target)
        print(f"{installed}  {tag} (unpacked)")
    if not verify_only:
        locked = {f"{tag}.tar.gz" for tag, _, _ in read_lock()}
        for stale in CACHE.glob("*.tar.gz"):
            if stale.name not in locked:
                stale.unlink()


def main(args: list[str]) -> int:
    if args in ([], ["--verify"]):
        fetch(verify_only=bool(args))
    else:
        sys.exit(__doc__)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
