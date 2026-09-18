#!/usr/bin/env python3
# //! 安装文件清单与校验；不管理桌面服务、系统文件或用户数据。
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tarfile


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    action, prefix_text, *args = sys.argv[1:]
    prefix = Path(prefix_text)
    if not prefix.is_absolute() or prefix == Path('/'):
        raise SystemExit('需要非根绝对安装路径')
    prefix = prefix.resolve()
    manifest = prefix / 'share/qingjian/install-manifest.json'
    old = json.loads(manifest.read_text()) if manifest.is_file() else {}
    if action == 'uninstall':
        for filename, checksum in old.items():
            path = Path(filename)
            if path.is_file() and not path.is_symlink() and digest(path) == checksum:
                path.unlink()
            elif path.exists():
                print(f'保留已修改文件：{path}')
        manifest.unlink(missing_ok=True)
        return
    root, server, plugin, sample = args
    root = Path(root)
    data = Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share')))
    if not data.is_absolute():
        raise SystemExit('XDG_DATA_HOME 必须是绝对路径')
    files = {prefix / 'share/licenses/qingjian/LICENSE': root / 'LICENSE',
             data / 'icons/hicolor/128x128/apps/qingjian.png': root / 'assets/icon/logo.png',
             prefix / 'bin/qingjian-linux-server': Path(server),
             prefix / 'lib/fcitx5/qingjian.so': Path(plugin)}
    for kind in ('addon', 'inputmethod'):
        files[data / f'fcitx5/{kind}/qingjian.conf'] = root / f'apps/linux/fcitx5/data/{kind}/qingjian.conf'
    resources = prefix / 'share/qingjian/resources'
    for kind in ('sample', 'glossary', 'levels', 'emoji'):
        for source in (root / 'assets' / kind).rglob('*'):
            if source.is_file():
                files[resources / source.relative_to(root)] = source
    if sample != 'true':
        generated = root / 'data/generated'
        dictionary = generated / 'dict.qj'
        archive = root / 'target/release-data/qingjian-data.tar.gz'
        lock = dict(line.strip().split(' = ', 1) for line in (root / 'tools/release/data.lock').read_text().splitlines() if ' = ' in line)
        if not dictionary.is_file() or not archive.is_file():
            raise SystemExit('缺少产品数据，请先运行 tools/release/data-fetch.sh，或用 --sample 体验样例词库')
        if digest(archive) != lock.get('qingjian-data.tar.gz'):
            raise SystemExit('产品数据包与 tools/release/data.lock 校验值不符')
        verified = {}
        with tarfile.open(archive) as bundle:
            for member in bundle.getmembers():
                if not member.isfile():
                    continue
                source = (generated / member.name).resolve()
                if not source.is_relative_to(generated.resolve()):
                    raise SystemExit('数据包路径越界')
                checksum = hashlib.file_digest(bundle.extractfile(member), 'sha256').hexdigest()
                if not source.is_file() or digest(source) != checksum:
                    raise SystemExit(f'产品数据校验失败：{member.name}')
                verified[source] = checksum
        wanted = ('dict.qj', 'lm.qj', 'lm-unigram.tsv', 'lm-bigram.tsv', 'english.tsv')
        for source in generated.rglob('*'):
            if source.is_file() and (source.name in wanted or source.name.startswith('glossary-') or source.parent.name == 'dicts'):
                if source.resolve() not in verified:
                    raise SystemExit(f'产品数据没有校验记录：{source}')
                files[resources / source.relative_to(root)] = source
    # 安装前先检查所有目标，避免覆盖其他来源的同名文件。
    for target in files:
        if target.is_symlink() or (target.exists() and (str(target) not in old or digest(target) != old[str(target)])):
            raise SystemExit(f'目标已存在且不属于本次安装：{target}')
    installed = {}
    for target, source in files.items():
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + '.qingjian-tmp')
        shutil.copy2(source, temporary)
        if target == data / 'fcitx5/addon/qingjian.conf':
            temporary.write_text(temporary.read_text().replace('Library=qingjian\n', f'Library={prefix}/lib/fcitx5/qingjian\n'))
        temporary.replace(target)
        installed[str(target)] = digest(target)
    for filename, checksum in old.items():
        path = Path(filename)
        if filename not in installed and path.is_file() and not path.is_symlink() and digest(path) == checksum:
            path.unlink()
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps(installed, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
