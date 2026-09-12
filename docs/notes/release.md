| `.github/workflows/ci.yml` | push main、PR | `core`（Linux）fmt / clippy / 全 workspace 测试（排除 IMK 壳）；`macos` 编 IMK 壳并跑它的测试；`windows` 编 Server / TSF DLL / Settings 并跑测试。仓库公开，Actions 不计费 |
| `.github/workflows/audit.yml` | 每周一、Cargo.lock 变动 | `cargo audit`（RustSec 已知漏洞） |
| `.github/dependabot.yml` | 每周一 | Cargo 依赖与钉 commit 的 actions 的更新 PR |# 发版流程

2026-09-07 搭起来的：GitHub Actions 按标签打包、建 Release、生成官网下载页用的 `releases.json`。
这里记怎么发一版、各环节的依赖，以及官网怎么消费产物。

## 一次发版做什么

（下面以 macOS 为例；Windows 见「Windows 发版」一节，步骤同构。）

1. 改 `apps/macos/Cargo.toml` 的 `version`（`apps/macos` 的 Info.plist 版本号从这里取，pkg 文件名也是）：把 `0.1.2-dev` 改成 `0.1.2`。
   **发版之间版本号一直带 `-dev`**（Rust nightly / Firefox Nightly 那套）：本地装的、CI 中间构建的都显示 `0.1.2-dev`，版本号干净的一定是线上包；
   带 `-dev` 的标签 CI 直接拒绝。pkg 的 `--version` 与 `distribution.xml` 只认数字点号，`bundle.sh` 去掉后缀再传，Info.plist 与 pkg 文件名保留完整版本。
   **各平台壳版本号独立**：macOS 的版本只在 `apps/macos/Cargo.toml`，跟 workspace 与其他壳无关（例：mac 到 `0.1.1`、win 还在 `0.1.0`）。
2. `CHANGELOG.md` 顶上加一节 `## <版本> · <日期> · <渠道>`（渠道是 `alpha` / `beta` / `rc` / `stable`），一行一条、面向用户的措辞。
   **更新日志手写，不由提交自动生成**：提交信息里有大量内部改动（拆模块、修 RefCell 重入），用户看不懂也不关心；
   做法是发版前按上个标签以来的 `git log` 起草几条，人审一遍再定稿。
3. 提交，打**带平台前缀**的注释标签并推：`git tag -a macos-v0.1.1 -m "青简 macOS 0.1.1" && git push origin main macos-v0.1.1`
   （标签按平台加前缀 `macos-v*` / 将来 `windows-v*`，因为各平台版本号独立、光靠 `v<版本>` 会撞车；旧的 `v*` 标签仍能被官网识别，向后兼容）。
3b. 标签推出去之后紧接一个普通提交把版本号改成下一个开发版（只是改 Cargo.toml，不打标签、不建 Release；-dev 版本永远没有标签与 Release）：`apps/macos/Cargo.toml` 改成 `0.1.3-dev`（Windows 同理 `0.1.0-alpha.3-dev`），本地从此打的包都带 `-dev`。
4. `release.yml` 跑完后 GitHub Release 上有 `Qingjian-<版本>-arm64.pkg`、`Qingjian-<版本>-x86_64.pkg`、`SHA256SUMS`、`build-info.json`（提交、构建时间、工具链）、`releases.json`。
5. 官网由 Cloudflare Workers Builds 按官网仓库的提交自动构建，没有可调用的构建钩子，所以主仓库靠**往官网仓库推一个小提交**来触发：
   `tools/release/bump-website.sh` 把版本标签与文档提交号写进官网的 `src/content/upstream.json` 并提交推送（提交者 qingjian-ci）。
   配了 `QINGJIAN_WEB_TOKEN`（对 qingjian-web 有 Contents: read and write 的 fine-grained PAT）release.yml 末尾自动做；
   `docs/user/` 单独改动推到 main 也会做（`docs.yml`）。没配就在官网仓库随便提交一次（或本地跑这个脚本）。
   官网构建时才拉最新 Release 的 `releases.json` 与主仓库 `docs/user`，所以提交内容本身不重要，`upstream.json` 只是留个记录、
   顺便让文档按记下的提交号拉（版本对得上）。

