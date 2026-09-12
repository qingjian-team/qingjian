| [notes/domain-words.md](notes/domain-words.md) | 领域词（2026-09-12）：从输入日志人工挑 48 条进基础词库的挑法、低频词当 token 统计为什么伤整句、两把尺子的前后数字 |
# docs

README.md 只介绍项目，所有技术内容放在这里，分四类：前三类是给开发者看的内部文档，`user/` 是给用户看的、官网渲染的文档。

| 目录 | 放什么 |
|---|---|
| [design/](design/) | 设计：现在是什么样、为什么这么定 |
| [plan/](plan/) | 计划：接下来做什么、按什么顺序 |
| [notes/](notes/) | 工程记录：性能优化史、事故复盘、踩坑，按时间顺序写，以后复习与面试用 |
| [user/](user/) | 用户文档：官网「文档」页的内容源，按目录结构渲染；维护约定见 [user/README.md](user/README.md) |

| 文件 | 内容 |
|---|---|
| [design/architecture.md](design/architecture.md) | Core 与平台层的划分、crate 结构、必须遵守的架构约束、`.qj` 数据容器 |
| [design/candidate-ui.md](design/candidate-ui.md) | 候选窗口、按键约定与翻译 annotation 的设计 |
| [design/landscape.md](design/landscape.md) | 同类项目（水杉、Rime）、可用数据源及其许可 |
| [plan/roadmap.md](plan/roadmap.md) | 分阶段路线图、各阶段的依赖关系与已完成项 |
| [plan/todo.md](plan/todo.md) | 待办清单，按「从自用到能给别人用」排 |
| [notes/performance.md](notes/performance.md) | 历次性能优化：起因、定位方法、改法、数字前后对比与经验 |
| [notes/release.md](notes/release.md) | 发版流程：CHANGELOG、标签触发的 CI 打包、产品数据包、签名公证的 Secrets、官网用的 `releases.json` |
| [notes/phrase-layer.md](notes/phrase-layer.md) | 短语层（2026-09-12）：常用词表收不到的 我的 / 不知道 怎么从语料挖、怎么进语言模型而不伤整句、两把尺子的前后数字 |
| [notes/constant-sweep.md](notes/constant-sweep.md) | 排序常数扫描（2026-09-12）：插值与敲错代价在冻结日志上扫网格，全在平台区不改；没命中的构成与复现步骤 |

约定：文档写中文，代码标识符一律英文。实现与文档产生分歧时以代码为准，并同步更新文档。
