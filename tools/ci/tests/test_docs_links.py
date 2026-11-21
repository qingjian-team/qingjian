import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import check_docs_links as links


class IterLinksTests(unittest.TestCase):
    def test_skips_fenced_blocks_and_inline_code(self) -> None:
        text = (
            "`[示例](missing.md)`\n"
            "```\n"
            "[围栏里](missing2.md)\n"
            "```\n"
            "[真链接](exists.md)\n"
        )
        found = list(links.iter_links(text))
        self.assertEqual(found, [(5, "exists.md")])


class MarkdownFilesTests(unittest.TestCase):
    def test_scans_subproject_readmes_and_skips_build_dirs(self) -> None:
        files = links.markdown_files()
        rel = {p.relative_to(links.ROOT).as_posix() for p in files}
        self.assertIn("apps/android/README.md", rel)
        self.assertFalse(any(path.startswith("target/") for path in rel))


class CheckTests(unittest.TestCase):
    def test_reports_missing_and_ignores_external_and_placeholder(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "exists.md").write_text("# ok\n", encoding="utf-8")
            page = root / "page.md"
            page.write_text(
                "[ok](exists.md)\n"
                "[bad](missing.md)\n"
                "[外链](https://example.com/doc.md)\n"
                "[占位](链接)\n"
                "[锚点](#section)\n",
                encoding="utf-8",
            )
            missing = links.check([page])
        self.assertEqual(len(missing), 1)
        self.assertIn("missing.md", missing[0])

    def test_folder_link_must_exist(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "sub").mkdir()
            page = root / "page.md"
            page.write_text("[dir](sub)\n[gone](gone)\n", encoding="utf-8")
            missing = links.check([page])
        self.assertEqual(len(missing), 1)
        self.assertIn("gone", missing[0])


if __name__ == "__main__":
    unittest.main()
