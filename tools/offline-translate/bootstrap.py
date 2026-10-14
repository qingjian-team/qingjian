"""准备独立的 Windows 离线翻译运行时；下载只发生在此安装步骤。"""

import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.request
import zipfile


ASSETS = {
    "python.zip": "https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip",
    "zh_en.argosmodel": "https://argos-net.com/v1/translate-zh_en-1_9.argosmodel",
    "en_zh.argosmodel": "https://argos-net.com/v1/translate-en_zh-1_9.argosmodel",
}
PYTHON_MD5 = "fe8ef205f2e9c3ba44d0cf9954e1abd3"


def download(name: str, url: str, downloads: Path) -> dict:
    target = downloads / name
    if not target.exists():
        temporary = target.with_suffix(target.suffix + ".part")
        request = urllib.request.Request(url, headers={"User-Agent": "Qingjian-offline-setup/1"})
        with urllib.request.urlopen(request, timeout=120) as response, temporary.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
        temporary.replace(target)
    if name == "python.zip":
        digest = hashlib.md5(target.read_bytes(), usedforsecurity=False).hexdigest()
        if digest != PYTHON_MD5:
            raise RuntimeError("Python 压缩包与官方发布摘要不一致")
    with zipfile.ZipFile(target) as archive:
        bad_file = archive.testzip()
        if bad_file is not None:
            raise RuntimeError(f"压缩包校验失败：{name}")
    with target.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    print(f"下载并校验完成：{name} ({target.stat().st_size // 1048576} MB)", flush=True)
    return {"file": name, "url": url, "sha256": digest, "bytes": target.stat().st_size}


def extract(archive_path: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive_path) as archive:
        for item in archive.infolist():
            if not (destination / item.filename).resolve().is_relative_to(destination.resolve()):
                raise RuntimeError("压缩包包含越界路径")
        archive.extractall(destination)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    downloads = root / "downloads"
    downloads.mkdir(parents=True, exist_ok=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        futures = [pool.submit(download, name, url, downloads) for name, url in ASSETS.items()]
        manifest = [future.result() for future in futures]
    (root / "assets.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    extract(downloads / "python.zip", root / "runtime")
    for direction in ("zh_en", "en_zh"):
        extract(downloads / f"{direction}.argosmodel", root / "models" / direction)
    (root / "runtime" / "python312._pth").write_text(
        "python312.zip\n.\nLib/site-packages\n../app\nimport site\n", encoding="utf-8"
    )
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "--only-binary=:all:",
         "--python-version=3.12", "--platform=win_amd64", "--implementation=cp", "--abi=cp312",
         "--target", str(root / "runtime" / "Lib" / "site-packages"),
         "-r", str(Path(__file__).with_name("requirements.txt"))], check=True,
    )
    shutil.copytree(Path(__file__).parent / "qingjian_offline", root / "app" / "qingjian_offline", dirs_exist_ok=True)
    print("离线翻译运行时准备完成。", flush=True)


if __name__ == "__main__":
    main()
