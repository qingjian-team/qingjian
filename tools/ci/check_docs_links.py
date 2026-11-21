#!/usr/bin/env python3
"""检查仓库内 Markdown 的相对链接是否指向真实存在的文件。

外部 http(s) 链接不在这里查：网络抖动会让 CI 假红，外链失效由定期任务处理。
这里只抓「文档里写了 docs/foo.md，文件已经改名/删掉」这类本地断链。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[2]
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
INLINE_CODE_RE = re.compile(r"`[^`]*`")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "tel:", "data:", "#")


def markdown_files() -> list[Path]:
    files = [ROOT / "README.md", ROOT / "CONTRIBUTING.md", ROOT / "CHANGELOG.md"]
    files += sorted((ROOT / "docs").rglob("*.md"))
    return [path for path in files if path.exists()]


def iter_links(text: str):
    """跳过 ``` 围栏里的示例，只返回 (行号, 链接)。"""
    fenced = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        # 行内代码里的 `[示例](占位)` 是在教写法，不是真链接。
        line = INLINE_CODE_RE.sub("", line)
        for match in LINK_RE.finditer(line):
            yield number, match.group(1).strip()


def check() -> list[str]:
    missing: list[str] = []
    for path in markdown_files():
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, target in iter_links(text):
            if not target or target.startswith(SKIP_PREFIXES):
                continue
            if target.startswith("<") and target.endswith(">"):
                target = target[1:-1]
            local = target.split("#", 1)[0].split("?", 1)[0].strip()
            if not local:
                continue
            # `[名称](链接)` 这类占位没有扩展名也没有路径分隔符，不当文件链接。
            if "." not in local and "/" not in local:
                continue
            resolved = (path.parent / unquote(local)).resolve()
            if not resolved.exists():
                rel = path.relative_to(ROOT).as_posix()
                missing.append(f"{rel}:{number} -> {target}")
    return missing


def main() -> int:
    missing = check()
    if missing:
        print("文档里有指向不存在文件的链接：", file=sys.stderr)
        for item in missing:
            print(f"  - {item}", file=sys.stderr)
        return 1
    print(f"文档链接检查通过：{len(markdown_files())} 个 Markdown 文件")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
