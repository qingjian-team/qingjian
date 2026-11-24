#!/usr/bin/env python3
"""检查 Cargo 构建产物是否会落在 Windows C 盘。

用法：
  python tools/check_build_drive.py
  python tools/check_build_drive.py --root <仓库根>
  python tools/check_build_drive.py --quiet

退出码：0 不在 C 盘；1 在 C 盘；2 配置读不了。
"""

from __future__ import annotations

import argparse
import os
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DRIVE = re.compile(r"^([A-Za-z]):")


def is_c_drive(path: Path) -> bool:
    """只看盘符，不要求路径存在；`C:foo` 和 `C:/foo` 都算。"""
    match = DRIVE.match(str(path))
    return bool(match and match.group(1).upper() == "C")


def resolve(path: str, root: Path) -> Path:
    candidate = Path(path)
    if not candidate.is_absolute():
        candidate = root / candidate
    return candidate.resolve(strict=False)


def target_dir(root: Path, env: dict[str, str]) -> tuple[Path, str]:
    """按 Cargo 的优先级取 target：环境变量 > .cargo/config.toml > 默认 target。"""
    raw = env.get("CARGO_TARGET_DIR", "").strip()
    if raw:
        return resolve(raw, root), "CARGO_TARGET_DIR"

    config = root / ".cargo" / "config.toml"
    if config.is_file():
        try:
            data = tomllib.loads(config.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            raise SystemExit(f"build-drive: 读不了 {config}：{error}") from error
        build = data.get("build", {})
        if isinstance(build, dict):
            raw = build.get("target-dir")
            if isinstance(raw, str) and raw.strip():
                return resolve(raw, root), ".cargo/config.toml"

    return (root / "target").resolve(strict=False), "默认 target"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="仓库根，默认脚本上一级")
    parser.add_argument("--quiet", action="store_true", help="通过时不打印")
    args = parser.parse_args(argv)

    root = args.root.resolve()
    if not root.is_dir():
        print(f"build-drive: 仓库根不存在：{root}", file=sys.stderr)
        return 2

    path, source = target_dir(root, os.environ)
    if is_c_drive(path):
        print(f"build-drive: DENY  {path}  ({source})")
        print("Cargo 会在 C 盘生成 target；C 盘空间不足时 cargo 会 system_error=28，")
        print("还可能把生成文件写坏。请把构建产物指到非 C 盘：")
        print("  PowerShell: $env:CARGO_TARGET_DIR='E:/cargo-target/qingjian'")
        print("  bash:       export CARGO_TARGET_DIR=E:/cargo-target/qingjian")
        print("或在用户级 %USERPROFILE%\\.cargo\\config.toml 写 [build] target-dir。")
        return 1

    if not args.quiet:
        print(f"build-drive: OK    {path}  ({source})")
    return 0


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.exit(main())
