"""unsafe 块门：产品代码里 `unsafe { … }` 块与 `unsafe fn` / `unsafe impl` 的数量。

  Rust 的 unsafe 是「这块编译器不帮你担保内存安全」——用得越多，未定义行为面越大，reviewer 要
  逐行盯。FFI / 平台后端里 unsafe 是正当的（`ffi` / `sys` / `platform` / `bindings` / 各 OS 壳目录），
  排除；测试也排除。其余代码每多一个 unsafe 块就多一分风险——本门只拦新增。

  X1 产品代码（排除 FFI/平台/测试）unsafe 块数 —— 棘轮：只准减（新增即红）。

用法：python3 -X utf8 scripts/gates/unsafe_gate.py [--git-tracked] [--list] [--write]
"""
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gates_common as gc  # noqa: E402

# FFI / 平台后端里的 unsafe 是正当的：这些目录不进扫描面
FFI_DIRS = ("/ffi/", "/sys/", "/platform/", "/bindings/", "/macos/", "/windows/",
            "/linux/", "/darwin/", "-sys/")
UNSAFE = re.compile(r"unsafe\s*\{|\bunsafe\s+(?:fn|impl|trait)\b")

if __name__ == "__main__":
    sys.exit(gc.run_count_gate("UNSAFE-GATE", "docs/review/unsafe-baseline.json",
                               gc.regex_scan(UNSAFE, extra_skip=FFI_DIRS),
                               "处 unsafe 块/声明"))
