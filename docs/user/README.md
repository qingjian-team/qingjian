# 用户文档（官网「文档」页的内容源）

这个目录里的 Markdown 是给**用户**看的，官网（`qingjian-web`）构建时拉取整个目录，按目录结构渲染成文档页。
`docs/` 下其他目录是给开发者看的工程文档，不渲染。

这个 README 是维护约定，不渲染。

## 结构

```text
docs/user/
├── index.md              # 文档首页
├── getting-started/      # 开始使用：安装、第一次输入
├── input/                # 输入：英文模式、拼写纠错、模糊音与双拼、快捷输入
├── learning/             # 学习：译词与生词、统计
├── cloud/                # 云联想（可选）
├── settings/             # 设置：偏好设置、词库
└── help/                 # 帮助：数据与日志、卸载、反馈
```

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

- 面向普通用户，不出现实现词（`config.toml`、marked text、Viterbi、panic、600 权限这类）。要指路时说「偏好设置 → 高级」，不说文件名；
  数据文件位置只在 [help/data-and-logs.md](help/data-and-logs.md) 一处列出。
- 按键写成 `空格`、`Tab`、`⌥ + 数字`、`⌃⌥T`；菜单与页名用「」：「系统设置 → 隐私与安全性」。
- 一个功能改了行为，同一个提交里改这里对应的页；README 的安装 / 基本用法 / 隐私一节与这里重复的内容以这里为准，README 只留概要。
- 事实来源是代码与 `docs/design/candidate-ui.md`（按键约定）、`docs/design/architecture.md`；写之前对一遍，不凭印象。
