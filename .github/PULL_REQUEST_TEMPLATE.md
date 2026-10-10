<!-- 谢谢投稿。下面的清单是合并前维护者会逐条看的，提前对一遍能少一轮往返；不适用的项划掉即可。 -->

## 改了什么

<!-- 一两句：解决什么问题、怎么解的。关联的 issue 写「关闭 #编号」。 -->

## 平台

- [ ] macOS
- [ ] Windows
- [ ] Linux
- [ ] iOS
- [ ] Android
- [ ] 与平台无关（Core / 数据 / 文档）

## 合并前清单

- [ ] `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 本机全过
- [ ] **改到 `Cargo.lock`（加/升依赖）时，`supply-chain` 检查（cargo-deny + SBOM）已同步通过**
- [ ] **用户能感知的行为变了（按键、候选、菜单、设置项、配置文件、数据文件），`docs/user/` 对应页面已同步改**，按键改动同时更新 `docs/user/getting-started/keys.md`
- [ ] 一个功能只在一个平台实现时，已在文档里标明平台，并在 PR 里说明另一平台的差距
- [ ] 代码标识符英文、注释与文档中文；新文件有 `//!` 文件头；单文件不超过 800 行
- [ ] 提交信息用 Conventional Commits（`fix(core): ……` / `feat(windows): ……`，说明写中文）；其余约定见 [docs/contributing.md](../docs/contributing.md)
- [ ] 不改 `CHANGELOG.md`（发版时由维护者统一写）

## 外部贡献者注意

首次从 fork 提 PR 时，`ci` / `CodeQL` 可能会停在「等待批准」状态（GitHub 对 fork 的安全策略），
不是仓库坏了。维护者 approve 一次后，后续 PR 会自动跑。


## 怎么验证的

<!-- 修 bug、改按键 / 上屏 / 候选窗位置的 PR 必填：在自己机器上复现过、改完在同一个应用里确认修好，写明系统版本（Windows 注明 10 还是 11）、应用、操作步骤，可附截图或录屏。
     编译与 CI 通过不算验证；复现不了的请把分析写在 issue 里，或只提加日志的 PR。 -->
