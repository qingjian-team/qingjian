#!/usr/bin/env python3
"""界面文案的多语言对照表：各 crate 的 locales/*.toml 按键排成一张网页，一个键一行、每种语言一列，改文案与审译文时横着比。

    tools/i18n/review.py                    # 工作区比 HEAD，只列改过的键（黄底改过、绿底新补，悬停看原文）
    tools/i18n/review.py --base main~3      # 跟任一提交比
    tools/i18n/review.py --all              # 列全部键，哪种语言还没译标红
    tools/i18n/review.py --out 文件.html     # 写到指定位置；缺省写进临时目录并用浏览器打开

简体中文是源语言，键的顺序照它；其余语言缺的键会退到英文再退到中文显示（见 crate-notes 的 bamboo-slip-i18n 一节）。
"""

from __future__ import annotations

import argparse
import html
import subprocess
import sys
import tempfile
import tomllib
import webbrowser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# 有界面文字的 crate；新加了 locales/ 的 crate 补进这里
CATALOGS = [
    "apps/settings/locales",
    "apps/clipboard/locales",
    "crates/qingjian-platform/locales",
    "crates/qingjian-core/locales",
]

SOURCE = "zh-Hans"

# 列的顺序与表头：源语言、英文在前，其余按语言代码
NAMES = {
    "zh-Hans": "简体中文",
    "en": "English",
    "de": "Deutsch",
    "es": "Español",
    "ja": "日本語",
    "vi": "Tiếng Việt",
    "zh-Hant": "繁體中文",
}


def flatten(table: dict, prefix: str = "") -> dict[str, str]:
    out: dict[str, str] = {}
    for key, value in table.items():
        if isinstance(value, dict):
            out.update(flatten(value, f"{prefix}{key}."))
        else:
            out[f"{prefix}{key}"] = str(value)
    return out


def read_worktree(path: Path) -> dict[str, str]:
    with path.open("rb") as file:
        return flatten(tomllib.load(file))


def read_revision(revision: str, path: Path) -> dict[str, str] | None:
    relative = path.relative_to(ROOT).as_posix()
    shown = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{revision}:{relative}"],
        capture_output=True,
    )
    if shown.returncode != 0:
        return None
    return flatten(tomllib.loads(shown.stdout.decode()))


def languages(directory: Path) -> list[str]:
    found = sorted(path.stem for path in directory.glob("*.toml"))
    known = [code for code in NAMES if code in found]
    return known + [code for code in found if code not in NAMES]


def cell(new: str | None, old: str | None, compare: bool) -> str:
    if new is None:
        return '<td class="missing">未译</td>'
    text = html.escape(new)
    if not compare or old == new:
        return f'<td class="same">{text}</td>' if compare else f"<td>{text}</td>"
    if old is None:
        return f'<td class="added">{text}</td>'
    return f'<td class="changed" title="原：{html.escape(old)}">{text}</td>'


def section(directory: str, base: str, show_all: bool) -> tuple[str, int]:
    path = ROOT / directory
    codes = languages(path)
    new = {code: read_worktree(path / f"{code}.toml") for code in codes}
    old = {code: read_revision(base, path / f"{code}.toml") or {} for code in codes}
    source = list(new[SOURCE])
    if show_all:
        keys = source
    else:
        keys = [
            key
            for key in source
            if any(new[code].get(key) != old[code].get(key) for code in codes)
        ]
    deleted = sorted(set(old[SOURCE]) - set(new[SOURCE]))
    if not keys and not deleted:
        return "", 0
    head = "".join(f"<th>{NAMES.get(code, code)}</th>" for code in codes)
    rows = "".join(
        f"<tr><th>{html.escape(key)}</th>"
        + "".join(cell(new[code].get(key), old[code].get(key), not show_all) for code in codes)
        + "</tr>"
        for key in keys
    )
    note = f"<p>删掉的键：{html.escape('、'.join(deleted))}</p>" if deleted else ""
    block = (
        f"<h2>{html.escape(directory)}（{len(keys)} 个键）</h2>{note}"
        f"<table><thead><tr><th>键</th>{head}</tr></thead><tbody>{rows}</tbody></table>"
    )
    return block, len(keys)


STYLE = """
body{font:13px -apple-system,"PingFang SC","Microsoft YaHei",sans-serif;margin:24px;color:#222}
h1{font-size:18px} h2{font-size:15px;margin-top:28px} p{color:#555}
table{border-collapse:collapse;width:100%;table-layout:fixed}
th,td{border:1px solid #ddd;padding:6px 8px;vertical-align:top;text-align:left;word-break:break-word}
thead th{position:sticky;top:0;background:#f4f4f6}
tbody th{font:11px ui-monospace,Menlo,monospace;color:#666;width:160px}
td.changed{background:#fff8e1} td.added{background:#e8f5e9} td.same{color:#888}
td.missing{background:#fdecea;color:#c62828}
.legend span{display:inline-block;padding:2px 8px;margin-right:8px;border:1px solid #ddd}
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", default="HEAD", help="跟哪个提交比（缺省 HEAD）")
    parser.add_argument("--all", action="store_true", help="列全部键，不只改过的")
    parser.add_argument("--out", type=Path, help="写到这个文件；缺省写临时文件并打开")
    args = parser.parse_args()

    blocks = []
    total = 0
    for directory in CATALOGS:
        block, count = section(directory, args.base, args.all)
        blocks.append(block)
        total += count
    if args.all:
        legend = '<span style="background:#fdecea">未译（界面上退到英文再退到中文）</span>'
        title = "界面文案对照：全部键"
    else:
        legend = (
            '<span style="background:#fff8e1">改过（悬停看原文）</span>'
            '<span style="background:#e8f5e9">新补</span><span>没变</span>'
        )
        title = f"界面文案对照：工作区比 {html.escape(args.base)}"
    page = (
        f'<!doctype html><meta charset="utf-8"><title>{title}</title><style>{STYLE}</style>'
        f'<h1>{title}</h1><p class="legend">{legend}共 {total} 个键</p>{"".join(blocks)}'
    )
    out = args.out or Path(tempfile.gettempdir()) / "qingjian-i18n-review.html"
    out.write_text(page, encoding="utf-8")
    print(out)
    if args.out is None:
        webbrowser.open(out.as_uri())
    return 0


if __name__ == "__main__":
    sys.exit(main())
