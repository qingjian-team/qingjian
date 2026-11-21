#!/usr/bin/env python3
"""质量趋势门禁：不安全、易崩溃写法的总量只许降，不许涨。

规模门（单文件行数、结构体字段、impl 方法、函数行数）由
`tools/quality/check_code_quality.py` 负责；这里管另一类问题：

- `unsafe` 关键字（块、fn、impl）
- `.unwrap()` / `.expect()`（panic 风险）
- `panic!` / `todo!` / `unimplemented!` / `dbg!`
- `#[allow(...)]`（绕过 lint 的入口）
- `transmute` / `from_utf8_unchecked` / `get_unchecked` / `mem::forget`（关掉具体检查的入口）

存量冻结在 `tools/ci/quality-baseline.toml`：已有文件涨数、或新增文件里出现这些写法，
都会失败。重构让它降下来后跑 `--update` 收紧基线。

    python3 tools/ci/quality_gate.py
    python3 tools/ci/quality_gate.py --update
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE = ROOT / "tools" / "ci" / "quality-baseline.toml"
REPORT = ROOT / "target" / "quality-report.md"
SCAN_ROOTS = ("crates", "apps", "tools")
SKIP_PARTS = {"target", ".git", "tests"}
SKIP_NAMES = {"tests.rs"}

# 行级匹配；注释里的词先被 `line.split("//", 1)[0]` 去掉，块注释不单独处理（基线是冻结值，误报不会漂移）。
PATTERNS = {
    "unsafe": re.compile(r"\bunsafe\b"),
    "unwrap": re.compile(r"\.unwrap\(\)"),
    "expect": re.compile(r"\.expect\("),
    "panic": re.compile(r"\bpanic!"),
    "todo": re.compile(r"\b(?:todo|unimplemented|dbg)!"),
    "allow": re.compile(r"#!?\[allow\("),
    # 比 unsafe 更具体的入口：这几个一旦出现，基本等于“把安全检查关掉了”。
    "unchecked": re.compile(r"\b(?:transmute|from_utf8_unchecked|get_unchecked|mem::forget)\b"),
}


def rust_files() -> list[Path]:
    files: list[Path] = []
    for root_name in SCAN_ROOTS:
        root = ROOT / root_name
        if not root.exists():
            continue
        for path in root.rglob("*.rs"):
            rel = path.relative_to(ROOT)
            if any(part in SKIP_PARTS for part in rel.parts):
                continue
            if path.name in SKIP_NAMES:
                continue
            files.append(path)
    return sorted(files)


def count_file(path: Path) -> dict[str, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    # 测试模块里的 unwrap/unsafe 不算：它们不是随产品发出去的代码。
    text = text.split("#[cfg(test)]", 1)[0]
    counts = {key: 0 for key in PATTERNS}
    for line in text.splitlines():
        code = line.split("//", 1)[0]
        for key, pattern in PATTERNS.items():
            counts[key] += len(pattern.findall(code))
    return {key: value for key, value in counts.items() if value}


def scan() -> tuple[dict[str, int], dict[str, dict[str, int]]]:
    total = {key: 0 for key in PATTERNS}
    files: dict[str, dict[str, int]] = {}
    for path in rust_files():
        counts = count_file(path)
        if not counts:
            continue
        rel = path.relative_to(ROOT).as_posix()
        files[rel] = counts
        for key, value in counts.items():
            total[key] += value
    return total, files


def load_baseline() -> tuple[dict[str, int], dict[str, dict[str, int]]] | None:
    if not BASELINE.exists():
        return None
    data = tomllib.loads(BASELINE.read_text(encoding="utf-8"))
    total = {key: int(value) for key, value in data.get("total", {}).items()}
    files = {
        str(name): {key: int(value) for key, value in counts.items()}
        for name, counts in data.get("files", {}).items()
    }
    return total, files


def write_baseline(total: dict[str, int], files: dict[str, dict[str, int]]) -> None:
    lines = [
        "# 由 tools/ci/quality_gate.py --update 生成。",
        "# 这些数字只许降，不许涨；新增文件带这些写法也必须先登记（即先 review 这份基线）。",
        "",
        "[total]",
    ]
    for key in PATTERNS:
        lines.append(f"{key} = {total.get(key, 0)}")
    for name in sorted(files):
        lines.append("")
        lines.append(f'[files."{name}"]')
        for key in PATTERNS:
            if files[name].get(key):
                lines.append(f"{key} = {files[name][key]}")
    BASELINE.parent.mkdir(parents=True, exist_ok=True)
    BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_report(total: dict[str, int], files: dict[str, dict[str, int]], failures: list[str]) -> None:
    lines = ["# Quality report", "", "| 指标 | 当前 |", "| --- | --- |"]
    for key in PATTERNS:
        lines.append(f"| `{key}` | {total.get(key, 0)} |")
    lines += ["", "## Top 文件", "", "| 文件 | 合计 |", "| --- | --- |"]
    ranked = sorted(files.items(), key=lambda item: sum(item[1].values()), reverse=True)[:15]
    for name, counts in ranked:
        lines.append(f"| `{name}` | {sum(counts.values())} |")
    if failures:
        lines += ["", "## 超标", ""]
        lines += [f"- {failure}" for failure in failures]
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text("\n".join(lines) + "\n", encoding="utf-8")


def compare(
    total: dict[str, int],
    files: dict[str, dict[str, int]],
    baseline: tuple[dict[str, int], dict[str, dict[str, int]]],
) -> list[str]:
    """返回所有越过基线的项；空列表表示通过。"""
    base_total, base_files = baseline
    failures: list[str] = []
    for key, value in total.items():
        limit = base_total.get(key, 0)
        if value > limit:
            failures.append(f"总数 `{key}`：{value} > 基线 {limit}")
    for name, counts in files.items():
        limits = base_files.get(name, {})
        for key, value in counts.items():
            limit = limits.get(key, 0)
            if value > limit:
                failures.append(f"`{name}` 的 `{key}`：{value} > 基线 {limit}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help="把当前值写入基线（只应在收紧后使用）")
    args = parser.parse_args()

    total, files = scan()
    if args.update:
        write_baseline(total, files)
        write_report(total, files, [])
        print(f"基线已更新：{BASELINE.relative_to(ROOT)}")
        return 0

    baseline = load_baseline()
    if baseline is None:
        print(f"缺少基线 {BASELINE.relative_to(ROOT)}；先跑 --update 并提交。", file=sys.stderr)
        return 1

    failures = compare(total, files, baseline)

    write_report(total, files, failures)
    if failures:
        print("质量基线被突破：", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(f"\n明细见 {REPORT.relative_to(ROOT)}；确认是合理新增后跑 --update。", file=sys.stderr)
        return 1

    print(f"质量基线通过：{len(files)} 个文件有登记项")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
