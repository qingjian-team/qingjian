#!/usr/bin/env python3
"""代码规模门禁：单文件行数、结构体字段数、impl 方法数、函数行数。

存量超标项登记在 allowlist.toml 里冻结，只允许维持现有数值；一旦变大（或新增超标项）即失败。
重构让某项降到阈值以内后，跑 `--update` 清掉对应豁免即可。

用法：
    python3 tools/quality/check_code_quality.py
    python3 tools/quality/check_code_quality.py --update
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

ALLOWLIST = Path("tools/quality/allowlist.toml")

SKIP_DIRS = {"target", ".git"}

IMPL_RE = re.compile(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?impl(?:<[^>{]*>)?[ \t]+([^\n{]*)\{")
STRUCT_RE = re.compile(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?struct[ \t]+(\w+)(?:<[^>{]*>)?[ \t]*\{")
FN_RE = re.compile(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:async[ \t]+)?(?:unsafe[ \t]+)?(?:const[ \t]+)?fn[ \t]+(\w+)")
FIELD_RE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?[A-Za-z_]\w*\s*:\s*")
TOP_FN_RE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?fn\s+\w+")


def skip_noncode(text: str) -> list[bool]:
    """标记每个字符是否为代码（略过注释与字符串字面量），用于括号配对。"""
    n = len(text)
    code = [True] * n
    i = 0
    while i < n:
        c = text[i]
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            j = text.find("\n", i)
            j = n if j == -1 else j
            for k in range(i, j):
                code[k] = False
            i = j
            continue
        if c == "/" and i + 1 < n and text[i + 1] == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            for k in range(i, min(j, n)):
                code[k] = False
            i = j
            continue
        if c == "r" and i + 1 < n and text[i + 1] in "\"'":
            hashes, j = 0, i + 1
            while j < n and text[j] == "#":
                hashes, j = hashes + 1, j + 1
            if j < n and text[j] == '"':
                close = '"' + "#" * hashes
                end = text.find(close, j + 1)
                end = n if end == -1 else end + len(close)
                for k in range(i, end):
                    code[k] = False
                i = end
                continue
        if c == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                elif text[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            for k in range(i, min(j, n)):
                code[k] = False
            i = j
            continue
        if c == "'":
            if i + 1 < n and text[i + 1] == "\\":
                j = i + 2
                while j < n and text[j] != "'":
                    j += 1
                if j < n:
                    for k in range(i, j + 1):
                        code[k] = False
                    i = j + 1
                    continue
            elif i + 2 < n and text[i + 2] == "'":
                for k in range(i, i + 3):
                    code[k] = False
                i += 3
                continue
        i += 1
    return code


def match_brace(text: str, code: list[bool], open_idx: int) -> int:
    depth = 0
    for i in range(open_idx, len(text)):
        if not code[i]:
            continue
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    return len(text) - 1


def count_top_items(body: str, item_re: re.Pattern[str]) -> int:
    """数块内深度 0 的条目（结构体字段或 impl 直接方法）。"""
    depth = 0
    n = 0
    for line in body.splitlines():
        s = line.strip()
        if depth == 0 and s and not s.startswith(("//", "/*", "*", "#")) and item_re.match(s):
            n += 1
        depth += s.count("{") - s.count("}")
    return n


def scan(root: Path) -> tuple[dict[str, int], dict[str, int], dict[str, int], dict[str, int]]:
    files: dict[str, int] = {}
    structs: dict[str, int] = {}
    impls: dict[str, int] = {}
    fns: dict[str, int] = {}

    for path in root.rglob("*.rs"):
        rel = path.relative_to(root).as_posix()
        if any(part in SKIP_DIRS for part in path.relative_to(root).parts):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        code = skip_noncode(text)
        files[f"file|{rel}"] = len(text.splitlines())

        for m in STRUCT_RE.finditer(text):
            if not code[m.start()]:
                continue
            brace = m.end() - 1
            body = text[brace + 1:match_brace(text, code, brace)]
            key = f"struct|{rel}|{m.group(1)}"
            structs[key] = max(structs.get(key, 0), count_top_items(body, FIELD_RE))

        for m in IMPL_RE.finditer(text):
            if not code[m.start()]:
                continue
            brace = m.end() - 1
            body = text[brace + 1:match_brace(text, code, brace)]
            key = f"impl|{rel}|{m.group(1).strip()}"
            impls[key] = max(impls.get(key, 0), count_top_items(body, TOP_FN_RE))

        for m in FN_RE.finditer(text):
            if not code[m.start()]:
                continue
            brace = text.find("{", m.end())
            semi = text.find(";", m.end())
            if brace == -1 or (semi != -1 and semi < brace):
                continue
            end = match_brace(text, code, brace)
            key = f"fn|{rel}|{m.group(1)}"
            fns[key] = max(fns.get(key, 0), text[m.start():end].count("\n") + 1)

    return files, structs, impls, fns


def load_allowlist(root: Path) -> tuple[dict[str, int], dict[str, int]]:
    path = root / ALLOWLIST
    if not path.exists():
        return {}, {}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    limits = {k: int(v) for k, v in data.get("limits", {}).items()}
    allow = {k: int(v) for k, v in data.get("allow", {}).items()}
    return limits, allow


def render_allowlist(limits: dict[str, int], allow: dict[str, int]) -> str:
    lines = [
        "# 代码规模存量豁免。只允许维持登记时的数值，变大（或新增超标项）CI 会失败。",
        "# 阈值见 [limits]；重构让某项降到阈值内后，跑 `python3 tools/quality/check_code_quality.py --update` 清理。",
        "",
        "[limits]",
    ]
    lines += [f"{k} = {v}" for k, v in limits.items()]
    lines += ["", "[allow]"]
    for k in sorted(allow):
        lines.append(f'"{k}" = {allow[k]}')
    return "\n".join(lines) + "\n"


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true", help="按当前代码重写 allowlist")
    args = parser.parse_args()

    limits, allow = load_allowlist(root)
    defaults = {"file_lines": 800, "struct_fields": 40, "impl_methods": 40, "fn_lines": 150}
    limits = {**defaults, **limits}

    files, structs, impls, fns = scan(root)
    checks = [
        ("file_lines", "文件行数", files, limits["file_lines"]),
        ("struct_fields", "结构体字段", structs, limits["struct_fields"]),
        ("impl_methods", "impl 方法", impls, limits["impl_methods"]),
        ("fn_lines", "函数行数", fns, limits["fn_lines"]),
    ]

    if args.update:
        allow = {}
        for _, _, items, limit in checks:
            allow.update({k: v for k, v in items.items() if v > limit})
        (root / ALLOWLIST).write_text(render_allowlist(limits, allow), encoding="utf-8")
        print(f"已写入 {ALLOWLIST}，豁免 {len(allow)} 项")
        for _, label, items, limit in checks:
            print(f"  {label} > {limit}: {sum(1 for v in items.values() if v > limit)}")
        return 0

    failed = False
    for _, label, items, limit in checks:
        bad = []
        stale = []
        for key, actual in items.items():
            if actual > limit:
                allowed = allow.get(key)
                if allowed is None:
                    bad.append((key, actual, None))
                elif actual > allowed:
                    bad.append((key, actual, allowed))
            elif key in allow:
                stale.append(key)
        print(f"{label}: {len(items)} 项，超阈值 {sum(1 for v in items.values() if v > limit)}，违规 {len(bad)}")
        for key, actual, allowed in sorted(bad, key=lambda x: -x[1]):
            where = f"（基线 {allowed}）" if allowed is not None else ""
            print(f"  ✗ {key}: {actual}{where}")
        if stale:
            print(f"  提示：{len(stale)} 项已降到阈值内，可跑 --update 清理")
        failed |= bool(bad)

    if failed:
        print("\n代码规模门禁失败：拆分文件 / 结构体 / impl 或把长函数拆小；存量只能维持不能变大。")
        return 1
    print("\n代码规模门禁通过。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
