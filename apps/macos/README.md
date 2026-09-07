# qingjian-macos

macOS 输入法壳（InputMethodKit）。按键进 Core 的 `Engine`，候选画在自绘 NSPanel；源码按 `app / host / imk / candidates / menubar / preferences` 分目录，
架构见 `docs/design/architecture.md`「macOS：IMK」。

## 开发安装

```bash
apps/macos/scripts/bundle.sh --install
```

打包到 `target/Qingjian.app` 并装到 `~/Library/Input Methods/`，杀掉旧进程；切换一次输入法就会拉起新的。
首次要在「系统设置 → 键盘 → 输入法 → 编辑 → +」的简体中文下添加「青简」，列表里没有就注销再登录。
日志在 `~/Library/Logs/Qingjian/`，用户数据与 `config.toml` 在 `~/Library/Application Support/Qingjian/`。

## 分发 pkg

```bash
apps/macos/scripts/bundle.sh --pkg        # target/pkg/Qingjian-<版本>.pkg
```

pkg 装到 `/Library/Input Methods/`（需要管理员密码），装完 postinstall 以登录用户身份跑 `qingjian-macos --register`（注册、启用并切成当前输入源），
输入源自动注册并启用，不用手动添加。版本号取 workspace `Cargo.toml`，构建号是提交数。
机器上同时有 `~/Library` 的开发副本时两份会互相顶，先跑 `scripts/uninstall.sh`。

签名与公证靠环境变量，没设就 ad-hoc 签 `.app`、pkg 不签（测试者首次打开要在「系统设置 → 隐私与安全性」里点「仍要打开」）：

| 变量 | 作用 |
|---|---|
| `QINGJIAN_SIGN_IDENTITY` | `Developer ID Application: …`，给 `.app` 签名并开 hardened runtime |
| `QINGJIAN_INSTALLER_IDENTITY` | `Developer ID Installer: …`，给 pkg 签名 |
| `QINGJIAN_NOTARY_PROFILE` | `xcrun notarytool store-credentials` 存的 profile 名，设了就公证并 staple |

二进制只有本机架构（`hostArchitectures` 按 `uname -m` 写进 Distribution），Intel 机器要另打一份。

## 卸载

```bash
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh          # 保留学习数据与配置
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge  # 连数据、配置、日志一起删
```

仓库里的 `apps/macos/scripts/uninstall.sh` 是同一个文件，两处安装位置都会清。
