| `.github/workflows/ci.yml` | push main、PR | `core`（Linux）fmt / clippy / 全 workspace 测试（排除 IMK 壳）；`macos` 编 IMK 壳并跑它的测试；`windows` 编 Server / TSF DLL / Settings 并跑测试。仓库公开，Actions 不计费 |
| `.github/workflows/audit.yml` | 每周一、Cargo.lock 变动 | `cargo audit`（RustSec 已知漏洞） |
| `.github/dependabot.yml` | 每周一 | Cargo 依赖与钉 commit 的 actions 的更新 PR |# 发版流程

2026-09-07 搭起来的：GitHub Actions 按标签打包、建 Release、生成官网下载页用的 `releases.json`。
这里记怎么发一版、各环节的依赖，以及官网怎么消费产物。

## 一次发版做什么

0.1.4 起三个平台共用版本号、共用一个 GitHub Release（标签 `v<版本>`）：同一个版本号、同一份更新日志只出现一次，三个平台的安装包都挂在上面。

1. 改版本号，把 `-dev` 去掉：`apps/macos/Cargo.toml`（Info.plist 与 pkg 文件名从这里取）、`apps/windows/{server,tsf,settings}/Cargo.toml`（三个一起改，打包读 `server`）、
   `apps/linux/server/Cargo.toml`，然后 `cargo update --workspace --offline` 同步 `Cargo.lock`。
   **发版之间版本号一直带 `-dev`**（Rust nightly / Firefox Nightly 那套）：Cargo.toml 写 `0.1.5-dev`，打包脚本再接上 git 短哈希，
   本地装的、CI 中间构建的都显示 `0.1.5-dev-1a2b3c4`（工作区有改动加 `+`），测试时一眼知道装的是哪个提交；版本号干净的一定是线上包；带 `-dev` 的标签 CI 直接拒绝。
   pkg 的 `--version`、Inno 的 `VersionInfoVersion` 只认数字点号，`bundle.sh` / `build.ps1` 去掉后缀再传，文件名保留完整版本。
2. `CHANGELOG.md` 顶上那一节把「未发布」换成日期：`## <版本> · <日期> · <渠道>`，一行一条、面向用户的措辞；只属于一个平台的条目加「macOS：」「Windows：」「Linux：」前缀。
   渠道是**更新渠道**（2026-09-17 定）：定期发的版本标 `stable`、版本号干净（`0.1.4`）；中间放给测试者的版本标 `alpha` / `beta` / `rc`，
   版本号带同名预发布后缀（`0.1.5-beta.1`），`release.yml` 见到后缀就把 GitHub Release 标成预发布、不抢 latest。
   「产品还在测试期」由 0.x 的版本号表达，不占渠道字段；0.1.2 及更早的条目标的 `beta` 是旧含义，不回改。
   **更新日志手写，不由提交自动生成**：发版前按上个标签以来的 `git log` 起草几条，人审一遍再定稿。
   标题下可以写一行 `网盘：[夸克网盘](链接)`（GitHub 下载不方便的用户用）：它不算更新条目，Release 说明把它放在最前面，`releases.json` 单独输出成 `mirrors`，官网显示成按钮。
   网盘文件在 CI 跑完后手动上传。
3. 提交，打注释标签并推：`git tag -a v0.1.4 -m "青简 0.1.4" && git push origin main v0.1.4`。
4. 标签推出去之后紧接一个普通提交把版本号改成下一个开发版（`0.1.5-dev`），CHANGELOG 顶上加 `## 0.1.5 · 未发布 · stable`；-dev 版本永远没有标签与 Release。

`release.yml` 的流程：

- `prepare`（`ubuntu-26.04`）：门禁（要打的平台版本号都与标签一致、不带 `-dev`、标签在 main 上、配了索引签名密钥）→ 用 CHANGELOG 那一节加各平台未签名提示建**草稿** Release。
- `macos`（`macos-26`）、`windows`（`windows-latest`）、`linux`（`ubuntu-26.04`）并行：下载产品数据 → 打包 → `tools/release/upload-build.sh` 把安装包和
  这个平台的 `SHA256SUMS-<平台>`、`build-info-<平台>.json` 片段传到草稿上。
- `publish`：三个平台都成功才跑，`tools/release/finish-release.sh` 把片段合成一份 `SHA256SUMS` 与 `build-info.json`（各平台构建机记在 `platforms` 里）、删掉片段、草稿转正
  → `publish-releases-json.sh` 生成并签名 `releases.json` → `bump-website.sh` 触发官网构建。
  任何平台失败，Release 就停在草稿、用户看不到；修好后删掉草稿与标签重推，或在 Actions 页重跑失败的 job。

