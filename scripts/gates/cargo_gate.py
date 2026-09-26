"""Cargo.toml 约定门：把 `docs/contributing.md`「依赖与配置 / 版本号」那几条写死的规矩变成硬判。

  这三条都是**发版时会出真事故**的，且存量都为 0 ⇒ 全部走**硬规则**（不接受基线祖父化）：

  C1 `crates/*/Cargo.toml` 必须 `version.workspace = true`
     （workspace 统一版本；写死就会在发版时对不上）
  C2 `apps/{macos,linux,windows}*/Cargo.toml` 必须**写死**自己的 `version`（不许 `version.workspace`）
     （各壳是独立发布的产品，Windows 读 `server/Cargo.toml` 取版本号 ⇒ 写错就是发错版）
     `apps/cli` 不判：它是 workspace 里的工具，不是独立发布的壳。
  C3 **全仓不许 `anyhow`**（依赖表里出现即红，代码里 `anyhow::` 同样即红）
     （contributing：错误用 `thiserror` 不用 `anyhow`。库里抛 `anyhow::Error` 会把具体错误类型
      抹掉，调用方只能 `downcast` 猜，等于放弃了 Rust 的错误可分类性）

用法：python3 -X utf8 scripts/gates/cargo_gate.py [--git-tracked] [--list]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

SHELLS = ("apps/macos/", "apps/linux/", "apps/windows/")
ANYHOW = re.compile(r"\banyhow\b")
VER_OWN = re.compile(r"^version\s*=\s*\"", re.M)
VER_WS = re.compile(r"^version\.workspace\s*=\s*true", re.M)


def cargo_tomls(root: pathlib.Path, git_tracked: bool):
    return [p for p in gc.rs_files(root, git_tracked, suffix="Cargo.toml")]


def scan(root, git_tracked):
    bad = []
    for rel in cargo_tomls(root, git_tracked):
        text = gc.read_text(root, rel)
        if not text:
            continue
        if rel.startswith("crates/") and "/" not in rel[len("crates/"):-len("/Cargo.toml")]:
            if not VER_WS.search(text):
                bad.append(f"C1 {rel}: crates 必须 `version.workspace = true`")
        if rel.startswith(SHELLS):
            if VER_WS.search(text):
                bad.append(f"C2 {rel}: 壳必须写死自己的 version，不许 `version.workspace`")
            elif not VER_OWN.search(text):
                bad.append(f"C2 {rel}: 壳缺写死的 `version = \"…\"`")
        if ANYHOW.search(text):
            bad.append(f"C3 {rel}: 依赖/配置里出现 anyhow（错误用 thiserror）")
    for rel in gc.rs_files(root, git_tracked):
        text = gc.read_text(root, rel)
        if text and ANYHOW.search(gc.mask(text)):
            bad.append(f"C3 {rel}: 代码里用了 anyhow（错误用 thiserror）")
    return sorted(bad)


def main() -> int:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--git-tracked", action="store_true")
    a = ap.parse_args()
    bad = scan(gc.ROOT, a.git_tracked)
    print(f"CARGO-GATE 命中={len(bad)}（C1 版本统一 / C2 壳写死版本 / C3 不用 anyhow，全部硬判）")
    for line in bad[:25]:
        print(f"  ✗ {line}")
    if len(bad) > 25:
        print(f"  …另有 {len(bad) - 25} 条")
    print(f"CARGO-GATE {'FAIL' if bad else 'OK'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
