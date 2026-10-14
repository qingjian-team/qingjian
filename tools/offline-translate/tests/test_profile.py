"""验证接入离线服务时保留输入习惯设置，并留下可恢复的备份。"""

import contextlib
import io
from pathlib import Path
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import install_profile


class ProfileTests(unittest.TestCase):
    def test_existing_input_preferences_and_comments_are_preserved(self):
        original = '# 用户备注\n[general]\npage_size = 5\nlearning = false\n\n[predict]\nenabled = false\n\n[fuzzy]\nn_l = true\n'
        updated = install_profile.update_section(original, "predict", {"enabled": True, "automatic": False})
        parsed = tomllib.loads(updated)
        self.assertEqual(parsed["general"], {"page_size": 5, "learning": False})
        self.assertTrue(parsed["fuzzy"]["n_l"])
        self.assertIn("# 用户备注", updated)
        self.assertFalse(parsed["predict"]["automatic"])

    def test_new_section_is_appended_without_duplicate_keys(self):
        updated = install_profile.update_section('[general]\npage_size = 9\n', "update", {"check": False})
        self.assertFalse(tomllib.loads(updated)["update"]["check"])
        self.assertEqual(updated.count("[update]"), 1)

    def test_install_is_idempotent_and_preserves_original_backup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "config.toml"
            original = '[general]\npage_size = 5\nlearning = false\n\n[update]\ncheck = true\n'
            config.write_text(original, encoding="utf-8")
            arguments = ["install_profile.py", "--root", str(root / "offline"), "--config", str(config)]
            with patch.object(sys, "argv", arguments), contextlib.redirect_stdout(io.StringIO()):
                install_profile.main()
                first = tomllib.loads(config.read_text(encoding="utf-8"))
                install_profile.main()
            second = tomllib.loads(config.read_text(encoding="utf-8"))
            self.assertEqual(first, second)
            self.assertFalse(second["general"]["learning"])
            self.assertEqual(second["general"]["page_size"], 5)
            self.assertFalse(second["general"]["input_log"])
            self.assertFalse(second["predict"]["automatic"])
            self.assertEqual(second["predict"]["base_url"], "http://127.0.0.1:17863/v1")
            self.assertFalse(second["update"]["check"])
            backups = list((root / "offline" / "backups").glob("*.toml"))
            self.assertEqual(len(backups), 2)
            self.assertIn(original, [backup.read_text(encoding="utf-8") for backup in backups])


if __name__ == "__main__":
    unittest.main()
