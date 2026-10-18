# Issue 维护标签指南

新建 Issue 模板只自动添加基础类型标签（如 `bug`、`feature`）；平台、模块、优先级、贡献难度及流程标签由维护者根据证据补充。下面列的标签都已在仓库里建好，新增标签先改这一页再建。

## 类型（至多一个）

| 标签 | 含义 |
| --- | --- |
| `bug` | 功能不正常、崩溃、行为与文档不符 |
| `feature` | 新功能请求：目前没有的能力 |
| `enhancement` | 改进：现有功能的体验优化 |
| `performance` | 性能：延迟、内存、启动速度 |
| `question` | 使用疑问，不一定是缺陷 |
| `documentation` | 文档缺失、有误或需要补充 |

平台用 `os: *`，模块用 `area: *`（候选窗口、引擎、词库、输入方案、设置、安装、应用兼容、云服务、本地模型、翻译、学习、隐私），另有 `accessibility` 无障碍。没打 `area: *` 的就是还没分类。

## 优先级（至多一个）

| 标签 | 含义 |
| --- | --- |
| `priority: critical` | 大范围无法输入、数据丢失、严重安全/隐私问题或发布阻断，需要立即关注 |
| `priority: high` | 核心操作明显受损、影响较广、缺乏合理绕过方式 |
| `priority: medium` | 有明确价值但不紧急，可正常排期 |
| `priority: low` | 影响有限的细节优化或低紧迫需求 |

优先级衡量用户影响和紧急程度，不代表开发难度，也不构成版本承诺。信息不充分时不强行分级。

## 社区参与与工作量

| 标签 | 含义 |
| --- | --- |
| `good first issue` | 适合第一次贡献：边界清楚、修改集中、验收标准明确、可测试 |
| `help wanted` | 欢迎外部贡献，维护者接受围绕此 Issue 的 PR |
| `effort: small` | 初步评估工作量小；仍需独立判断技术风险 |
| `effort: large` | 跨模块或工程量较大，建议先讨论方案 |

`good first issue` 可以与 `help wanted` 同时使用。小改动不一定适合新人，涉及平台协议、并发或上屏逻辑时尤其谨慎。

## 状态

| 标签 | 含义 |
| --- | --- |
| `status: needs-info` | 缺少关键环境、日志或步骤，已明确向报告者提出问题 |
| `status: needs-repro` | 信息够了，但维护者还没能复现，缺可靠的复现或验证条件 |
| `status: confirmed` | 已复现或已确认原因 |
| `status: planned` | 已接受并排期，方向和验收标准清楚，可以开始做 |
| `status: in-progress` | 已有人实际着手开发（不是仅表示计划做） |
| `status: blocked` | 受外部条件阻塞（证书、上游等）；评论里写明阻塞内容 |
| `status: deferred` | 有效但暂缓处理，受条件所限 |
| `status: fixed` | 已修复，代码已改并验证，等发版后关闭 |

没打 `status: confirmed` 的就是根因还没定位。`status: needs-info` 与 `status: needs-repro` 表达不同的阻碍，需要定期清理过期状态。不要因为 Issue 久未更新就自动贴 `status: blocked` 或 `release blocker`。

## 其他

| 标签 | 含义 |
| --- | --- |
| `regression` | 有证据显示此前版本正常而新版退化 |
| `release blocker` | 明确阻断某次具体发布，评论注明目标版本和原因 |
| `upstream` | 与系统框架或上游库的问题/修复相关 |

## 约定

- `duplicate`、`invalid`、`wontfix` 属于关闭理由；能使用 GitHub 原生关闭原因（尤其 Duplicate）时，优先用原生功能，不为标签而重复标记。
- `dependencies`、`github_actions`、`rust` 由 Dependabot 打，不手动用。
- 所有维护标签都不由报告人自行选择，且不要在 Issue 模板中自动附加。

## 参考

- GitHub: https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/encouraging-helpful-contributions-to-your-project-with-labels
- VS Code: https://github.com/microsoft/vscode/wiki/Issues-Triaging
- Kubernetes: https://github.com/kubernetes/kubernetes/labels
- Homebrew: https://github.com/Homebrew/brew/labels
