# docs

README.md 只介绍项目（各语言译本在 [i18n/](i18n/)），所有技术内容放在这里，分四类：前三类是给开发者看的内部文档，`user/` 是给用户看的、官网渲染的文档。

| 目录 | 放什么 |
|---|---|
| [design/](design/) | 设计：现在是什么样、为什么这么定 |
| [plan/](plan/) | 计划：接下来做什么、按什么顺序 |
| [notes/](notes/) | 工程记录：性能优化史、事故复盘、踩坑，按时间顺序写，以后复习与面试用 |
| [user/](user/) | 用户文档：官网「文档」页的内容源，按目录结构渲染；中文原文在这里，各语言译本在 [user/i18n/](user/i18n/)；维护约定见 [user/README.md](user/README.md) |
| [i18n/](i18n/) | 根目录 README.md 的各语言译本（繁體中文、English、日本語、Español、Tiếng Việt、Deutsch）；改 README 时一起改 |

| 文件 | 内容 |
|---|---|
| [contributing.md](contributing.md) | 开发约定：架构约束短版、代码组织、版本号、提交信息、文档同步、提交前检查、发版与外部 PR 流程（CLAUDE.md 直接载入它） |
| [design/architecture.md](design/architecture.md) | Core 与平台层的划分、crate 结构、必须遵守的架构约束、`.qj` 数据容器 |
| [design/candidate-ui.md](design/candidate-ui.md) | 候选窗口、按键约定与翻译 annotation 的设计 |
| [design/landscape.md](design/landscape.md) | 同类项目（水杉、Rime）、可用数据源及其许可 |
| [design/rendering.md](design/rendering.md) | 跨平台 UI 与自绘渲染器（2026-09-13）：显示面与控件面的区分、Flutter / Avalonia / WebView 的调研结论、主题的两条路、tiny-skia + cosmic-text 的 spike 结果与两平台接入 |
| [design/aux-code.md](design/aux-code.md) | 辅码（2026-09-14）：触发键与过滤语义、Rime 码表导入、笔画原生表与大陆序覆盖、候选注记与设置界面 |
| [design/core-internals.md](design/core-internals.md) | Core 与显示层拆解（2026-09-23）：`Engine` 的字段分组与线程归属、一次按键的数据流、三条异步旁路、上屏写回、缓存，三个平台的显示层与还各写一份的地方 |
| [design/voice-and-swipe.md](design/voice-and-swipe.md) | 语音与滑行输入（2026-10-20）：Rust 与平台壳的分工、语音结果怎么经 Core 上屏、识别方式接口 `Transcriber`、火山引擎豆包流式识别、入口与面板交互；滑行的范围、解码接口 `SwipeDecoder`、按词表打分的 `ShapeDecoder` 与判定 |
| [plan/roadmap.md](plan/roadmap.md) | 分阶段路线图、各阶段的依赖关系与已完成项 |
| [plan/zh_tw_support_plan.md](plan/zh_tw_support_plan.md) | 繁体输出与台湾注音支持的分析与方案（贡献者 pinchiu，#22）：读音标准差异、台湾用语、简转繁一对多；两条路线 |
| [plan/todo.md](plan/todo.md) | 待办清单，按「从自用到能给别人用」排 |
| [plan/wubi.md](plan/wubi.md) | 五笔（形码）支持方案（2026-09-15）：为什么不能按双拼的方式接、两条平行管线、码表与词频、`[general] scheme` 收敛、分期 |
| [notes/crate-notes.md](notes/crate-notes.md) | 各 crate / app / tool 的实现要点：入口类型、数据文件、常数、生成命令 |
| [notes/linux-fcitx5.md](notes/linux-fcitx5.md) | Linux 默认面板构建、协议、安装与排错 |
| [notes/linux-packaging.md](notes/linux-packaging.md) | Linux 发行包（2026-09-23）：`build.sh` 在容器里打 deb / rpm，各包装了什么、能装哪些发行版、装法与限制，`smoke/run.sh` 在五个发行版的干净容器里装包自检 |
| [notes/linux-dual-frontend.md](notes/linux-dual-frontend.md) | Linux 双前端（Fcitx5 / IBus）怎么验（2026-09-21）：能自动跑的三套、只能真会话手测的六条及其判据、现在验不了的两件事、踩过的四个坑 |
| [notes/linux-bringup.md](notes/linux-bringup.md) | Linux 走 IBus 那次的工程记录（2026-09-14，路线已改走 Fcitx5，留作双框架支持的参考）：分层怎么切、五个只有真机才暴露的坑（总线地址 / Shift+数字 / GVariant 嵌套 / 把 `/` 当仓库根 / 锁毒化）、合成客户端验不了什么、装机验收数字 |
| [notes/linux-fcitx4.md](notes/linux-fcitx4.md) | Ubuntu 22.04 的 Fcitx4 源码适配、安装与验证 |
| [notes/android.md](notes/android.md) | Android 输入法：键盘、候选条、面板、剪贴板、快捷翻译怎么接到 Core，取舍与踩坑 |
| [notes/performance.md](notes/performance.md) | 历次性能优化：起因、定位方法、改法、数字前后对比与经验 |
| [notes/release.md](notes/release.md) | 发版流程：CHANGELOG、标签触发的 CI 打包、产品数据包、签名公证的 Secrets、官网用的 `releases.json` |
| [notes/model-identities.md](notes/model-identities.md) | 含章·通变与含章·知微的正式名称和检查点编号 |
| [notes/phrase-layer.md](notes/phrase-layer.md) | 短语层（2026-09-12）：常用词表收不到的 我的 / 不知道 怎么从语料挖、怎么进语言模型而不伤整句、两把尺子的前后数字 |
| [notes/domain-words.md](notes/domain-words.md) | 领域词（2026-09-12）：从输入日志人工挑 48 条进基础词库的挑法、低频词当 token 统计为什么伤整句、两把尺子的前后数字 |
| [notes/constant-sweep.md](notes/constant-sweep.md) | 排序常数扫描（2026-09-12）：插值与敲错代价在冻结日志上扫网格，全在平台区不改；没命中的构成与复现步骤 |
| [notes/windows-win10.md](notes/windows-win10.md) | Windows 10 与设置程序（2026-09-13）：Reactor 早期绑定 Windows 11 才有的 AppModel API 导致加载期失败，改自包含部署 + 延迟加载 |

约定：文档写中文，代码标识符一律英文。实现与文档产生分歧时以代码为准，并同步更新文档。
