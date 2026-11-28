#!/usr/bin/env python3
"""守着 crate 依赖方向：谁都不许往上层要东西。

分层写在 `LEVELS` 里，数字小的在下。规则只有一条：**一个包只能依赖层级不高于自己的包**
（依赖同样层级的兄弟包是允许的）。另外 `BANS` 里是几条点名禁止的依赖，用来挡住「层级上说得通、
但语义上不该有」的边，比如 UI 引擎反过来认识青简的概念。

跑法：`python3 tools/ci/layers.py`，出错时退出码 1。CI 的 `core` job 会在 clippy 之前跑它。
改分层就改 `LEVELS`，同时同步 `docs/design/architecture.md` 的「crate 依赖方向」一节。
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

# Windows 控制台默认是 cp1252 / cp936，输出中文会炸；CI 上是 UTF-8，这里统一一下。
sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parents[2]

# 层级：0 是最底层。同一层可以互相依赖。
LEVELS = {
    # 叶层：零内部依赖
    "qingjian-format": 0,
    "qingjian-code": 0,
    "bamboo-slip": 0,
    "bamboo-slip-i18n": 0,
    "qingjian-clipboard-sync": 0,
    # 领域层
    "qingjian-dictionary": 1,
    "qingjian-core": 2,
    # 能力层：翻译、学习、联想、语言模型、神经模型、语音、剪贴板存储
    "qingjian-translate": 3,
    "qingjian-learning": 3,
    "qingjian-predict": 3,
    "qingjian-lm": 3,
    "qingjian-neural": 3,
    "qingjian-voice": 3,
    "qingjian-clipboard": 3,
    # 渲染层
    "qingjian-render": 4,
    # 平台层
    "qingjian-platform": 5,
    "qingjian-update": 6,
}

# 叶层与渲染层之后的东西（app、工具）统一按最上层算：它们只出不进。
TOP = 7

# 点名禁止的依赖：`依赖方 -> 被依赖方`，层级规则拦不住的语义错位放这里。
BANS = {
    # UI 引擎不认识青简的任何概念，反过来也不行
    "bamboo-slip": {"qingjian-core", "qingjian-platform", "qingjian-render", "qingjian-dictionary"},
    "bamboo-slip-i18n": {"qingjian-core", "qingjian-platform", "qingjian-render"},
    # 渲染层只画帧，不去拿引擎
    "qingjian-render": {
        "qingjian-core",
        "qingjian-platform",
        "qingjian-learning",
        "qingjian-predict",
    },
    # Core 平台无关：不许碰平台层、壳层与渲染
    "qingjian-core": {"qingjian-platform", "qingjian-render", "qingjian-update"},
    # 词库不反过来依赖引擎
    "qingjian-dictionary": {"qingjian-core", "qingjian-platform"},
}

def cargo_metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        sys.exit(f"cargo metadata 失败：\n{result.stderr}")
    return json.loads(result.stdout)


def level_of(name: str) -> int:
    return LEVELS.get(name, TOP)


def main() -> int:
    metadata = cargo_metadata()
    packages = {package["name"]: package for package in metadata["packages"]}
    internal = set(packages)
    problems: list[str] = []

    for name in sorted(packages):
        # 不在 LEVELS 里的是 app 与工具：按最上层算（只能往下要东西），
        # 它们自己不会被库 crate 依赖，所以下面只按 BANS 与层级两条判。
        manifest = Path(packages[name]["manifest_path"])
        relative = manifest.parent.relative_to(ROOT).as_posix()
        for dependency in packages[name]["dependencies"]:
            target = dependency["name"]
            if target not in internal:
                continue
            if target in BANS.get(name, set()):
                problems.append(
                    f"{name} 不许依赖 {target}（{relative}/Cargo.toml）"
                )
                continue
            if level_of(target) > level_of(name):
                problems.append(
                    f"{name}（第 {level_of(name)} 层）依赖了 {target}（第 {level_of(target)} 层）："
                    f"只能往下层或同层要东西（{relative}/Cargo.toml）"
                )

    if not problems:
        print(f"依赖方向正常：{len(packages)} 个包，分 {len(set(LEVELS.values())) + 1} 层")
        return 0

    print("依赖方向被破坏：", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    print(
        "\n分层见 docs/design/architecture.md「crate 依赖方向」；"
        "确要新增一条边，先改那里与 tools/ci/layers.py 的 LEVELS/BANS，再改代码。",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