**单平台补丁**（只修一个平台的急事）：只改那个平台的版本号，打 `macos-v<版本>` / `windows-v<版本>` / `linux-v<版本>`，同一套流程只打那一个平台，Release 标题带平台名。

发版后 GitHub Release 上有：`qingjian-<版本>-macos-arm64.pkg`、`-macos-x86_64.pkg`、`-windows-x86_64-setup.exe`、`-linux-x86_64.tar.gz`、`SHA256SUMS`、`build-info.json`、`releases.json` 与 `.sig`。

官网由 Cloudflare Workers Builds 按官网仓库的提交自动构建，没有可调用的构建钩子，所以主仓库靠**往官网仓库推一个小提交**来触发：
`tools/release/bump-website.sh` 把版本标签与文档提交号写进官网的 `src/content/upstream.json` 并提交推送（提交者 qingjian-ci）。
配了 `QINGJIAN_WEB_TOKEN`（对 qingjian-web 有 Contents: read and write 的 fine-grained PAT）release.yml 末尾自动做；
官网文档只随发版更新（`docs/user/` 平时改动不推官网，免得文档领先于用户装到的版本）。没配就在官网仓库随便提交一次（或本地跑这个脚本）。
官网构建时才拉最新 Release 的 `releases.json` 与主仓库 `docs/user`，所以提交内容本身不重要，`upstream.json` 只是留个记录、顺便让文档按记下的提交号拉（版本对得上）。

Rust 工具链由 `rust-toolchain.toml` 钉版本（现在 1.96.0），两个 workflow 里 `dtolnay/rust-toolchain` 的 `toolchain:` 输入写同一个号；升级 Rust 时三处一起改。

## 各平台的特别之处

- **Windows**：没有代码签名证书时 workflow 设 `QINGJIAN_UIACCESS=0`：没签名的 exe 带 uiAccess=true 起不来。
  代价是候选窗在任务栏搜索 / 设置这类 UWP 宿主里可能被盖住，用户文档与 CHANGELOG 已列为已知问题。
  Certum 开源证书办下来后：在 `build.ps1` 加 signtool 一步（`sign-local.ps1` 是本机自签的参考），workflow 去掉那个环境变量；SmartScreen 对无签名安装包的拦截也一并消失。
  Inno Setup 钉 7.1.0（与开发机同版本、钉死 GitHub Release 上的安装程序与哈希）。
- **Linux**：`apps/linux/scripts/package.sh` 打的包里是编好的 Server 与 Fcitx5 插件、产品数据压缩包、`install.sh` / `uninstall.sh` / `files.py`，按仓库相对路径放，
  安装与源码安装走同一份清单与校验；打完先装进临时目录冒烟测试（文件齐、校验过、`ldd` 无缺库）再上传。
  插件链接构建机的 Fcitx5（`build-info.json` 的 `platforms.linux.runner` 记了版本），所以只承诺 Ubuntu 26.04，其他发行版走源码安装。
- **官网**：安装包按文件名识别平台（`ASSET_KINDS`），下载页按访问者平台取「有该平台安装包的最新版本」（`latestFor`）；0.1.3 及更早按平台分开的 Release 照样识别，同版本号合在一条里。

## 提交前检查与 CI

本地 `git config core.hooksPath .githooks` 启用一次后，每次提交前 `.githooks/pre-commit` 先拒绝装饰性分隔注释（`// ====` / `// ────`，只做视觉分组不带「为什么」），再跑 `cargo fmt --check` 与 `cargo clippy -D warnings`（含 IMK 外壳，增量几十秒）；
`.githooks/pre-push` 在推之前跑全 workspace 测试。外部 PR 走同一套 `ci.yml`，不过不合。

供应链：workflow 里的 actions 一律钉到 commit（注释写对应标签），`.github/dependabot.yml` 每周一提 Cargo 与 actions 的更新 PR；`audit.yml` 每周与 Cargo.lock 变动时跑 `cargo audit`；
cargo 命令全 `--locked`（含 `bundle.sh` 与 `build.ps1`）。普通 CI 只有 `contents: read`，checkout 不留凭据；release 的 secrets 不放顶层 env，只注入用它的那一步。