workflow 会核对 `apps/macos/Cargo.toml` 版本号与标签（去掉 `macos-v` 前缀后）一致，不一致直接失败，避免打出版本号错的包。

Rust 工具链由 `rust-toolchain.toml` 钉版本（现在 1.96.0），两个 workflow 里 `dtolnay/rust-toolchain@master` 的 `toolchain:` 输入写同一个号；升级 Rust 时三处一起改。

## Windows 发版

1. 改 `apps/windows/{server,tsf,settings}/Cargo.toml` 的 `version`（三个一起改；打包脚本与 workflow 读 `server` 那份）。
   同样带 `-dev`：发版之间是 `0.1.0-alpha.2-dev`，发版提交改成 `0.1.0-alpha.2`；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 把整个预发布后缀去掉再传，安装包与 DLL 文件名保留完整版本。
   内测版用 semver 预发布号 `0.1.0-alpha.1`、`0.1.0-alpha.2`…：CHANGELOG 按版本号索引、官网按版本号列条目，
   与 macOS 的 `0.1.0` / `0.1.1` 不能同号；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 会把后缀去掉再传。
2. `CHANGELOG.md` 加一节 `## 0.1.0-alpha.1 · 日期 · alpha`。
3. 打标签 `windows-v0.1.0-alpha.1` 推送。`release.yml` 的 `windows` job 在 `windows-latest` 上：核对版本 → 下载 `data` Release
   → 装 Inno Setup 7.1.0（与开发机同版本，钉死 GitHub Release 的安装程序）→ `build.ps1`→ 建 Release（`Qingjian-<版本>-Setup.exe` + `SHA256SUMS` + `build-info.json`）
   → `publish-releases-json.sh` 生成 `releases.json`，挂到本次发布并覆盖到 GitHub latest 那版上（官网只读 latest 的）。
4. **没有代码签名证书时** workflow 设 `QINGJIAN_UIACCESS=0`：没签名的 exe 带 uiAccess=true 起不来。
   代价是候选窗在任务栏搜索 / 设置这类 UWP 宿主里可能被盖住，用户文档与 CHANGELOG 已列为已知问题。
   Certum 开源证书办下来后：在 `build.ps1` 加 signtool 一步（`sign-local.ps1` 是本机自签的参考），workflow 去掉那个环境变量。
   SmartScreen 对无签名安装包的拦截也一并消失。
5. 官网：`releases.json` 里 Windows 包由文件名 `-Setup.exe` 识别（`ASSET_KINDS`），下载页按访问者平台取「有该平台安装包的最新版本」
   （`latestFor`），所以 macOS 与 Windows 各自的最新版互不干扰。

## 提交前检查与 CI

本地 `git config core.hooksPath .githooks` 启用一次后，每次提交前 `.githooks/pre-commit` 先拒绝装饰性分隔注释（`// ====` / `// ────`，只做视觉分组不带「为什么」），再跑 `cargo fmt --check` 与 `cargo clippy -D warnings`（含 IMK 外壳，增量几十秒）；
`.githooks/pre-push` 在推之前跑全 workspace 测试。外部 PR 走同一套 `ci.yml`，不过不合。

供应链：workflow 里的 actions 一律钉到 commit（注释写对应标签），`.github/dependabot.yml` 每周一提 Cargo 与 actions 的更新 PR；`audit.yml` 每周与 Cargo.lock 变动时跑 `cargo audit`；
cargo 命令全 `--locked`（含 `bundle.sh` 与 `build.ps1`）。普通 CI 只有 `contents: read`，checkout 不留凭据；release 的 secrets 不放顶层 env，只注入用它的那一步。

发版门禁（`release.yml` 第一步）：版本号与标签一致且不带 `-dev`；标签指向的提交必须在 `main` 上（`git merge-base --is-ancestor`）；产品数据下载后按 `data` Release 的 `SHA256SUMS` 校验，摘要写进 `build-info.json` 的 `data_sha256`。
**正式版前还欠**：产品数据改成不可变 tag 并在仓库里锁定版本（现在滚动覆盖，同一源码 tag 重跑可能拿到不同数据）、安装包内容验证（词库 / 模型 / 许可齐不齐、签名校验）。

