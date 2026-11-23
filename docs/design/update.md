# 检查更新

2026-09-17 定。0.1.3 起只做「检查 + 提示 + 打开下载页」，下载与安装仍交给系统安装程序，输入法不替换自己。

## 目标与边界

- 用户不用再从群里拿包：发了新版，已装的青简一天内自己知道。测试者把渠道切到「测试版」就能收到 beta / rc。
- 输入法不替换自己：只提示，点了打开 [下载页](https://qingjian.app/download)，安装仍由系统安装程序完成。
- 更新通道等于远程代码执行通道，所以索引从第一版起就签名；客户端只认签过名的索引。
- 隐私：请求是对静态文件的 GET，不带账号、设备标识或输入内容；`[update] check = false` 关掉后不联网。

## 数据流

```
发版（release.yml）
  releases_json.py → releases.json
  qingjian-release-sign sign → releases.json.sig      私钥：CI 密钥 QINGJIAN_INDEX_SIGNING_KEY
  两个文件挂到本次 Release 与 GitHub latest
官网构建（qingjian-web scripts/sync-releases.mjs）
  原样拷到 static/ → https://qingjian.app/releases.json 与 .sig
客户端（crates/qingjian-update）
  下载两者 → ed25519 验签 → 解析 → 按平台 / CPU / 渠道挑比当前新的最新版 → 写 update.json
```

索引放官网域名而不是直连 GitHub：国内访问 GitHub 不稳，官网是 Cloudflare 上的静态站（非浏览器 User-Agent 实测不被拦）。
签名是对文件字节的分离签名，所以官网必须原样拷贝，不能经 JSON 重排。

## 渠道与版本

- 索引里每个版本的 `channel` 是更新渠道：定期发的版本 `stable`，中间放给测试者的 `alpha` / `beta` / `rc`（版本号带同名预发布后缀）。
- 客户端两档：`stable` 只看 `stable`；`beta` 看全部，取版本号最大的，所以测试版用户在正式版出来后会升到正式版。
- 版本按语义化版本比较（`0.1.4-beta.2 < 0.1.4-rc.1 < 0.1.4`）。
- 本地开发包（`0.1.3-dev-<哈希>`）不检查。调试用环境变量：`QINGJIAN_UPDATE_VERSION` 顶替当前版本，`QINGJIAN_UPDATE_INDEX` 换索引地址（签名照验）；
  `cargo run -p qingjian-update --example check -- 0.1.2 beta` 手动走一遍。
- 只有带本机安装包（`platform` + `cpu`）的版本才算数：只发了 macOS 的版本不会提示 Windows 用户。

## 调度

壳在已有的每秒定时器里调 `Checker::poll`：开关开着、上次成功超过 24 小时（或换了渠道）、上次尝试超过 1 小时、没有正在查的，才起一个一次性线程去查。
失败（断网、验签不过、格式版本不认识）只记日志。结果落在数据目录的 `update.json`，装上新版后旧结果因为「不比当前新」自然失效。

- macOS 菜单的坑（2026-09-17）：「有新版本」这一行是可点的条目，必须放在「打开日志目录」那一组里。第一版把它放在最后一条分隔线之后、
  夹在「配置文件有错误」与「青简 x.y.z」两个纯展示条目之间，结果 IMK 每次按键后都 `deactivateServer` 再新建控制器，
  第二个字母盖掉第一个、候选框闪；与条目是否隐藏、标题、tag 都无关，挪个位置或去掉 action 就好（对照构建逐个验过）。
- macOS：IMK 进程自己查；菜单里「有新版本 x.y.z…」一行与「关于」页的状态行，状态变了才刷界面。
- Windows：Server 查并写 `update.json`（DLL 不联网）；设置程序的「关于」页读这个文件，「立即检查」在设置程序自己的后台线程里查。
  任务栏「中 / 英」图标的右键菜单在查到新版本时多一项「有新版本，前往下载…」：Server 随 `ModeSync` 下发 `IndicatorState.update_available`，
  点了发 `IndicatorCommand::OpenDownload`，由 Server 打开下载页（DLL 可能在 UWP 沙箱里起不了进程）。

## 签名密钥

`tools/release-sign`：`keygen --out <文件>` 生成密钥对（私钥只写文件），`sign <文件>` 按环境变量里的私钥写 `.sig`，`verify <文件>` 按内置公钥验。
公钥在 `crates/qingjian-update/src/index/signature.rs` 的 `PUBLIC_KEYS`，是个列表：换钥时新旧并列发一两个版本，等旧版本用户都升上来再去掉旧的。
`sign` 会先核对私钥与内置公钥对得上，对不上当场失败；`release.yml` 的门禁在没配私钥时直接失败——签不了名的索引等于所有用户收不到更新。

## 以后

- 下载 + 校验 sha256（索引里已有）+ 调起安装程序（macOS 打开 pkg，Windows 静默跑安装包）。
- 系统通知里的提示。
- SignPath 要求「向非用户指定的系统发数据」的功能在安装时可关：安装向导里加一个「自动检查更新」勾选项。
