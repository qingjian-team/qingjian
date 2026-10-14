"""备份现有设置，仅把翻译接口接到本机服务；不读取学习记录。"""

import argparse
from datetime import datetime
import json
import os
from pathlib import Path
import re
import secrets
import tomllib


def update_section(text: str, name: str, fields: dict) -> str:
    match = re.search(rf"(?m)^\[{re.escape(name)}\]\s*$", text)
    if match:
        following = re.search(r"(?m)^\[", text[match.end():])
        end = match.end() + following.start() if following else len(text)
        prefix, body, suffix = text[:match.end()], text[match.end():end], text[end:]
    else:
        prefix, body, suffix = text.rstrip() + f"\n\n[{name}]", "\n", ""
    for key, value in fields.items():
        literal = json.dumps(value, ensure_ascii=False)
        line = f"{key} = {literal}"
        expression = rf"(?m)^{re.escape(key)}\s*=.*$"
        if re.search(expression, body):
            body = re.sub(expression, lambda _match: line, body)
        else:
            body = body.rstrip() + "\n" + line + "\n"
    return prefix + body.rstrip() + "\n\n" + suffix


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--config", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    token_file = root / "local-token.txt"
    if token_file.exists():
        token = token_file.read_text(encoding="utf-8").strip()
    else:
        token = secrets.token_urlsafe(32)
        token_file.write_text(token, encoding="utf-8")
    if len(token) < 24:
        raise ValueError("本地访问令牌无效")
    config = args.config or Path(os.environ["APPDATA"]) / "Qingjian" / "config.toml"
    original = config.read_text(encoding="utf-8-sig") if config.exists() else ""
    tomllib.loads(original)
    backups = root / "backups"
    backups.mkdir(exist_ok=True)
    backup = backups / ("config-" + datetime.now().strftime("%Y%m%d-%H%M%S-%f") + ".toml")
    backup.write_text(original, encoding="utf-8")
    updated = update_section(original, "predict", {
        "enabled": True, "automatic": False,
        "base_url": "http://127.0.0.1:17863/v1", "model": "qingjian-offline-zh-en",
        "api_key": token, "timeout_ms": 60_000, "debounce_ms": 0, "min_interval_ms": 0,
        "lookback": 0, "lookahead": 0, "slots": 0, "sentence": False,
    })
    updated = update_section(updated, "general", {
        "learning_language": "en", "renderer": "qingjian", "input_log": False, "log_level": "info",
    })
    updated = update_section(updated, "shortcut", {"translate_selection": "ctrl+alt+t"})
    updated = update_section(updated, "update", {"check": False})
    tomllib.loads(updated)
    config.parent.mkdir(parents=True, exist_ok=True)
    temporary = config.with_suffix(".toml.tmp")
    temporary.write_text(updated, encoding="utf-8")
    temporary.replace(config)
    print("离线翻译配置已应用；原设置备份位于：" + str(backup))


if __name__ == "__main__":
    main()
