"""check_god_objects.py 的单元测试。

跑法：python3 -m unittest discover -s tools/quality -p "test_*.py"
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import check_god_objects as gate


def run_quiet(fn, *args, **kwargs):
    """跑被测函数但不让它把报告打到测试输出里。

    CI 日志里出现「上帝对象门禁失败。」会被误读成这个 job 红了——那只是反向用例在验失败路径。
    """
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        return fn(*args, **kwargs)


def write(root: Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def impl_lines(count: int) -> str:
    """生成一个 impl 块，块内恰好 count 行代码（含 impl 签名与右括号）。"""
    body_count = max(0, count - 2)
    body = "\n".join(f"    fn f{i}(&self) -> u8 {{ {i} }}" for i in range(body_count))
    return f"impl Widget {{\n{body}\n}}\n"


class ImplTargetTest(unittest.TestCase):
    def test_plain(self) -> None:
        self.assertEqual(gate.impl_target(" Widget "), "Widget")

    def test_generic_and_lifetime(self) -> None:
        self.assertEqual(gate.impl_target(" Metrics<'_> "), "Metrics")
        self.assertEqual(gate.impl_target("<T: Into<Vec<u8>>> Widget<T> "), "Widget")

    def test_trait_impl(self) -> None:
        self.assertEqual(gate.impl_target(" Display for Widget "), "Widget")
        self.assertEqual(gate.impl_target(" Display for &Widget "), "Widget")

    def test_where_clause(self) -> None:
        self.assertEqual(gate.impl_target(" Widget<T> where T: Copy "), "Widget")

    def test_path(self) -> None:
        self.assertEqual(gate.impl_target(" crate::ui::Widget "), "Widget")


class ScanTest(unittest.TestCase):
    def test_definition_and_impl_aggregate_across_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root, "crates/demo/src/lib.rs", "pub struct Widget {\n    a: u8,\n}\n")
            write(root, "crates/demo/src/impl_a.rs", impl_lines(10))
            write(root, "crates/demo/src/impl_b.rs", impl_lines(7))
            entries = gate.scan(root)
            entry = entries["crates/demo::Widget"]
            # 定义 3 行 + impl 10 行 + impl 7 行
            self.assertEqual(entry.lines, 20)
            self.assertEqual(len(entry.refs), 3)

    def test_comments_and_blank_lines_do_not_count(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            text = "pub struct Widget {\n    a: u8,\n}\n"
            text += "\n".join("// 注释" for _ in range(400)) + "\n"
            text += "\n".join("" for _ in range(400)) + "\n"
            text += impl_lines(10)
            write(root, "crates/demo/src/lib.rs", text)
            entry = gate.scan(root)["crates/demo::Widget"]
            self.assertEqual(entry.lines, 3 + 10)

    def test_block_comment_does_not_break_brace_matching(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(
                root,
                "crates/demo/src/lib.rs",
                "pub struct Widget {\n    a: u8,\n}\n"
                "/* impl Fake { 这个注释里的括号不能参与配对 */\n"
                + impl_lines(5),
            )
            entry = gate.scan(root)["crates/demo::Widget"]
            self.assertEqual(entry.lines, 3 + 5)

    def test_string_with_braces_does_not_break_matching(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(
                root,
                "crates/demo/src/lib.rs",
                "pub struct Widget {\n    a: u8,\n}\n"
                + "impl Widget {\n    fn a(&self) -> &'static str { \"}\" }\n}\n",
            )
            entry = gate.scan(root)["crates/demo::Widget"]
            self.assertEqual(entry.lines, 3 + 3)

    def test_same_name_in_different_crates_are_separate(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root, "crates/one/src/lib.rs", "pub struct Widget;\n")
            write(root, "crates/two/src/lib.rs", "pub struct Widget;\n")
            entries = gate.scan(root)
            self.assertIn("crates/one::Widget", entries)
            self.assertIn("crates/two::Widget", entries)
            self.assertEqual(entries["crates/one::Widget"].lines, 1)


class CheckTest(unittest.TestCase):
    def test_new_god_object_fails(self) -> None:
        entries = {"crates/demo::Big": gate.Entry("crates/demo::Big", lines=900)}
        violations, _ = gate.check(entries, 800, {})
        self.assertEqual(len(violations), 1)
        self.assertIn("reason=new", violations[0])

    def test_registered_growth_fails(self) -> None:
        entries = {"crates/demo::Big": gate.Entry("crates/demo::Big", lines=901)}
        violations, _ = gate.check(entries, 800, {"crates/demo::Big": 900})
        self.assertEqual(len(violations), 1)
        self.assertIn("reason=grew", violations[0])

    def test_registered_unchanged_passes(self) -> None:
        entries = {"crates/demo::Big": gate.Entry("crates/demo::Big", lines=900)}
        violations, notes = gate.check(entries, 800, {"crates/demo::Big": 900})
        self.assertEqual(violations, [])
        self.assertEqual(notes, [])

    def test_below_limit_passes(self) -> None:
        entries = {"crates/demo::Small": gate.Entry("crates/demo::Small", lines=700)}
        violations, _ = gate.check(entries, 800, {})
        self.assertEqual(violations, [])

    def test_stale_allowlist_is_reported(self) -> None:
        entries = {"crates/demo::Big": gate.Entry("crates/demo::Big", lines=120)}
        _, notes = gate.check(entries, 800, {"crates/demo::Big": 900})
        self.assertEqual(len(notes), 1)
        self.assertIn("--update", notes[0])


class UpdateTest(unittest.TestCase):
    def test_update_writes_allowlist(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "allow.toml"
            entries = {
                "crates/demo::Big": gate.Entry("crates/demo::Big", lines=900),
                "crates/demo::Small": gate.Entry("crates/demo::Small", lines=700),
            }
            run_quiet(gate.write_allowlist, path, 800, entries)
            data = tomllib.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(data["limits"]["god_object_lines"], 800)
            self.assertEqual(data["allow"], {"crates/demo::Big": 900})

    def test_main_update_then_check_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root, "crates/demo/src/lib.rs", impl_lines(900))
            allow = root / "allow.toml"
            self.assertEqual(run_quiet(gate.main, ["--root", str(root), "--allowlist", str(allow), "--update"]), 0)
            self.assertEqual(run_quiet(gate.main, ["--root", str(root), "--allowlist", str(allow)]), 0)
            # 再长一行就失败
            write(root, "crates/demo/src/lib.rs", impl_lines(901))
            self.assertEqual(run_quiet(gate.main, ["--root", str(root), "--allowlist", str(allow)]), 1)


if __name__ == "__main__":
    unittest.main()