## 两个 workflow

| 文件 | 触发 | 做什么 |
|---|---|---|
| `.github/workflows/ci.yml` | push main、PR | Linux 上 `cargo fmt --check` / clippy / test，排除 `qingjian-macos`（IMK 外壳只能在 macOS 编译，macOS runner 计费是 Linux 的 10 倍） |
| `.github/workflows/release.yml` | 推 `macos-v*` / `windows-v*` 标签 | `macos` job（`macos-26`）：下载产品数据 → 可选签名公证 → `bundle.sh --pkg` 打 arm64 与交叉编译的 x86_64 → 建 Release；`windows` job（`windows-latest`）：下载产品数据 → `build.ps1` 打 Inno Setup 安装包 → 建 Release。两者最后都跑 `publish-releases-json.sh` |

## 产品数据从哪来

词库、语言模型、释义表（`data/generated/*.qj`、`dicts/*.qj`、英文词表）不在 git 里，体积约 85 MB 且由本机数据管道生成。
`tools/release/data-bundle.sh` 把它们打成 `qingjian-data.tar.gz`，连同 LLM 生成的续跑中间产物 `qingjian-llm-intermediates.tar.gz`
一起上传到仓库里一个名为 `data` 的**预发布** Release（预发布不会成为 GitHub 的 latest，官网取 latest 时不会拿到它）。
`release.yml` 用 `gh release download data` 取回解到 `data/generated/`，`bundle.sh` 见到 `dict.qj` 就按产品数据打包。

数据重生成之后（重跑 lexicon / bigram / gloss-gen export）要重跑一次 `data-bundle.sh`，否则 CI 打的包还是旧数据。

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
  "generated": "2026-09-07T12:00:00Z",
  "repository": "owner/qingjian",
  "latest": "0.1.0",
  "releases": [
    {
      "version": "0.1.0",
      "date": "2026-09-07",
      "channel": "beta",
      "notes": ["整句输入：……", "候选旁有词性和译词……"],
      "commit": "869ad00…（40 位）",
      "built_at": "2026-09-07T08:38:12Z",
      "toolchain": "rustc 1.96.0 (ac68faa20 2026-05-25)",
      "assets": [
        { "platform": "macos", "arch": "Apple Silicon", "file": "Qingjian-0.1.0-arm64.pkg",
          "url": "https://github.com/owner/qingjian/releases/download/v0.1.0/Qingjian-0.1.0-arm64.pkg",
          "size": 35989277, "sha256": "…" },
        { "platform": "macos", "arch": "Intel", "file": "Qingjian-0.1.0-x86_64.pkg", "url": "…", "size": 36172871, "sha256": "…" }
      ]
    }
  ]
}
```

- `releases` 从新到旧，`latest` 是第一条的版本号；官网「当前版本」取它，历史版本列表就是整个数组。
- `channel` 是 `alpha` / `beta` / `rc` / `stable`，显示成什么字由官网定；`commit` / `built_at` / `sha256` 给用户核对下载的包，下载页应显示 sha256 与提交短哈希。
- 平台与架构由文件名判定（`-arm64.pkg` → Apple Silicon，`-x86_64.pkg` → Intel，`-Setup.exe` → Windows x64），以后 Linux 的包在脚本的 `ASSET_KINDS` 里加一行。
- `SHA256SUMS` 与 `releases.json` 自己不列进 `assets`。
- 官网侧要做的：构建时下载这个文件替代手写的 `releases` 数组（与拉 `docs/user` 的 `sync-docs.mjs` 同一处、同一个令牌），
  `downloadsOpen` 开关仍由官网自己控制。

## 本机打包

`apps/macos/scripts/bundle.sh --pkg` 打本机架构；`QINGJIAN_TARGET=x86_64-apple-darwin` 交叉编译 Intel 包（要先 `rustup target add`，
本机不需要时不必装，CI 上两个都打）。成品在 `target/pkg/Qingjian-<版本>-<arch>.pkg`，每个架构一个工作目录，连着打互不覆盖。
