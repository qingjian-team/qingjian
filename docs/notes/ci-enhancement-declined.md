# 2026-10-10：外部提出的 CI/CD 增强被关闭

一次来自外部贡献者（fork `bfxh/qingjian`）的 CI/CD 增强尝试。PR [#522](https://github.com/qingjian-team/qingjian/pull/522) 提交后 CI 三平台全绿、可合并状态 `CLEAN`，但维护者主动关闭：

> 这个 PR 没有对应的需求，里面的 CODEOWNERS 分工和发版流程的改动也需要我们自己来定，先关掉了。

记在这里是为了后人（包括未来的自己）不再重复同一个动作：**仓库当前对 CI/CD 基础设施没有增量需求，不要再主动提同类 PR。**

## 为什么会被关

贡献内容分四块，其中两块属于「组织决策」而非「技术改进」，外部贡献者越过了边界：

| 改动 | 性质 | 结果 |
|---|---|---|
| `supply-chain.yml`（cargo-deny + SBOM + attestation） | 技术改进，无争议 | 技术可行，但未获采纳 |
| `ci.yml` 加超时、拆步骤、失败上传诊断构件 | 技术改进，无争议 | 技术可行，但未获采纳 |
| `.github/CODEOWNERS` | **组织决策**：给谁分哪块代码的 review 责任 | 触发关闭的主因之一 |
| `release.yml` 加 `workflow_dispatch` + `environment: release` | **组织决策**：要不要加人工审批环节、要不要开手动发版入口 | 触发关闭的主因之一 |

维护者关得对：后两项得他们自己拍板，一个外部贡献者按自己的判断写进代码库，等于替他们做了决定。

技术质量本身不是问题——CI 三 job 全 pass，YAML 解析、`needs.prepare.outputs.*` 引用核对、`bash -n` 都验过。真正的问题在**范围**。

## `version_override` 是一个危险设计，别再照抄

那版 PR 给 `release.yml` 的 `workflow_dispatch` 加了一个 `version_override` 输入，用来「在误打标签时手动纠正版本号」。

它绕过的是 `docs/notes/release.md` 里那条核心门禁：

> 发版门禁（`release.yml` 第一步）：版本号与标签一致且不带 `-dev`

发版门禁存在的意义，是保证「Release 上那个版本号 == git 标签 == 各平台 `Cargo.toml` 里的 `version`」三者强制对齐。任何能从外部输入的、能改写版本号的入口，都是把这道门禁变成可选项——`dry_run` 同理，如果它没真正停住 `gh release create`，就只是「看起来检查了」。

写的时候已经发现一处 bug 并修掉：`version_override` 的处理逻辑原本写在 `case "$GITHUB_REF_NAME"` **之前**，那时 `$VERSION` 还没从标签里解析出来，`[[ "$MANUAL_VERSION" != "$VERSION" ]]` 永远拿不到正确的旧值，覆盖逻辑失效。修法是把它移到 `case` 之后。这说明这个改动本身的正确性得靠事后核对才兜得住，本就不该进主干。

**结论：发版门禁不要加任何能绕过的输入。** 真要重发，走「改版本号 → 重新打标签」的既有路径，别造一个 workflow 层的逃生口。

## 当时 CI 上真实存在的问题

core job 第一轮确实挂了，原因值得记一笔——是我自己引入的，不是仓库原有的问题。

为了抓 ctest 和安装测试的日志，给两处管道加了 `set -o pipefail`：

```yaml
set -o pipefail
env -u DBUS_SESSION_BUS_ADDRESS ctest --test-dir ... --output-on-failure | tee target/ci-logs/fcitx5-ctest.log
```

`pipefail` 会让管道中**任一环节**非零退出就判整步失败，包括 `tee` 自己。而原命令本来没设 `pipefail`——这一改凭空多出一类原本不存在的失败模式，也偏离了仓库既有行为。去掉 `set -o pipefail` 后 core job 3m5s 通过。

教训：**改 CI 时不要顺手引入原来没有的失败判据。** 想留日志，`tee` 裸接就行，别捎带改变退出码语义。

## 后人该怎么做

想给这个仓库做 CI/CD 增强，先确认维护者**明确提过**这个需求（issue 或讨论）。没有的话：

- 供应链类（cargo-deny、SBOM）属于行业基线，可以先在 PR 里**只问一句**「要不要加」，别直接把文件写全
- 涉及人员分工、发版流程、审批环节的，一律留给维护者自己定
- 想验证自己的改动能不能跑，用 fork 跑通即可，不必开上游 PR
