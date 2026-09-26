"""参数个数门：函数形参超过阈值（默认 >7）。

  参数一多，调用方就得记住每个位置是什么，两个相邻的 `bool` / `u32` 参数随时能被对调而编译器
  一声不吭——这是最典型的一类「编译通过但语义错了」的 bug。正解是把一组参数收进一个
  结构体/配置对象（或用 builder）。Rust 没有 clippy 默认项拦这个（`clippy::too_many_arguments`
  默认阈值 7 且是 warn，不在 `-D warnings` 的 deny 面里就白搭）。

  X1 函数形参 >7 —— 棘轮（只准减）。`&self` 也占一个位置（方法与自由函数同口径，
  否则「把方法改成自由函数」就能绕过去）。只判产品代码。

用法：python3 -X utf8 scripts/gates/args_gate.py [--git-tracked] [--list] [--write] [--max 7]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

MAX = 7
SIG = re.compile(r"\bfn\s+[A-Za-z_]\w*\s*(?:<[^>]*>)?\s*\(")


def params_of(src: str, start: int) -> int:
    """从形参表的 `'('` 之后数**顶层**逗号：`fn f(a: Vec<(u8, u8)>, b: B)` 只算 2 个。"""
    depth, commas, nonempty = 0, 0, False
    for ch in src[start:]:
        if ch in "([{<":
            depth += 1
            nonempty = True
        elif ch in ")]}>":
            if depth == 0:
                break
            depth -= 1
        elif ch.isspace():
            continue
        else:
            nonempty = True
            if ch == "," and depth == 0:
                commas += 1
    return commas + 1 if nonempty else 0


def scan(root, a):
    cur = {}
    for rel in gc.rs_files(root, a.git_tracked):
        if rel.startswith("tests/") or gc.skip_hit(rel, gc.TESTISH):
            continue
        text = gc.read_text(root, rel)
        if not text:
            continue
        lines = gc.mask(text).splitlines()
        n = 0
        for i, _end in gc.iter_fns(lines):
            # 签名可能折行：往后看几行拼起来找 '('
            src = "\n".join(lines[i:i + 6])
            m = SIG.search(src)
            if m and params_of(src, m.end()) > a.max:
                n += 1
        if n:
            cur[rel] = n
    return cur


if __name__ == "__main__":
    sys.exit(gc.run_count_gate(
        "ARGS-GATE", "docs/review/args-baseline.json", scan, f"个 >{MAX} 参函数",
        extra_args=lambda ap: ap.add_argument("--max", type=int, default=MAX),
        head=lambda cur, a: print(f"ARGS-GATE(>{a.max}) count={sum(cur.values())}"),
        hard=True))         # 存量 = 0 ⇒ 硬阈：一个 >7 参的函数都不许有
