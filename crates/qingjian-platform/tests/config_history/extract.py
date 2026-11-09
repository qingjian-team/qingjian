#!/usr/bin/env python3
# //! 从 git 历史里还原某个提交写给用户的配置模板（qingjian-platform 的 TEMPLATE），按平台各出一份，存成回归测试的样本。
# //! TEMPLATE 是 concat! 拼起来的：原始字符串字面量，加按平台 #[cfg(...)] 选的 macro_rules! 片段；这里照 cfg 规则求值后拼回去。
# 用法：extract.py 提交 名字前缀 [macos windows linux]，在仓库根目录跑；例如 extract.py v0.2.1 v0.2.1 生成 v0.2.1-macos.toml 等\n# 发版后把新 tag 也提一份进来（见 docs/notes/testing.md）；内容相同的平台合成一个文件
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
PATHS = ('crates/qingjian-platform/src/config/mod.rs', 'crates/qingjian-platform/src/config.rs')


def source(commit):
    for path in PATHS:
        result = subprocess.run(['git', 'show', f'{commit}:{path}'], capture_output=True, text=True)
        if result.returncode == 0:
            return result.stdout
    raise SystemExit(f'{commit} 里找不到配置模块')


def cfg_matches(predicate, platform):
    """求 cfg 表达式：只用到 windows、target_os、not / any / all。"""
    predicate = predicate.strip()
    if predicate == 'windows':
        return platform == 'windows'
    if predicate == 'unix':
        return platform != 'windows'
    match = re.fullmatch(r'target_os\s*=\s*"(\w+)"', predicate)
    if match:
        return match.group(1) == platform
    match = re.fullmatch(r'(not|any|all)\((.*)\)', predicate, re.S)
    if not match:
        raise SystemExit(f'不认识的 cfg：{predicate}')
    parts, depth, current = [], 0, ''
    for char in match.group(2):
        if char == ',' and depth == 0:
            parts.append(current)
            current = ''
            continue
        depth += char == '('
        depth -= char == ')'
        current += char
    if current.strip():
        parts.append(current)
    values = [cfg_matches(part, platform) for part in parts]
    return {'not': lambda: not values[0], 'any': lambda: any(values), 'all': lambda: all(values)}[match.group(1)]()


# 原始字符串字面量 r#"…"#；用命名分组，拼进别的正则时编号不会错位
RAW = r'r(?P<hashes>#*)"(?P<raw>.*?)"(?P=hashes)'


def template(text, platform):
    macros = {}
    for match in re.finditer(r'(?:#\[cfg\((?P<cfg>[^\n]*?)\)\]\s*)?macro_rules!\s*(?P<name>\w+)\s*\{\s*\(\)\s*=>\s*\{\s*' + RAW, text, re.S):
        predicate, name, body = match.group('cfg'), match.group('name'), match.group('raw')
        if predicate is None or cfg_matches(predicate, platform):
            macros[name] = body
    body = re.search(r'pub const TEMPLATE: &str = concat!\((.*?)\n\);', text, re.S)
    if not body:
        # 0.1.1 及以前：整份模板是一个原始字符串，没有按平台分
        single = re.search(r'pub const TEMPLATE: &str = ' + RAW + ';', text, re.S)
        if not single:
            raise SystemExit('找不到 TEMPLATE')
        return single.group('raw')
    pieces = []
    for match in re.finditer(RAW + r'|(?P<macro>\w+)!\(\)|"(?P<plain>(?:[^"\\]|\\.)*)"', body.group(1), re.S):
        if match.group('macro'):
            pieces.append(macros[match.group('macro')])
        elif match.group('plain') is not None:
            pieces.append(match.group('plain').encode().decode('unicode_escape'))
        else:
            pieces.append(match.group('raw'))
    return ''.join(pieces)


def main():
    commit, prefix = sys.argv[1], sys.argv[2]
    text = source(commit)
    # 内容一样的平台合成一份，文件名列出它们：v0.1.4-macos+linux.toml
    groups = {}
    for platform in sys.argv[3:] or ['macos', 'windows', 'linux']:
        groups.setdefault(template(text, platform), []).append(platform)
    for content, platforms in groups.items():
        out = HERE / f'{prefix}-{"+".join(platforms)}.toml'
        out.write_text(f'# 由 extract.py 从 {commit} 还原（{" / ".join(platforms)}），是那一版第一次运行时写给用户的配置模板；不要手改\n' + content)
        print(out.name)


if __name__ == '__main__':
    main()
