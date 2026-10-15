# Issue 维护标签指南

新建 Issue 模板只自动添加基础类型标签（如 `bug`、`feature`）；平台、模块、优先级、贡献难度及流程标签由维护者根据证据补充。此文档是标签命名与分配的约定，不代表标签已经在 GitHub 仓库中创建。

## 优先级（最多一个）

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

## 排查、流转与发布

| 标签 | 含义 |
| --- | --- |
| `needs triage` | 尚未完成分类和初步判断 |
| `needs reproduction` | 缺少可靠复现或验证条件 |
| `needs info` | 缺少关键环境、日志或步骤，已明确向报告者提出问题 |
| `needs investigation` | 现象基本明确，但技术根因未定位 |
| `blocked` | 存在具体依赖；评论里写明阻塞内容 |
| `status: ready` | 需求/解决方向和验收标准清楚，可开始实施 |
| `status: in progress` | 已有人实际着手开发（不是仅表示计划做） |
| `regression` | 有证据显示此前版本正常而新版退化 |
| `release blocker` | 明确阻断某次具体发布，评论注明目标版本和原因 |
| `upstream` | 与系统框架或上游库的问题/修复相关 |

`needs info`、`needs reproduction`、`needs investigation` 表达不同的阻碍。可按实际情况组合，但需要定期清理过期状态。不要因为 Issue 久未更新就自动贴 `blocked` 或 `release blocker`。

## 与既有标签的关系

- 类型标签沿用 `bug`、`feature`、`enhancement`、`question`，每个 Issue 至多一个。
- 平台与模块沿用现有的 `os: *` 和 `area: *`。
- 现有 `status: *` 如有同义标签，应优先复用；避免 `status: ready` / `status: in progress` 与原状态体系冲突。
- `duplicate`、`invalid`、`wontfix` 属于关闭理由；能使用 GitHub 原生关闭原因（尤其 Duplicate）时，优先用原生功能，不为标签而重复标记。
- 所有维护标签都不由报告人自行选择，且不要在 Issue 模板中自动附加。
- 创建/重命名标签前须核对仓库实时已有标签，名称和描述一致时直接复用。

## 参考

- GitHub: https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/encouraging-helpful-contributions-to-your-project-with-labels
- VS Code: https://github.com/microsoft/vscode/wiki/Issues-Triaging
- Kubernetes: https://github.com/kubernetes/kubernetes/labels
- Homebrew: https://github.com/Homebrew/brew/labels