发版门禁（`release.yml` 第一步）：版本号与标签一致且不带 `-dev`；标签指向的提交必须在 `main` 上（`git merge-base --is-ancestor`）；产品数据下载后按 `data` Release 的 `SHA256SUMS` 校验，摘要写进 `build-info.json` 的 `data_sha256`。
**别给这条门禁开逃生口。** 2026-10-10 有个外部 PR（[#522](https://github.com/qingjian-team/qingjian/pull/522)）想加 `workflow_dispatch` 手动触发，并带一个 `version_override` 输入「在误打标签时手动纠正版本号」。这条门禁存在的意义就是让 Release 版本号、git 标签、各平台 `Cargo.toml` 的 `version` 三者强制对齐；任何能从外部输入改写版本号的入口，都等于把这道门禁变成可选项——`dry_run` 同理，若它没真正停住 `gh release create`，就只是「看起来检查了」。要重发就走「改版本号 → 重新打标签」的既有路径。该 PR 已被关闭，理由见 [`ci-enhancement-declined.md`](ci-enhancement-declined.md)。
**正式版前还欠**：产品数据改成不可变 tag 并在仓库里锁定版本（现在滚动覆盖，同一源码 tag 重跑可能拿到不同数据）、安装包内容验证（词库 / 模型 / 许可齐不齐、签名校验）。

## 两个 workflow

| 文件 | 触发 | 做什么 |
|---|---|---|
| `.github/workflows/ci.yml` | push main、PR | Linux 上 `cargo fmt --check` / clippy / test，排除 `qingjian-macos`（IMK 外壳只能在 macOS 编译，macOS runner 计费是 Linux 的 10 倍） |
| `.github/workflows/release.yml` | 推 `v*`（三个平台）或 `macos-v*` / `windows-v*` / `linux-v*`（单平台）标签 | `prepare` 门禁并建草稿 Release → `macos` / `windows` / `linux` 并行打包上传 → `publish` 合并摘要、转正、生成 `releases.json`、触发官网构建（见上文） |

## 产品数据从哪来

词库、语言模型、释义表（`data/generated/*.qj`、`dicts/*.qj`、英文词表）不在 git 里，体积约 90 MB 且由本机数据管道生成。
它们发在仓库里一个个**不可变**的预发布 Release 上：`data-v1`、`data-v2`……每次数据重生成发一个新号、从不覆盖
（预发布不会成为 GitHub 的 latest，官网取 latest 时不会拿到它）。仓库里 `tools/release/data.lock` 钉住当前要用的标签与资产的 SHA-256，
跟用到新数据的代码同一个提交进去：checkout 哪个提交就拿到它对应的那版数据，离线自编译的人不会因为我们改了数据而编出坏包。

- `tools/release/data-bundle.sh`：把 `data/generated/`、含章·知微 `hanzhang-zhiwei-small.qjm` 与含章·通变 `hanzhang-tongbian-small.qjm` 装进唯一附件 `qingjian-data.tar.gz`
  （导出的三件套放各自模型目录，`tools/release/pack-model.sh` 打成 `.qjm` 容器）。训练中间产物不进入公开包；发到下一个 `data-vN`（`--tag` 可指定，已存在就拒绝），然后改写 `data.lock`。
- `tools/release/data-fetch.sh`：按 `data.lock` 下载（有 gh 用 gh，没有就 curl 直连）、按锁文件里的哈希校验，
  数据包解到 `data/generated/` 与两份模型各自的 `data/models/<标识>/`。旧锁仍可读取分离附件。`release.yml` 三个平台的 job 和离线自编译走同一个脚本；
  `bundle.sh`、`qingjian.iss` 与 Linux 打包脚本将两份模型分别装入对应目录，产品端优先使用通变。
  标签与各资产哈希记进 `build-info.json`（`data_tag` / `data_sha256` / `model_sha256` / `p2c_model_sha256`）。

数据重生成之后（重跑 lexicon / bigram / gloss-gen export）或模型重训之后跑一次 `data-bundle.sh`（三件套比 `.qjm` 新会自动重打），
把锁文件的改动提交（`chore(data): 数据 data-vN`），否则 CI 打的包还是锁文件指的旧数据。模型文件缺失或哈希不符时 CI 会失败，不会静默地发出错数据的包。
2026-09-16 之前用的是滚动覆盖的 `data` Release，已冻结不再更新。

## 版本索引的签名

软件内「检查更新」只认签过名的 `releases.json`（设计见 `docs/design/update.md`）。`publish-releases-json.sh` 生成索引后用 `tools/release-sign` 签出 `releases.json.sig`，
两个文件一起挂到本次 Release 与 GitHub latest；官网构建时原样拷到 `https://qingjian.app/releases.json` 与 `.sig`。

- 私钥是仓库 Secret `QINGJIAN_INDEX_SIGNING_KEY`（base64 的 32 字节）；没配时 `release.yml` 的门禁直接失败。维护者本机留一份在 `~/.config/qingjian/index-signing.key`（不进仓库）。
- 换钥：`cargo run -p qingjian-release-sign -- keygen --out <新文件>`，把打印的公钥加进 `crates/qingjian-update/src/index/signature.rs` 的 `PUBLIC_KEYS`（新旧并列），
  发一两个版本后再换 Secret、去掉旧公钥。直接换 Secret 会让所有已装版本收不到更新。
- 改了 CHANGELOG 后用 `publish-releases-json.sh` 重刷索引同样要带这个环境变量。

## 签名与公证

没有证书时 CI 照样出包（ad-hoc 签名，Release 说明里自动加一句「首次打开要在隐私与安全性里放行」）。
Apple Developer 账号有了以后，在仓库 Secrets 里配齐 `release.yml` 头部注释列的七个值（.p12 与 .p8 都 base64），
下一次发版就是签名 + 公证 + 钉票据的包，用户下载双击即装。`bundle.sh` 本身通过 `QINGJIAN_SIGN_IDENTITY` /
`QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE` 三个环境变量工作，本机有证书也能这样打。

## releases.json：官网下载页的数据源

`tools/release/releases_json.py` 从 `CHANGELOG.md`（日期、渠道、更新日志）、GitHub Releases API（附件、地址、大小）
与每次发布的 `SHA256SUMS` / `build-info.json`（每个包的 sha256、提交哈希、构建时间、工具链）生成，挂在每个版本的 Release 上；官网固定取
`https://github.com/<repo>/releases/latest/download/releases.json`（仓库私有期间要带令牌走 API 下载附件）。

结构对应官网 `src/lib/releases.ts` 里的 `Release` / `Asset` 类型：

```json
{
  "schema_version": 1,
  "generated": "2026-09-07T12:00:00Z",
  "repository": "owner/qingjian",
  "latest": "0.1.3",
  "releases": [
    {
      "version": "0.1.3",
      "date": "2026-09-18",
      "channel": "stable",
      "notes": ["整句输入：……", "候选旁有词性和译词……"],
      "mirrors": [{ "name": "夸克网盘", "url": "https://pan.quark.cn/s/…" }],
      "commit": "869ad00…（40 位）",
      "built_at": "2026-09-07T08:38:12Z",
      "toolchain": "rustc 1.96.0 (ac68faa20 2026-05-25)",
      "assets": [
        { "platform": "macos", "arch": "Apple Silicon", "cpu": "arm64", "file": "qingjian-0.1.3-macos-arm64.pkg",
          "url": "https://github.com/owner/qingjian/releases/download/macos-v0.1.3/qingjian-0.1.3-macos-arm64.pkg",
          "size": 35989277, "sha256": "…" },
        { "platform": "macos", "arch": "Intel", "cpu": "x86_64", "file": "qingjian-0.1.3-macos-x86_64.pkg", "url": "…", "size": 36172871, "sha256": "…" }
      ]
    }
  ]
}
```

- `releases` 从新到旧，`latest` 是第一条的版本号；官网「当前版本」取它，历史版本列表就是整个数组。
- `channel` 是 `alpha` / `beta` / `rc` / `stable`，显示成什么字由官网定；`commit` / `built_at` / `sha256` 给用户核对下载的包，下载页应显示 sha256 与提交短哈希。
- 安装包文件名固定为 `qingjian-<版本>-<平台>-<cpu>[-setup].<扩展名>`（全小写；平台 `macos` / `windows` / `linux`，cpu `arm64` / `x86_64`，2026-09-17 定，
  包管理器的地址模板与检查更新都靠它稳定）。平台与架构由文件名判定（`ASSET_KINDS`，同时认 0.1.2 及更早的 `Qingjian-<版本>-arm64.pkg` / `-Setup.exe` 旧名），
  `arch` 给人看（Apple Silicon / Intel / x64），`cpu` 给程序比对。
- `schema_version` 现在是 1：只加字段不用动，改了已有字段的含义或结构才加一。
- `mirrors` 来自 CHANGELOG 那一节的「网盘：」行，没有就是空数组；官网在下载按钮旁单独显示。
- `SHA256SUMS` 与 `releases.json` 自己不列进 `assets`。
- 官网侧要做的：构建时下载这个文件替代手写的 `releases` 数组（与拉 `docs/user` 的 `sync-docs.mjs` 同一处、同一个令牌），
  `downloadsOpen` 开关仍由官网自己控制。

## 本机打包

`apps/macos/scripts/bundle.sh --pkg` 打本机架构；`QINGJIAN_TARGET=x86_64-apple-darwin` 交叉编译 Intel 包（要先 `rustup target add`，
本机不需要时不必装，CI 上两个都打）。成品在 `target/pkg/qingjian-<版本>-macos-<arch>.pkg`，每个架构一个工作目录，连着打互不覆盖。
