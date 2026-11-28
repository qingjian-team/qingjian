#!/usr/bin/env python3
"""上帝对象门禁：单个类型的代码行数不得超过 800。

统计对象是「一个类型」而不是一个文件：`struct` / `enum` / `union` / `trait` 的定义，
加上它在同一个 crate 里的全部 `impl` 块（inherent 与 trait impl 都算），跨文件、跨模块累计。
这么算是因为上帝对象最常见的形态就是把字段留在一个文件、方法摊到另外五个文件里，
按文件查行数看不出来。

口径：
- 只算代码行：空行、纯注释行不计；块注释与字符串字面量不参与括号配对。
- 包含 `#[cfg(test)]` 与 `tests/` 下的代码：测试里给同一个类型写的 impl 也算进这个类型。
- 类型按「crate 根 + 类型名」聚合，例如 `crates/qingjian-core::Engine`。

存量超过 800 行的类型冻结在 `tools/quality/god_object_allowlist.toml`，只允许维持或变小；
新增超限类型、或已登记类型变大，CI 失败。重构让某个类型降到 800 行以内后，跑 `--update` 清掉豁免。

用法：
    python3 tools/quality/check_god_objects.py
    python3 tools/quality/check_god_objects.py --update
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

ALLOWLIST = Path("tools/quality/god_object_allowlist.toml")
SKIP_DIRS = {"target", ".git"}
LIMIT_KEY = "god_object_lines"

TYPE_RE = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?"
    r"(struct|enum|union|trait)[ \t]+(\w+)"
)
IMPL_RE = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:unsafe[ \t]+)?(?:default[ \t]+)?"
    r"impl\b([^\n{]*)\{"
)


@dataclass
class Entry:
    """一个类型的累计代码行数与出处。"""

    key: str
    lines: int = 0
    refs: list[tuple[str, int]] = field(default_factory=list)

    def add(self, path: str, line: int, count: int) -> None:
        self.lines += count
        self.refs.append((path, line))


def skip_noncode(text: str) -> list[bool]:
    """标记每个字符是否为代码（注释与字符串 / 字符字面量记为 False）。"""
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
    """返回与 open_idx 处 `{` 配对的 `}` 下标；配不上时返回最后位置。"""
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


def strip_generics(s: str) -> str:
    out: list[str] = []
    depth = 0
    for c in s:
        if c == "<":
            depth += 1
        elif c == ">":
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(c)
    return "".join(out).strip()


def impl_target(header: str) -> str | None:
    """从 impl 头部（`{` 之前）解析出被 impl 的类型名。"""
    h = re.sub(r"\bwhere\b.*$", "", header).strip()
    while h.startswith("<"):
        depth = 0
        for i, c in enumerate(h):
            if c == "<":
                depth += 1
            elif c == ">":
                depth -= 1
                if depth == 0:
                    h = h[i + 1:].strip()
                    break
        else:
            return None
    if " for " in h:
        h = h.rsplit(" for ", 1)[1].strip()
    h = strip_generics(h)
    h = h.lstrip("&").strip()
    if h.startswith("mut "):
        h = h[4:].strip()
    h = h.split("::")[-1].strip()
    m = re.match(r"([A-Za-z_]\w*)", h)
    return m.group(1) if m else None


def crate_root(path: Path, root: Path) -> str:
    parts = path.relative_to(root).parts
    if len(parts) >= 2 and parts[0] in ("crates", "apps", "tools"):
        return f"{parts[0]}/{parts[1]}"
    return "."


def code_line_flags(text: str, code: list[bool]) -> list[bool]:
    """逐行标记：这一行是否有代码（非空且不全是注释 / 字符串内容）。"""
    flags: list[bool] = []
    start = 0
    for line in text.splitlines(keepends=True):
        end = start + len(line)
        flags.append(any(code[k] and text[k] not in " \t\r\n" for k in range(start, end)))
        start = end
    return flags


def line_index(text: str) -> list[int]:
    """返回每个字符所在的行号（从 0 开始）。"""
    index: list[int] = []
    line = 0
    for c in text:
        index.append(line)
        if c == "\n":
            line += 1
    index.append(line)
    return index


def scan(root: Path) -> dict[str, Entry]:
    entries: dict[str, Entry] = {}

    for path in sorted(root.rglob("*.rs")):
        rel = path.relative_to(root)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        code = skip_noncode(text)
        flags = code_line_flags(text, code)
        row = line_index(text)
        key_prefix = crate_root(path, root)
        rel_posix = rel.as_posix()

        def add(name: str, start_idx: int, end_idx: int) -> None:
            key = f"{key_prefix}::{name}"
            entry = entries.setdefault(key, Entry(key))
            first_line = row[start_idx] if start_idx < len(row) else 0
            last_line = row[end_idx] if end_idx < len(row) else first_line
            count = sum(1 for i in range(first_line, last_line + 1) if flags[i])
            entry.add(rel_posix, first_line + 1, count)

        for m in TYPE_RE.finditer(text):
            if m.start() >= len(code) or not code[m.start()]:
                continue
            name = m.group(2)
            # 找到类型体：先遇到 `;` 就是单元 / 元组结构体，只算一行。
            brace = text.find("{", m.end())
            semi = text.find(";", m.end())
            if semi != -1 and (brace == -1 or semi < brace):
                add(name, m.start(), semi)
                continue
            if brace == -1:
                continue
            close = match_brace(text, code, brace)
            add(name, m.start(), close)

        for m in IMPL_RE.finditer(text):
            if m.start() >= len(code) or not code[m.start()]:
                continue
            name = impl_target(m.group(1))
            if not name:
                continue
            brace = text.find("{", m.end() - 1)
            if brace == -1:
                continue
            close = match_brace(text, code, brace)
            add(name, m.start(), close)

    return entries


def load_limits_and_allow(path: Path) -> tuple[int, dict[str, int]]:
    if not path.exists():
        return 800, {}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    limits = data.get("limits", {})
    limit = int(limits.get(LIMIT_KEY, 800))
    allow = {str(k): int(v) for k, v in data.get("allow", {}).items()}
    return limit, allow


def check(entries: dict[str, Entry], limit: int, allow: dict[str, int]) -> tuple[list[str], list[str]]:
    """返回 (违规, 提示)。违规让 CI 红；提示只打印。"""
    violations: list[str] = []
    notes: list[str] = []

    for key, entry in sorted(entries.items(), key=lambda kv: kv[1].lines, reverse=True):
        if entry.lines <= limit:
            continue
        allowed = allow.get(key)
        where = ",".join(f"{p}:{n}" for p, n in entry.refs[:3])
        if allowed is None:
            violations.append(
                f"VIOLATION type={key} lines={entry.lines} limit={limit} "
                f"reason=new evidence={where}"
            )
        elif entry.lines > allowed:
            violations.append(
                f"VIOLATION type={key} lines={entry.lines} allowed={allowed} "
                f"reason=grew evidence={where}"
            )

    for key, allowed in allow.items():
        entry = entries.get(key)
        if entry is None:
            notes.append(f"note: allowlist 里的 {key} 没找到，跑 --update 清掉")
        elif entry.lines <= limit:
            notes.append(f"note: {key} 已降到 {entry.lines} 行（≤{limit}），跑 --update 清掉")

    return violations, notes


def write_allowlist(path: Path, limit: int, entries: dict[str, Entry]) -> int:
    rows = sorted(
        (key, entry.lines)
        for key, entry in entries.items()
        if entry.lines > limit
    )
    lines = [
        "# 上帝对象存量豁免。只允许维持登记时的数值；变大（或新增超 800 行的类型）CI 会失败。",
        "# 重构让某个类型降到 800 行以内后，跑 `python3 tools/quality/check_god_objects.py --update` 清理。",
        "",
        "[limits]",
        f"{LIMIT_KEY} = {limit}",
        "",
        "[allow]",
    ]
    for key, value in rows:
        lines.append(f'"{key}" = {value}')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return len(rows)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="单个类型的代码行数不得超过 800")
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--allowlist", type=Path, default=ALLOWLIST)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args(argv)

    limit, allow = load_limits_and_allow(args.allowlist)
    if args.limit is not None:
        limit = args.limit

    entries = scan(args.root)
    if args.update:
        count = write_allowlist(args.allowlist, limit, entries)
        print(f"已写入 {args.allowlist}：{count} 个超过 {limit} 行的类型")
        return 0

    violations, notes = check(entries, limit, allow)
    over = sum(1 for entry in entries.values() if entry.lines > limit)
    print(
        f"扫描 {len(entries)} 个类型；超过 {limit} 行的 {over} 个"
        f"（已登记 {len(allow)} 个）"
    )
    for note in notes:
        print(note)
    for violation in violations:
        print(violation)

    if violations:
        print("上帝对象门禁失败。", file=sys.stderr)
        return 1
    print("上帝对象门禁通过。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
