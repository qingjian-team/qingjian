#!/usr/bin/env python3
# //! 临时用户目录真实文件安装：重复安装、路径登记、校验拒绝与保留用户文件。
import json
import gzip
import importlib.util
import os
from pathlib import Path
import struct
import subprocess
import tempfile

root = Path(__file__).resolve().parents[3]
helper = root / 'apps/linux/scripts/files.py'
with tempfile.TemporaryDirectory(prefix='qingjian-install-') as temporary:
    directory = Path(temporary)
    prefix = directory / 'prefix with spaces'
    data = directory / 'user data'
    server = directory / 'server'
    settings = directory / 'settings'
    plugin = directory / 'plugin'
    server.write_bytes(b'test binary\n')
    settings.write_bytes(b'test settings\n')
    plugin.write_bytes(b'test plugin\n')
    server.chmod(0o755)
    settings.chmod(0o755)
    env = dict(os.environ, XDG_DATA_HOME=str(data))
    args = ['python3', str(helper), 'install', str(prefix), '--root', str(root),
            '--server', str(server), '--settings', str(settings), '--plugin', str(plugin), '--sample']
    subprocess.run(args, env=env, check=True)
    subprocess.run(args, env=env, check=True)
    addon = data / 'fcitx5/addon/qingjian.conf'
    assert f'Library={prefix}/lib/fcitx5/qingjian\n' in addon.read_text()
    assert (prefix / 'bin/qingjian-linux-server').stat().st_mode & 0o111
    desktop = data / 'applications/qingjian-settings.desktop'
    assert f'Exec={prefix}/bin/qingjian-settings\n' in desktop.read_text()
    # hicolor 目录名必须与 PNG 实际像素一致：1024px 源图塞进 128x128 会让不缩放的托盘（Waybar）显示巨图（#547）。
    for size in (16, 22, 24, 32, 48, 64, 128, 256):
        icon = data / f'icons/hicolor/{size}x{size}/apps/qingjian.png'
        assert icon.is_file()
        header = icon.read_bytes()[:24]
        assert header[:8] == b'\x89PNG\r\n\x1a\n'
        width, height = struct.unpack('>II', header[16:24])
        assert (width, height) == (size, size), f'{icon} 实际尺寸 {width}x{height}'
    manifest = json.loads((prefix / 'share/qingjian/install-manifest.json').read_text())
    assert all(Path(filename).is_file() for filename in manifest)
    packaged = directory / 'packaged resources'
    subprocess.run(['python3', str(helper), 'resources', str(packaged), '--root', str(root), '--sample'],
                   env=env, check=True)
    assert (packaged / 'assets/sample/dict.tsv').is_file()
    assert not (packaged / 'assets/schemes').exists()
    user = data / 'qingjian/learning/user.tsv'
    user.parent.mkdir(parents=True, exist_ok=True)
    user.write_text('学习数据')
    user_scheme = data / 'qingjian/custom/schemes/custom/table.tsv'
    user_scheme.parent.mkdir(parents=True)
    user_scheme.write_text('自定义方案')
    other = data / 'fcitx5/addon/other.conf'
    other.write_text('其他输入法')
    modified = prefix / 'share/qingjian/resources/assets/sample/dict.tsv'
    modified.write_text('用户修改过的样例')
    failed = subprocess.run(args, env=env, capture_output=True)
    assert failed.returncode != 0
    subprocess.run(['python3', str(helper), 'uninstall', str(prefix)], env=env, check=True)
    subprocess.run(['python3', str(helper), 'uninstall', str(prefix)], env=env, check=True)
    assert not addon.exists() and not (prefix / 'bin/qingjian-linux-server').exists()
    assert not desktop.exists() and not (prefix / 'bin/qingjian-settings').exists()
    assert user.read_text() == '学习数据' and other.read_text() == '其他输入法'
    assert user_scheme.read_text() == '自定义方案'
    assert modified.read_text() == '用户修改过的样例'
    fcitx4_prefix = directory / 'fcitx4 prefix'
    config = directory / 'user config'
    fcitx4_env = dict(env, XDG_CONFIG_HOME=str(config))
    fcitx4_args = ['python3', str(helper), 'install', str(fcitx4_prefix), '--root', str(root),
                   '--server', str(server), '--settings', str(settings), '--plugin', str(plugin),
                   '--sample', '--flavor', 'fcitx4']
    subprocess.run(fcitx4_args, env=fcitx4_env, check=True)
    subprocess.run(fcitx4_args, env=fcitx4_env, check=True)
    fcitx4_addon = config / 'fcitx/addon/fcitx-qingjian.conf'
    assert f'Library={fcitx4_prefix}/lib/fcitx/fcitx-qingjian.so\n' in fcitx4_addon.read_text()
    assert (fcitx4_prefix / 'lib/fcitx/fcitx-qingjian.so').read_bytes() == plugin.read_bytes()
    imicon = config / 'fcitx/imicon/qingjian.png'
    tray_icon = data / 'icons/hicolor/48x48/apps/fcitx-qingjian.png'
    assert imicon.read_bytes() == tray_icon.read_bytes() == (root / 'assets/icon/fcitx4.png').read_bytes()
    subprocess.run(['python3', str(helper), 'uninstall', str(fcitx4_prefix)],
                   env=fcitx4_env, check=True)
    assert not fcitx4_addon.exists()
    assert not imicon.exists() and not tray_icon.exists()
spec = importlib.util.spec_from_file_location('qingjian_files', helper)
files_helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(files_helper)
with tempfile.TemporaryDirectory(prefix='qingjian-packed-wubi-') as temporary:
    fixture = Path(temporary)
    cache = fixture / 'target/release-data/dicts/wubi86-v1.0.0.qjz'
    source = fixture / 'data/generated/schemes/wubi86.qj'
    lock = fixture / 'tools/release/dicts.lock'
    for path in (cache, source, lock):
        path.parent.mkdir(parents=True, exist_ok=True)
    source.write_bytes(b'packed wubi fixture')
    cache.write_bytes(gzip.compress(source.read_bytes(), mtime=0))
    lock.write_text(f'wubi86-v1.0.0 = {files_helper.digest(cache)} schemes/wubi86.qj\n')
    resources = fixture / 'resources'
    files = {}
    files_helper.bundled_dicts(fixture, resources, files)
    assert files == {resources / 'data/generated/schemes/wubi86.qj': source}
    source.write_bytes(b'tampered code table')
    try:
        files_helper.bundled_dicts(fixture, resources, {})
    except SystemExit:
        pass
    else:
        raise AssertionError('篡改后的随包五笔码表没有被拒绝')
print('用户目录安装、重复安装、注册路径、打包五笔校验和卸载保留测试通过')
