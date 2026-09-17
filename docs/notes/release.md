| `.github/workflows/ci.yml` | push main、PR | `core`（Linux）fmt / clippy / 全 workspace 测试（排除 IMK 壳）；`macos` 编 IMK 壳并跑它的测试；`windows` 编 Server / TSF DLL / Settings 并跑测试。仓库公开，Actions 不计费 |
| `.github/workflows/audit.yml` | 每周一、Cargo.lock 变动 | `cargo audit`（RustSec 已知漏洞） |
| `.github/dependabot.yml` | 每周一 | Cargo 依赖与钉 commit 的 actions 的更新 PR |# 发版流程

2026-09-07 搭起来的：GitHub Actions 按标签打包、建 Release、生成官网下载页用的 `releases.json`。
这里记怎么发一版、各环节的依赖，以及官网怎么消费产物。

## 一次发版做什么

（下面以 macOS 为例；Windows 见「Windows 发版」一节，步骤同构。）

1. 改 `apps/macos/Cargo.toml` 的 `version`（`apps/macos` 的 Info.plist 版本号从这里取，pkg 文件名也是）：把 `0.1.2-dev` 改成 `0.1.2`。
   **发版之间版本号一直带 `-dev`**（Rust nightly / Firefox Nightly 那套）：Cargo.toml 写 `0.1.2-dev`，`bundle.sh` 打包时再接上 git 短哈希，
   本地装的、CI 中间构建的都显示 `0.1.2-dev-1a2b3c4`（工作区有改动加 `+`），测试时一眼知道装的是哪个提交；版本号干净的一定是线上包；
   带 `-dev` 的标签 CI 直接拒绝。pkg 的 `--version` 与 `distribution.xml` 只认数字点号，`bundle.sh` 去掉后缀再传，Info.plist 与 pkg 文件名保留完整版本。
   **各平台壳版本号独立**：macOS 的版本只在 `apps/macos/Cargo.toml`，跟 workspace 与其他壳无关（例：mac 到 `0.1.1`、win 还在 `0.1.0`）。
2. `CHANGELOG.md` 顶上加一节 `## <版本> · <日期> · <渠道>`，一行一条、面向用户的措辞。
   渠道是**更新渠道**（2026-09-17 定）：定期发的版本标 `stable`、版本号干净（`0.1.3`）；中间放给测试者的版本标 `alpha` / `beta` / `rc`，
   版本号带同名预发布后缀（`0.1.4-beta.1`，标签 `macos-v0.1.4-beta.1`），`release.yml` 见到后缀就把 GitHub Release 标成预发布、不抢 latest。
   「产品还在测试期」由 0.x 的版本号表达，不占渠道字段；0.1.2 及更早的条目标的 `beta` 是旧含义，不回改。
   **更新日志手写，不由提交自动生成**：提交信息里有大量内部改动（拆模块、修 RefCell 重入），用户看不懂也不关心；
   做法是发版前按上个标签以来的 `git log` 起草几条，人审一遍再定稿。
3. 提交，打**带平台前缀**的注释标签并推：`git tag -a macos-v0.1.1 -m "青简 macOS 0.1.1" && git push origin main macos-v0.1.1`
   （标签按平台加前缀 `macos-v*` / 将来 `windows-v*`，因为各平台版本号独立、光靠 `v<版本>` 会撞车；旧的 `v*` 标签仍能被官网识别，向后兼容）。
3b. 标签推出去之后紧接一个普通提交把版本号改成下一个开发版（只是改 Cargo.toml，不打标签、不建 Release；-dev 版本永远没有标签与 Release）：`apps/macos/Cargo.toml` 改成 `0.1.3-dev`（Windows 同理 `0.1.3-dev`），本地从此打的包都带 `-dev`。
4. `release.yml` 跑完后 GitHub Release 上有 `qingjian-<版本>-macos-arm64.pkg`、`qingjian-<版本>-macos-x86_64.pkg`、`SHA256SUMS`、`build-info.json`（提交、构建时间、工具链）、`releases.json`。
5. 官网由 Cloudflare Workers Builds 按官网仓库的提交自动构建，没有可调用的构建钩子，所以主仓库靠**往官网仓库推一个小提交**来触发：
   `tools/release/bump-website.sh` 把版本标签与文档提交号写进官网的 `src/content/upstream.json` 并提交推送（提交者 qingjian-ci）。
   配了 `QINGJIAN_WEB_TOKEN`（对 qingjian-web 有 Contents: read and write 的 fine-grained PAT）release.yml 末尾自动做；
   官网文档只随发版更新（`docs/user/` 平时改动不推官网，免得文档领先于用户装到的版本）。没配就在官网仓库随便提交一次（或本地跑这个脚本）。
   官网构建时才拉最新 Release 的 `releases.json` 与主仓库 `docs/user`，所以提交内容本身不重要，`upstream.json` 只是留个记录、
   顺便让文档按记下的提交号拉（版本对得上）。

workflow 会核对 `apps/macos/Cargo.toml` 版本号与标签（去掉 `macos-v` 前缀后）一致，不一致直接失败，避免打出版本号错的包。

Rust 工具链由 `rust-toolchain.toml` 钉版本（现在 1.96.0），两个 workflow 里 `dtolnay/rust-toolchain@master` 的 `toolchain:` 输入写同一个号；升级 Rust 时三处一起改。

## Windows 发版

