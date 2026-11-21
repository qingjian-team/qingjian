import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import kotlin_gate as gate


def write_temp(code: str) -> Path:
    directory = tempfile.mkdtemp()
    path = Path(directory) / "Sample.kt"
    path.write_text(code, encoding="utf-8")
    return path


class CountFileTests(unittest.TestCase):
    def test_counts_notnull_suppress_and_todo(self) -> None:
        path = write_temp(
            """
@SuppressLint("ViewConstructor")
class Sample {
    fun f() {
        val value = maybe!!
    }
    // TODO: 把手势补完
}
"""
        )
        counts = gate.count_file(path)
        self.assertEqual(counts["notnull"], 1)
        self.assertEqual(counts["suppress"], 1)
        self.assertEqual(counts["todo"], 1)

    def test_ignores_patterns_inside_comments(self) -> None:
        path = write_temp(
            """
// @Suppress("DEPRECATION") 只是注释
// 别写 value!!
val value = other
"""
        )
        counts = gate.count_file(path)
        self.assertNotIn("suppress", counts)
        self.assertNotIn("notnull", counts)


class CompareTests(unittest.TestCase):
    def test_total_growth_fails(self) -> None:
        baseline = ({"notnull": 1}, {"Sample.kt": {"notnull": 1}})
        failures = gate.compare({"notnull": 2}, {"Sample.kt": {"notnull": 2}}, baseline)
        self.assertTrue(any("总数" in item for item in failures))

    def test_new_file_with_pattern_fails(self) -> None:
        baseline = ({"todo": 0}, {})
        failures = gate.compare({"todo": 1}, {"New.kt": {"todo": 1}}, baseline)
        self.assertTrue(any("New.kt" in item for item in failures))


if __name__ == "__main__":
    unittest.main()
