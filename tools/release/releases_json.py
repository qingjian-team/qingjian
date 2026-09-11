#!/usr/bin/env python3
"""从 CHANGELOG.md 与 GitHub Releases 生成官网下载页用的 releases.json。

  releases_json.py --changelog CHANGELOG.md --repo owner/name --api-json releases.api.json --meta-dir meta/ --out releases.json
  releases_json.py --changelog CHANGELOG.md --notes-for 0.1.0      # 只打印某版本的更新日志（给 gh release create 用）

CHANGELOG.md 的格式（一个版本一节，日期与发布渠道写在标题里，正文是一行一条的列表）：

  ## 0.1.0 · 2026-09-07 · beta
  - 整句输入……
  - 候选旁有词性和译词……

渠道只能是 alpha / beta / rc / stable。api-json 是 `gh api repos/<repo>/releases --paginate` 的输出（数组）。
只取 tag 形如 [平台-]v<版本> 的非草稿发布（平台前缀可选：macos- / windows- / linux-，各平台壳版本号独立），安装包按文件名识别平台与架构，其他附件（SHA256SUMS、build-info.json、releases.json 本身）不列。
meta-dir 下每个 tag 一个目录，放那次发布的 SHA256SUMS（每个包的 sha256）与 build-info.json（提交哈希、构建时间、工具链），
没有就对应字段留空。输出结构与官网 src/lib/releases.ts 的 Release / Asset 类型对应：

  {
    "generated": "2026-09-07T12:00:00Z",
    "repository": "owner/name",
    "latest": "0.1.0",
    "releases": [
      {
        "version": "0.1.0", "date": "2026-09-07", "channel": "beta", "notes": ["…"],
        "commit": "869ad00…（40 位）", "built_at": "2026-09-07T08:38:12Z", "toolchain": "rustc 1.96.0 (…)",
        "assets": [
          {"platform": "macos", "arch": "Apple Silicon", "file": "Qingjian-0.1.0-arm64.pkg",
           "url": "https://github.com/…/releases/download/v0.1.0/Qingjian-0.1.0-arm64.pkg", "size": 123456, "sha256": "…"}
        ]
      }
    ]
  }
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

HEADING = re.compile(r"^##\s+(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)\s*·\s*(?P<date>\d{4}-\d{2}-\d{2})\s*·\s*(?P<channel>\S+)\s*$")
TAG = re.compile(r"^(?:(?:macos|windows|linux)-)?v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)$")
CHANNELS = ("alpha", "beta", "rc", "stable")

# 安装包文件名 → 平台与架构说明；不匹配的附件不进列表
ASSET_KINDS = [
    (re.compile(r"^Qingjian-.+-arm64\.pkg$"), "macos", "Apple Silicon"),
    (re.compile(r"^Qingjian-.+-x86_64\.pkg$"), "macos", "Intel"),
    (re.compile(r"^Qingjian-.+-Setup\.exe$"), "windows", "x64"),
]


def parse_changelog(path: Path) -> dict[str, dict]:
    """按版本号收集 {version: {date, channel, notes}}。"""
    entries: dict[str, dict] = {}
    current: dict | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip()
        if line.startswith("## "):
            m = HEADING.match(line)
            if not m:
                sys.exit(f"CHANGELOG 标题格式不对，应为「## 版本 · YYYY-MM-DD · 渠道」：{line}")
            if m["channel"] not in CHANNELS:
                sys.exit(f"渠道只能是 {' / '.join(CHANNELS)}：{line}")
            current = {"date": m["date"], "channel": m["channel"], "notes": []}
            entries[m["version"]] = current
        elif current is not None and line.startswith("- "):
            current["notes"].append(line[2:].strip())
    return entries


def semver_key(version: str) -> tuple:
    core, _, pre = version.partition("-")
    nums = tuple(int(x) for x in core.split("."))
    # 带预发布后缀的排在同号正式版前面
    return (*nums, 0 if pre else 1, pre)


def classify(name: str) -> tuple[str, str] | None:
    for pattern, platform, arch in ASSET_KINDS:
        if pattern.match(name):
            return platform, arch
    return None


def load_api_pages(text: str) -> list[dict]:
    """`gh api --paginate` 会把每页的数组直接拼在一起输出（不是一个大数组），逐个解码后合并。"""
    decoder = json.JSONDecoder()
    items: list[dict] = []
    pos = 0
    while True:
        while pos < len(text) and text[pos].isspace():
            pos += 1
        if pos >= len(text):
            return items
        page, pos = decoder.raw_decode(text, pos)
        items.extend(page if isinstance(page, list) else [page])


def read_meta(meta_dir: Path | None, tag: str) -> tuple[dict[str, str], dict]:
    """某次发布的 SHA256SUMS（文件名 → 摘要）与 build-info.json；没有就空。"""
    if meta_dir is None:
        return {}, {}
    sums: dict[str, str] = {}
    sums_file = meta_dir / tag / "SHA256SUMS"
    if sums_file.is_file():
        for line in sums_file.read_text(encoding="utf-8").splitlines():
            parts = line.split()
            if len(parts) == 2:
                sums[parts[1].lstrip("*").removeprefix("./")] = parts[0]
    info_file = meta_dir / tag / "build-info.json"
    info = json.loads(info_file.read_text(encoding="utf-8")) if info_file.is_file() else {}
    return sums, info


def build(changelog: dict[str, dict], api: list[dict], repo: str, meta_dir: Path | None) -> dict:
    releases = []
    for item in api:
        if item.get("draft"):
            continue
        m = TAG.match(item.get("tag_name", ""))
        if not m:
            continue
        version = m.group(1)
        log = changelog.get(version)
        if log is None:
            print(f"警告：CHANGELOG 里没有 {version}，更新日志留空", file=sys.stderr)
            log = {"date": item.get("published_at", "")[:10], "channel": "beta", "notes": []}
        sums, info = read_meta(meta_dir, item["tag_name"])
        assets = []
        for asset in item.get("assets", []):
            kind = classify(asset["name"])
            if kind is None:
                continue
            assets.append({
                "platform": kind[0],
                "arch": kind[1],
                "file": asset["name"],
                "url": asset["browser_download_url"],
                "size": asset.get("size", 0),
                "sha256": sums.get(asset["name"], ""),
            })
        assets.sort(key=lambda a: (a["platform"], a["arch"]))
        releases.append({
            "version": version,
            "date": log["date"],
            "channel": log["channel"],
            "notes": log["notes"],
            "commit": info.get("commit", ""),
            "built_at": info.get("built_at", ""),
            "toolchain": info.get("toolchain", ""),
            "assets": assets,
        })
    releases.sort(key=lambda r: semver_key(r["version"]), reverse=True)
    return {
        "generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "repository": repo,
        "latest": releases[0]["version"] if releases else None,
        "releases": releases,
    }


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--changelog", type=Path, default=Path("CHANGELOG.md"))
    ap.add_argument("--repo", help="owner/name，写进输出")
    ap.add_argument("--api-json", type=Path, help="gh api repos/<repo>/releases --paginate 的输出")
    ap.add_argument("--meta-dir", type=Path, help="每个 tag 一个子目录，放 SHA256SUMS 与 build-info.json")
    ap.add_argument("--out", type=Path, help="releases.json 输出路径，缺省打印到 stdout")
    ap.add_argument("--notes-for", metavar="VERSION", help="只打印这个版本的更新日志（Markdown 列表）")
    args = ap.parse_args()
    # Windows 上 stdout 缺省是 cp1252，打印中文更新日志会 UnicodeEncodeError；这里的输出全走 UTF-8。
    sys.stdout.reconfigure(encoding="utf-8")

    changelog = parse_changelog(args.changelog)
    if args.notes_for:
        log = changelog.get(args.notes_for)
        if log is None:
            sys.exit(f"CHANGELOG 里没有 {args.notes_for}")
        print("\n".join(f"- {n}" for n in log["notes"]))
        return

    if not args.repo or not args.api_json:
        ap.error("生成 releases.json 需要 --repo 与 --api-json")
    api = load_api_pages(args.api_json.read_text(encoding="utf-8"))
    result = build(changelog, api, args.repo, args.meta_dir)
    text = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.out:
        args.out.write_text(text, encoding="utf-8")
        print(f"{args.out}：{len(result['releases'])} 个版本，最新 {result['latest']}")
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