1. 改 `apps/windows/{server,tsf,settings}/Cargo.toml` 的 `version`（三个一起改；打包脚本与 workflow 读 `server` 那份）。
   同样带 `-dev`：发版之间是 `0.1.0-alpha.2-dev`，发版提交改成 `0.1.0-alpha.2`；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 把整个预发布后缀去掉再传，安装包与 DLL 文件名保留完整版本。
   内测版用 semver 预发布号 `0.1.0-alpha.1`、`0.1.0-alpha.2`（0.1.3 起与 macOS 共用版本号、不再走预发布：一个版本号一节 CHANGELOG，哪个平台发了就打哪个平台的标签，只发一个平台时那一节全是该平台的条目）：CHANGELOG 按版本号索引、官网按版本号列条目，
   与 macOS 的 `0.1.0` / `0.1.1` 不能同号；Inno 的 `VersionInfoVersion` 只认数字，`build.ps1` 会把后缀去掉再传。
2. `CHANGELOG.md` 加一节 `## 0.1.0-alpha.1 · 日期 · alpha`。
3. 打标签 `windows-v0.1.0-alpha.1` 推送。`release.yml` 的 `windows` job 在 `windows-latest` 上：核对版本 → 下载 `data` Release
   → 装 Inno Setup 7.1.0（与开发机同版本，钉死 GitHub Release 的安装程序）→ `build.ps1`→ 建 Release（`qingjian-<版本>-windows-x86_64-setup.exe` + `SHA256SUMS` + `build-info.json`）
   → `publish-releases-json.sh` 生成 `releases.json`，挂到本次发布并覆盖到 GitHub latest 那版上（官网只读 latest 的）。
4. **没有代码签名证书时** workflow 设 `QINGJIAN_UIACCESS=0`：没签名的 exe 带 uiAccess=true 起不来。
   代价是候选窗在任务栏搜索 / 设置这类 UWP 宿主里可能被盖住，用户文档与 CHANGELOG 已列为已知问题。
   Certum 开源证书办下来后：在 `build.ps1` 加 signtool 一步（`sign-local.ps1` 是本机自签的参考），workflow 去掉那个环境变量。
   SmartScreen 对无签名安装包的拦截也一并消失。
5. 官网：`releases.json` 里 Windows 包由文件名 `-windows-x86_64-setup.exe` 识别（`ASSET_KINDS`），下载页按访问者平台取「有该平台安装包的最新版本」
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

词库、语言模型、释义表（`data/generated/*.qj`、`dicts/*.qj`、英文词表）不在 git 里，体积约 90 MB 且由本机数据管道生成。
它们发在仓库里一个个**不可变**的预发布 Release 上：`data-v1`、`data-v2`……每次数据重生成发一个新号、从不覆盖
（预发布不会成为 GitHub 的 latest，官网取 latest 时不会拿到它）。仓库里 `tools/release/data.lock` 钉住当前要用的标签与两个资产的 SHA-256，
跟用到新数据的代码同一个提交进去：checkout 哪个提交就拿到它对应的那版数据，离线自编译的人不会因为我们改了数据而编出坏包。

- `tools/release/data-bundle.sh`：把 `data/generated/` 打成 `qingjian-data.tar.gz`，本地整句模型单文件 `data/model/model.qjm`
  （训练仓库导出三件套到 `data/model/`，`tools/release/pack-model.sh` 打成一个 `.qj` 容器，fp16 约 56 MB，元数据也写在那个脚本里）原样上传，
  连同 LLM 续跑中间产物 `qingjian-llm-intermediates.tar.gz` 发到下一个 `data-vN`（`--tag` 可指定，已存在就拒绝），然后改写 `data.lock`。
- `tools/release/data-fetch.sh`：按 `data.lock` 下载（有 gh 用 gh，没有就 curl 直连）、按锁文件里的哈希校验（不信 Release 自己那份 `SHA256SUMS`），
  数据包解到 `data/generated/`、`model.qjm` 放到 `data/model/`。`release.yml` 两个 job 和离线自编译走同一个脚本；
  `bundle.sh` 见到 `dict.qj` 就按产品数据打包、见到 `model.qjm` 就放进 `Resources/model/`，`qingjian.iss` 同理装进 `{app}\data\model`。
  标签与两个哈希记进 `build-info.json`（`data_tag` / `data_sha256` / `model_sha256`）。

数据重生成之后（重跑 lexicon / bigram / gloss-gen export）或模型重训之后跑一次 `data-bundle.sh`（三件套比 `.qjm` 新会自动重打），
把锁文件的改动提交（`chore(data): 数据 data-vN`），否则 CI 打的包还是锁文件指的旧数据。模型文件缺失或哈希不符时 CI 会失败，不会静默地发出错数据的包。
2026-09-16 之前用的是滚动覆盖的 `data` Release，已冻结不再更新。

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
  `arch` 给人看（Apple Silicon / Intel / x64），`cpu` 给程序比对；以后 Linux 的包在 `ASSET_KINDS` 里加一行。
- `schema_version` 现在是 1：只加字段不用动，改了已有字段的含义或结构才加一。
- `SHA256SUMS` 与 `releases.json` 自己不列进 `assets`。
- 官网侧要做的：构建时下载这个文件替代手写的 `releases` 数组（与拉 `docs/user` 的 `sync-docs.mjs` 同一处、同一个令牌），
  `downloadsOpen` 开关仍由官网自己控制。

## 本机打包

`apps/macos/scripts/bundle.sh --pkg` 打本机架构；`QINGJIAN_TARGET=x86_64-apple-darwin` 交叉编译 Intel 包（要先 `rustup target add`，
本机不需要时不必装，CI 上两个都打）。成品在 `target/pkg/qingjian-<版本>-macos-<arch>.pkg`，每个架构一个工作目录，连着打互不覆盖。
