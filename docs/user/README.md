# 用户文档（官网「文档」页的内容源）

这个目录里的 Markdown 是给**用户**看的，官网（`qingjian-web`）构建时拉取整个目录，按目录结构渲染成文档页。
这里是中文（简体）原文，其他语言的译本在 [i18n/](i18n/)；某页没有译本时官网显示替补版本，见 [i18n/README.md](i18n/README.md)。
`docs/` 下其他目录是给开发者看的工程文档，不渲染。

这个 README 是维护约定，不渲染。

## 结构

```text
docs/user/
├── index.md              # 文档首页：是什么、从哪开始、各平台现状
├── getting-started/      # 开始使用：安装（macOS、Windows）、第一次输入、按键速查（只放表）、设置窗口总览
├── input/                # 打字：整句与纠错、中英文、标点符号表情、日期算式、拼音方案与模糊音、五笔、辅码、繁体
├── learning/             # 学语言：候选旁的译词、生词本、统计
├── personalize/          # 个性化：外观与主题、改按键、自定义短语、词库、输入习惯
├── cloud/                # AI 服务（可选）：概述与发送内容、云联想、问字与翻译、AI 补译
├── platforms/            # 各平台：平台差异表、Linux、Android（三页）
├── help/                 # 隐私与帮助：隐私、常见问题、反馈、卸载
├── advanced/             # 进阶：配置文件、数据文件与日志、自制主题、自制方案与码表
└── i18n/                 # 各语言译本，目录结构与原文一一对应，约定见 i18n/README.md
```

- **按用户要做的事分组，不按设置程序的页分组。** 一个功能只在一页讲全；别处提到时一句话加链接，不重复写。
- **技术细节只放进阶组。** 配置键名、文件路径、文件格式只写在 `advanced/`；其他页只说「设置 → 某页 → 某项」。
  例外：Linux 安装要在终端里执行的命令写在 `platforms/linux.md`。
- **平台差异写在功能页里**（「macOS 与 Windows 支持」「Linux 暂不支持」），并同步 `platforms/differences.md` 的表。

- **一个文件夹是一个分组**，文件夹里的 `index.md` 给分组标题与顺序；文件夹里的其他 `.md` 是分组下的页面。
- **一个 `.md` 是一页**。顶部 frontmatter：

  ```yaml
  ---
  title: 安装            # 页面标题，也是侧栏里的名字
  order: 1               # 同级排序，小的在前
  description: 一句话     # 搜索引擎摘要与分享卡片用，40 字左右
  ---
  ```

- **文件名与文件夹名用 ASCII**（小写、连字符），中文只写在 `title` 里；URL 就是路径：`getting-started/install.md` → `/docs/getting-started/install`。
- **图片放在同一文件夹**，相对路径引用（`![候选窗口](candidate-window.png)`），构建时随文档一起拷走。截图用浅色外观、系统缺省字号、2x。
- 正文第一行不再写 `# 标题`，标题来自 frontmatter；正文从 `##` 开始。
- 跨页链接用相对路径指向 `.md` 文件（`[安装](../getting-started/install.md)`），渲染时改写成站内链接。

## 写法

- 说明书体：陈述句，不用「一下 / 多半 / 照常 / 免得」这类口语助词与副词，不拟人、不调侃；条件用「若…则」，可行用「可」，必要用「需」；术语固定（候选窗口、拼音行、上屏、组合键）。
- 面向普通用户，不出现实现词（`config.toml`、marked text、Viterbi、panic、600 权限这类）。要指路时说「设置 → 高级」，不说文件名；
  数据文件位置只在 [advanced/data-files.md](advanced/data-files.md) 一处列出。
- 按键按键帽写：`Space`、`Enter`、`Backspace`、`Delete`、`Esc`、`Tab`、`←` `→` `↑` `↓`、`Home` / `End`、`PageUp` / `PageDown`；
  macOS 修饰键用符号（`⌥ + 数字`、`⌃⌥T`、`⇧ + Tab`），Windows 用单词（`Ctrl + 数字`、`Ctrl + Alt + T`）；菜单与页名用「」：「系统设置 → 隐私与安全性」。
- 一个功能改了行为，同一个提交里改这里对应的页；README 的安装 / 基本用法 / 隐私一节与这里重复的内容以这里为准，README 只留概要。
- 事实来源是代码与 `docs/design/candidate-ui.md`（按键约定）、`docs/design/architecture.md`；写之前对一遍，不凭印象。
