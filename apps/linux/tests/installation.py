#!/usr/bin/env python3
# //! 临时用户目录真实文件安装：重复安装、路径登记、校验拒绝与保留用户文件。
import json
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
    plugin = directory / 'plugin'
    server.write_bytes(b'test binary\n')
    plugin.write_bytes(b'test plugin\n')
    server.chmod(0o755)
    env = dict(os.environ, XDG_DATA_HOME=str(data))
    args = ['python3', str(helper), 'install', str(prefix), str(root), str(server), str(plugin), 'true']
    subprocess.run(args, env=env, check=True)
    subprocess.run(args, env=env, check=True)
    addon = data / 'fcitx5/addon/qingjian.conf'
    assert f'Library={prefix}/lib/fcitx5/qingjian\n' in addon.read_text()
    assert (prefix / 'bin/qingjian-linux-server').stat().st_mode & 0o111
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
    user = data / 'qingjian/user.tsv'
    user.parent.mkdir(parents=True, exist_ok=True)
    user.write_text('学习数据')
    other = data / 'fcitx5/addon/other.conf'
    other.write_text('其他输入法')
    modified = prefix / 'share/qingjian/resources/assets/sample/dict.tsv'
    modified.write_text('用户修改过的样例')
    failed = subprocess.run(args, env=env, capture_output=True)
    assert failed.returncode != 0
    subprocess.run(['python3', str(helper), 'uninstall', str(prefix)], env=env, check=True)
    subprocess.run(['python3', str(helper), 'uninstall', str(prefix)], env=env, check=True)
    assert not addon.exists() and not (prefix / 'bin/qingjian-linux-server').exists()
    assert user.read_text() == '学习数据' and other.read_text() == '其他输入法'
    assert modified.read_text() == '用户修改过的样例'
print('用户目录安装、重复安装、注册路径和卸载保留测试通过')
