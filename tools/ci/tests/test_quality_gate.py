import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import quality_gate as gate


def write_temp(code: str) -> Path:
    directory = tempfile.mkdtemp()
    path = Path(directory) / "sample.rs"
    path.write_text(code, encoding="utf-8")
    return path


class CountFileTests(unittest.TestCase):
    def test_counts_dangerous_patterns(self) -> None:
        path = write_temp(
            """
fn f() {
    unsafe { std::ptr::null::<u8>(); }
    let a = x.unwrap();
    let b = y.expect("boom");
    if false { panic!("no"); }
    todo!();
}
#[allow(clippy::all)]
fn g() {}
"""
        )
        counts = gate.count_file(path)
        self.assertEqual(counts["unsafe"], 1)
        self.assertEqual(counts["unwrap"], 1)
        self.assertEqual(counts["expect"], 1)
        self.assertEqual(counts["panic"], 1)
        self.assertEqual(counts["todo"], 1)
        self.assertEqual(counts["allow"], 1)

    def test_ignores_unwrap_or_and_test_modules(self) -> None:
        path = write_temp(
            """
fn real() {
    let a = x.unwrap_or(0);
}
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let b = y.unwrap();
        unsafe { std::ptr::null::<u8>(); }
    }
}
"""
        )
        counts = gate.count_file(path)
        self.assertNotIn("unwrap", counts)
        self.assertNotIn("unsafe", counts)


class CompareTests(unittest.TestCase):
    def test_same_values_pass(self) -> None:
        baseline = ({"unsafe": 2, "unwrap": 1}, {"a.rs": {"unsafe": 2}})
        total = {"unsafe": 2, "unwrap": 1}
        files = {"a.rs": {"unsafe": 2}}
        self.assertEqual(gate.compare(total, files, baseline), [])

    def test_total_growth_fails(self) -> None:
        baseline = ({"unsafe": 2}, {"a.rs": {"unsafe": 2}})
        failures = gate.compare({"unsafe": 3}, {"a.rs": {"unsafe": 3}}, baseline)
        self.assertTrue(any("总数" in item for item in failures))

    def test_new_file_with_pattern_fails(self) -> None:
        baseline = ({"unwrap": 0}, {})
        failures = gate.compare({"unwrap": 1}, {"new.rs": {"unwrap": 1}}, baseline)
        self.assertTrue(any("new.rs" in item for item in failures))


class ReportTests(unittest.TestCase):
    def test_report_lists_failures(self) -> None:
        old = gate.REPORT
        try:
            with tempfile.TemporaryDirectory() as tmp:
                gate.REPORT = Path(tmp) / "report.md"
                gate.write_report({"unsafe": 3}, {"a.rs": {"unsafe": 3}}, ["总数 `unsafe`：3 > 基线 2"])
                self.assertIn("总数 `unsafe`", gate.REPORT.read_text(encoding="utf-8"))
        finally:
            gate.REPORT = old


if __name__ == "__main__":
    unittest.main()
