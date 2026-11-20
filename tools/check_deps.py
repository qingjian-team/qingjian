#!/usr/bin/env python3
"""架构依赖规则静态校验。

数据源只有 `cargo metadata --format-version 1` 的结构化输出；本脚本不解析任何
Cargo.toml 文本。规则分两类：

  * 失败项（退出码 1）：分层方向、跨层反向依赖、依赖图成环（正常图 / dev 图分别
    检查）。存量违规登记在 tools/deps-allowlist.toml，只有登记过的边才放行；
    新增违规、登记边扩大（多出来一种 kind）、豁免到期，都会失败。
  * 提示项（不影响退出码）：同一外部/内部依赖被多个 crate 以不同 feature 组合
    引入，仅打印 HINT，提醒收敛。

用法：
    python3 tools/check_deps.py                # 本地 / CI 常规跑（走网络解析）
    python3 tools/check_deps.py --offline      # 离线环境（只读 Cargo.lock / 本地缓存）
    python3 tools/check_deps.py --update      # 按当前违规重写 allowlist（保留已登记理由）
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST = ROOT / "tools" / "deps-allowlist.toml"

# --- 规则表 -----------------------------------------------------------------

# qingjian-core 是平台无关的引擎内核，不得反向依赖平台层、渲染层、业务服务层、
# 剪贴板套件与代码生成辅助 crate。
CORE_FORBIDDEN = {
    "qingjian-platform",
    "qingjian-render",
    "bamboo-slip",
    "qingjian-predict",
    "qingjian-neural",
    "qingjian-lm",
    "qingjian-voice",
    "qingjian-update",
    "qingjian-clipboard",
    "qingjian-clipboard-sync",
    "qingjian-code",
}

# 渲染层：自绘 UI 引擎与候选窗渲染器。
RENDER_LAYER = {"bamboo-slip", "qingjian-render"}

# 业务服务层：联想 / 翻译 / 神经打分 / 语言模型 / 语音 / 更新检查。
SERVICE_LAYER = {
    "qingjian-predict",
    "qingjian-translate",
    "qingjian-neural",
    "qingjian-lm",
    "qingjian-voice",
    "qingjian-update",
}

@dataclass
class Dep:
    name: str
    path: str | None
    kind: str  # normal | dev | build
    target: str | None
    features: tuple[str, ...]
    uses_default: bool
    optional: bool


@dataclass
class Violation:
    rule: str
    src: str
    dst: str
    kind: str
    target: str | None
    evidence: str

    @property
    def key(self) -> str:
        return f"{self.rule}|{self.src}|{self.dst}"


# --- metadata 解析 ----------------------------------------------------------


def load_metadata(offline: bool) -> dict:
    cargo = os.environ.get("CARGO", "cargo")
    toolchain = os.environ.get("CARGO_TOOLCHAIN", "")
    cmd = [cargo]
    if toolchain:
        cmd.append(toolchain)
    cmd += ["metadata", "--format-version", "1"]
    if offline:
        cmd.append("--offline")
    proc = subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"cargo metadata 失败（退出码 {proc.returncode}）")
    return json.loads(proc.stdout)


def classify(rel_manifest: Path) -> str:
    """按 workspace 成员清单路径归类：crates / apps / tools。"""
    parts = rel_manifest.parts
    return parts[0] if parts else "?"


def build_members(meta: dict) -> dict[str, dict]:
    members: dict[str, dict] = {}
    for pkg in meta["packages"]:
        manifest = Path(pkg["manifest_path"])
        try:
            rel_manifest = manifest.relative_to(ROOT)
            layer = classify(rel_manifest.parent)
        except ValueError:
            layer = "external"
        deps: list[Dep] = []
        for d in pkg["dependencies"]:
            deps.append(
                Dep(
                    name=d["name"],
                    path=d.get("path"),
                    kind=d.get("kind") or "normal",
                    target=d.get("target"),
                    features=tuple(sorted(d.get("features") or [])),
                    uses_default=bool(d.get("uses_default_features", True)),
                    optional=bool(d.get("optional", False)),
                )
            )
        members[pkg["name"]] = {
            "layer": layer,
            "manifest": rel_manifest if layer != "external" else manifest,
            "deps": deps,
        }
    return members


def internal_dep(members: dict[str, dict], dep: Dep) -> str | None:
    """返回内部依赖目标成员名；外部依赖返回 None。"""
    if not dep.path:
        return None
    try:
        Path(dep.path).relative_to(ROOT)
    except ValueError:
        return None
    return dep.name if dep.name in members else None


# --- 规则检查 ---------------------------------------------------------------


def edge_evidence(member: dict, dep: Dep, dst: str) -> str:
    feat = ""
    if dep.features:
        feat = f" features=[{', '.join(dep.features)}]"
    tgt = f" target={dep.target}" if dep.target else ""
    return f"{member['manifest']}: {dep.kind} dep {dst}{feat}{tgt}"


def check_layering(members: dict[str, dict]) -> list[Violation]:
    violations: list[Violation] = []
    for src, member in members.items():
        if member["layer"] == "external":
            continue
        for dep in member["deps"]:
            dst = internal_dep(members, dep)
            if dst is None:
                continue
            dst_layer = members[dst]["layer"]
            ev = edge_evidence(member, dep, dst)

            if src == "qingjian-core" and dst in CORE_FORBIDDEN:
                violations.append(Violation("core-forbidden", src, dst, dep.kind, dep.target, ev))
            if src in RENDER_LAYER and dst in SERVICE_LAYER:
                violations.append(Violation("render-to-service", src, dst, dep.kind, dep.target, ev))
            if member["layer"] == "crates" and dst_layer in ("apps", "tools"):
                violations.append(Violation("crate-to-shell", src, dst, dep.kind, dep.target, ev))
            if member["layer"] in ("apps", "tools") and dst_layer in ("apps", "tools"):
                violations.append(Violation("shell-to-shell", src, dst, dep.kind, dep.target, ev))
    return violations


def find_cycles(members: dict[str, dict], kinds: set[str]) -> list[list[str]]:
    """在指定 kind 集合构成的内部依赖图上找环，返回环上节点列表（含重复起点）。"""
    graph: dict[str, list[str]] = {
        name: [
            internal_dep(members, dep)
            for dep in m["deps"]
            if dep.kind in kinds and internal_dep(members, dep) is not None
        ]
        for name, m in members.items()
        if m["layer"] != "external"
    }
    cycles: list[list[str]] = []
    color: dict[str, int] = {n: 0 for n in graph}
    stack: list[str] = []

    def dfs(node: str) -> None:
        color[node] = 1
        stack.append(node)
        for nxt in graph[node]:
            if nxt not in color:
                continue
            if color[nxt] == 1:
                start = stack.index(nxt)
                cycles.append(stack[start:] + [nxt])
            elif color[nxt] == 0:
                dfs(nxt)
        stack.pop()
        color[node] = 2

    for n in graph:
        if color[n] == 0:
            dfs(n)
    return cycles


def check_cycles(members: dict[str, dict]) -> tuple[list[Violation], list[list[str]]]:
    normal = find_cycles(members, {"normal", "build"})
    dev = find_cycles(members, {"dev"})
    violations = [
        Violation(
            "cycle-normal",
            cycle[0],
            cycle[-2],
            "normal",
            None,
            "normal 依赖环: " + " -> ".join(cycle),
        )
        for cycle in normal
    ]
    return violations, dev


def feature_hints(members: dict[str, dict]) -> list[str]:
    """同一依赖被多个 crate 以不同 (default-features, features) 组合引入 → 提示。"""
    combos: dict[str, dict[tuple[bool, tuple[str, ...]], list[str]]] = {}
    for name, member in members.items():
        if member["layer"] == "external":
            continue
        for dep in member["deps"]:
            key = (dep.uses_default, dep.features)
            combos.setdefault(dep.name, {}).setdefault(key, []).append(name)
    hints: list[str] = []
    for dep_name, by in sorted(combos.items()):
        if len(by) <= 1:
            continue
        detail = "; ".join(
            f"default={default} features=[{', '.join(feats)}] <- {', '.join(sorted(crates))}"
            for (default, feats), crates in sorted(by.items())
        )
        hints.append(f"dep={dep_name} combos={len(by)} :: {detail}")
    return hints


# --- allowlist --------------------------------------------------------------


def load_allowlist() -> tuple[dict[str, dict], dict[str, dict], dict[str, object]]:
    if not ALLOWLIST.exists():
        return {}, {}, {}
    data = tomllib.loads(ALLOWLIST.read_text(encoding="utf-8"))
    allow = {k: dict(v) for k, v in data.get("allow", {}).items()}
    dev_cycles = {k: dict(v) for k, v in data.get("dev-cycles", {}).items()}
    meta = dict(data.get("meta", {}))
    return allow, dev_cycles, meta


def entry_ok(entry: dict, today: date) -> tuple[bool, str]:
    """校验豁免条目字段齐全且未到期；返回 (是否有效, 说明)。"""
    reason = str(entry.get("reason", "")).strip()
    if not reason:
        return False, "缺少 reason（豁免理由）"
    expires = str(entry.get("expires", "")).strip()
    if not expires:
        return False, "缺少 expires（到期日期或 permanent）"
    if expires != "permanent":
        try:
            due = date.fromisoformat(expires)
        except ValueError:
            return False, f"expires 不是 YYYY-MM-DD 或 permanent：{expires}"
        if due < today:
            return False, f"豁免已于 {expires} 到期，请续期或移除"
    return True, ""


def evaluate(
    violations: list[Violation],
    dev_cycles: list[list[str]],
    allow: dict[str, dict],
    cycle_allow: dict[str, dict],
    today: date,
) -> tuple[list[Violation], list[Violation], list[str]]:
    """返回 (失败违规, 已登记放行, 提示信息)。"""
    failing: list[Violation] = []
    allowed: list[Violation] = []
    notes: list[str] = []

    registered = set(allow)
    for v in violations:
        entry = allow.get(v.key)
        if entry is None:
            failing.append(v)
            continue
        ok, why = entry_ok(entry, today)
        if not ok:
            failing.append(v)
            notes.append(f"{v.key}: 豁免条目无效（{why}）")
            continue
        kinds = entry.get("kinds")
        if isinstance(kinds, list) and v.kind not in kinds:
            failing.append(v)
            notes.append(f"{v.key}: 登记 kinds={kinds}，实际多出 kind={v.kind}（违规变大）")
            continue
        allowed.append(v)

    registered.difference_update(v.key for v in violations)
    for cycle in dev_cycles:
        key = "|".join(cycle)
        entry = cycle_allow.get(key)
        if entry is None:
            failing.append(
                Violation("cycle-dev", cycle[0], cycle[-2], "dev", None, "dev 依赖环: " + " -> ".join(cycle))
            )
            continue
        ok, why = entry_ok(entry, today)
        if not ok:
            failing.append(
                Violation("cycle-dev", cycle[0], cycle[-2], "dev", None, "dev 依赖环: " + " -> ".join(cycle))
            )
            notes.append(f"dev 环 {key}: 豁免条目无效（{why}）")
        else:
            allowed.append(
                Violation("cycle-dev", cycle[0], cycle[-2], "dev", None, "dev 依赖环: " + " -> ".join(cycle))
            )
    for key in cycle_allow:
        if key not in {"|".join(c) for c in dev_cycles}:
            registered.add(key)

    for stale in sorted(registered):
        notes.append(f"提示：豁免条目 {stale} 已不对应任何违规，可从 allowlist 移除")
    return failing, allowed, notes


# --- 输出 -------------------------------------------------------------------


def print_violation(v: Violation, tag: str) -> None:
    target = v.target if v.target else "-"
    print(f"{tag} rule={v.rule} src={v.src} dst={v.dst} kind={v.kind} target={target} evidence={v.evidence}")


def render_allowlist(
    violations: list[Violation],
    dev_cycles: list[list[str]],
    allow: dict[str, dict],
    cycle_allow: dict[str, dict],
    meta: dict[str, object],
) -> str:
    lines = [
        "# 架构依赖存量豁免。只有登记过的违规才放行；新增违规、登记边变大（多出 kind）、",
        "# 豁免到期，CI 都会失败。重构消除违规后，删掉对应条目即可。",
        "# 每条必须带 reason（为什么暂时不能改）与 expires（YYYY-MM-DD 或 permanent）。",
        "",
    ]
    if meta:
        lines.append("[meta]")
        for key, value in meta.items():
            lines.append(f'{key} = "{value}"')
        lines.append("")
    lines.append("[allow]")
    current = {v.key: v for v in violations}
    for key in sorted(set(allow) | set(current)):
        old = allow.get(key, {})
        v = current.get(key)
        kinds = old.get("kinds", [v.kind] if v else [])
        reason = old.get("reason", "")
        expires = old.get("expires", "")
        lines.append(
            f'"{key}" = {{ reason = "{reason}", expires = "{expires}", kinds = {json.dumps(kinds)} }}'
        )
    lines += ["", "[dev-cycles]"]
    current_cycles = {"|".join(c) for c in dev_cycles}
    for key in sorted(set(cycle_allow) | current_cycles):
        old = cycle_allow.get(key, {})
        reason = old.get("reason", "")
        expires = old.get("expires", "")
        lines.append(f'"{key}" = {{ reason = "{reason}", expires = "{expires}" }}')
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="架构依赖规则静态校验")
    parser.add_argument("--offline", action="store_true", help="cargo metadata 加 --offline")
    parser.add_argument("--update", action="store_true", help="按当前违规重写 allowlist")
    args = parser.parse_args()

    meta = load_metadata(args.offline)
    members = build_members(meta)

    violations = check_layering(members)
    cycle_violations, dev_cycles = check_cycles(members)
    violations += cycle_violations
    hints = feature_hints(members)

    allow, cycle_allow, allow_meta = load_allowlist()
    today = date.today()

    if args.update:
        ALLOWLIST.write_text(
            render_allowlist(violations, dev_cycles, allow, cycle_allow, allow_meta),
            encoding="utf-8",
        )
        print(f"已重写 {ALLOWLIST.relative_to(ROOT)}：分层/成环违规 {len(violations)} 条，dev 环 {len(dev_cycles)} 个")
        return 0

    failing, allowed, notes = evaluate(violations, dev_cycles, allow, cycle_allow, today)

    ws = [m for m in members.values() if m["layer"] != "external"]
    edge_count = sum(
        1 for m in members.values() for d in m["deps"] if internal_dep(members, d)
    )
    print(f"workspace 成员 {len(ws)} 个；内部依赖边 {edge_count} 条"
          f"（metadata 共解析 {len(members)} 个包，含外部传递依赖）")
    print(f"分层/成环违规：{len(violations)} 条（已登记放行 {len(allowed)}，未放行 {len(failing)}）")
    print(f"feature 组合提示：{len(hints)} 条（不影响门禁）")

    for v in sorted(allowed, key=lambda x: x.key):
        print_violation(v, "ALLOWED")
    for note in notes:
        print(f"NOTE {note}")
    for h in hints:
        print(f"HINT {h}")
    for v in sorted(failing, key=lambda x: x.key):
        print_violation(v, "VIOLATION")

    if failing:
        print(f"\n架构依赖门禁失败：{len(failing)} 条未登记违规。"
              f"新依赖请遵守 docs/dependency-rules.md；存量需登记 tools/deps-allowlist.toml。")
        return 1
    print("\n架构依赖门禁通过。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
