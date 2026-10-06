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
        return digest_stream(stream)


def digest_stream(stream):
    value = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b''):
        value.update(chunk)
    return value.hexdigest()


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
    if len(args) not in (4, 5):
        raise SystemExit('install 需要仓库、服务、插件、样例标记及可选框架版本')
    root, server, plugin, sample = args[:4]
    flavor = args[4] if len(args) == 5 else 'fcitx5'
    if flavor not in ('fcitx4', 'fcitx5'):
        raise SystemExit('框架版本必须是 fcitx4 或 fcitx5')
    root = Path(root)
    data = Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share')))
    if not data.is_absolute():
        raise SystemExit('XDG_DATA_HOME 必须是绝对路径')
    files = {prefix / 'share/licenses/qingjian/LICENSE': root / 'LICENSE',
             prefix / 'share/licenses/qingjian/LICENSE-CNS11643.txt': root / 'assets/stroke/LICENSE-CNS11643.txt',
             data / 'icons/hicolor/128x128/apps/qingjian.png': root / 'assets/icon/logo.png',
             prefix / 'bin/qingjian-linux-server': Path(server)}
    if flavor == 'fcitx4':
        config = Path(os.environ.get('XDG_CONFIG_HOME', str(Path.home() / '.config')))
        if not config.is_absolute():
            raise SystemExit('XDG_CONFIG_HOME 必须是绝对路径')
        icon = root / 'assets/icon/fcitx4.png'
        files[prefix / 'lib/fcitx/fcitx-qingjian.so'] = Path(plugin)
        files[config / 'fcitx/addon/fcitx-qingjian.conf'] = root / 'apps/linux/fcitx4/data/addon/fcitx-qingjian.conf'
        files[config / 'fcitx/imicon/qingjian.png'] = icon
        files[data / 'icons/hicolor/48x48/apps/fcitx-qingjian.png'] = icon
    else:
        files[prefix / 'lib/fcitx5/qingjian.so'] = Path(plugin)
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
                source = (root / member.name).resolve()
                if not source.is_relative_to((root / 'data').resolve()):
                    raise SystemExit('数据包路径越界')
                checksum = digest_stream(bundle.extractfile(member))
                if not source.is_file() or digest(source) != checksum:
                    raise SystemExit(f'产品数据校验失败：{member.name}')
                verified[source] = checksum
        wanted = ('dict.qj', 'lm.qj', 'lm-unigram.tsv', 'lm-bigram.tsv', 'english.tsv')
        for source in generated.rglob('*'):
            if source.is_file() and not source.name.startswith('._') and (source.name in wanted or source.name.startswith('glossary-') or source.parent.name in ('dicts', 'codes')):
                if source.resolve() not in verified:
                    raise SystemExit(f'产品数据没有校验记录：{source}')
                files[resources / source.relative_to(root)] = source
        model = root / 'data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm'
        if not model.is_file():
            raise SystemExit('缺少含章·知微 hanzhang-zhiwei-small.qjm，请先运行 tools/release/data-fetch.sh，或用 --sample 体验样例词库')
        if verified.get(model.resolve()) != digest(model):
            raise SystemExit('含章·知微不在已校验的数据包中')
        files[resources / model.relative_to(root)] = model
        p2c_model = root / 'data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm'
        if not p2c_model.is_file() or verified.get(p2c_model.resolve()) != digest(p2c_model):
            raise SystemExit('含章·通变缺失或不在已校验的数据包中')
        files[resources / p2c_model.relative_to(root)] = p2c_model
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
        elif flavor == 'fcitx4' and target == config / 'fcitx/addon/fcitx-qingjian.conf':
            temporary.write_text(temporary.read_text().replace('Library=fcitx-qingjian.so\n', f'Library={prefix}/lib/fcitx/fcitx-qingjian.so\n'))
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
