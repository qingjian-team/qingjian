---
title: 直接编辑配置文件
order: 1
description: 青简的全部设置保存在一个文本配置文件里：位置与打开方式、生效方式、写错时的处理、配置项与设置界面的对照，以及 Linux 上常用的写法。
---

设置窗口里能改的，都不必动配置文件。本页写给想直接编辑配置文件，或要用设置窗口里没有的项的人。

## 位置与打开

全部设置保存在一个 TOML 格式的文本文件 `config.toml` 中，文件里每一项都附有说明：

- **macOS**：`~/Library/Application Support/Qingjian/config.toml`
- **Windows**：`%APPDATA%\Qingjian\config.toml`
- **Linux**：`~/.config/qingjian/config.toml`（设置了 `XDG_CONFIG_HOME` 时在它下面的 `qingjian/`）

「设置 → 高级 → 配置文件」的「在编辑器中打开」直接打开它，「打开数据目录」打开它所在的文件夹。

## 生效方式

- 保存后一秒内生效，不用重新切换输入法。Linux 上 `[general]` 的 `preedit` 与 `input_log` 例外，改完要重启青简。
- 写错时沿用上一份可用的设置，输入法不退出；macOS 的输入法菜单会多一条灰色的错误说明，按说明改回并保存即可。
- 某一条自定义短语写错（权重不在 1–999、内容不是文字、与前面的条目同码同内容等）只跳过那一条，其余照常生效，运行日志里记下是第几条、错在哪；在设置窗口里保存短语时，跳过的条目原样留在文件里。
- 一个分节（如 `[general]`）只写一次；要改的项写在已有的分节下面。

## 配置项与设置界面

| 配置项 | 设置界面 | 说明 |
|---|---|---|
| `[general] scheme` | 输入 → 拼音 | `pinyin`（全拼）、`xiaohe`、`ziranma`、`microsoft`、`sogou`、`abc`、`xiaolang`、`shoudao`、`jiajia`、`ziguang`、`zhuyin`，或自制方案的文件夹名 |
| `[general] hold_selections` | 输入 → 分次选词先不上屏 | `true` / `false` |
| `[aux_code] enabled`、`scheme` | 输入 → 辅码 | `scheme` 写码表的文件名或方案的文件夹名，随包笔画码是 `stroke` |
| `[general] aux_code_keep_empty` | 输入 → 码删空后留在辅码 | `false` 为删空即退出 |
| `[general] traditional_variant` | 候选 → 输出字形 | `off`、`taiwan`、`hongkong`、`standard`；0.2.1 及以前的 `traditional = true` 按 `taiwan` 读 |
| `[general] emoji`、`kaomoji`、`symbols` | 候选 → emoji / 颜文字 / 符号 | `0`–`3`，缺省 3 / 1 / 1；以前写的 `emoji = true / false` 相当于 3 / 0 |
| `[general] learning` | 候选 → 学习输入习惯 | `true` / `false` |
| `[general] learning_language` | 候选 → 译文 | `off` 为不显示译词 |
| `[general] english_in_chinese` | 候选 → 打拼音时也给英文词 | `true` / `false` |
| `[general] english_max_in_chinese` | 候选 → 英文候选最多几个 | `1`–`4`，缺省 `4` |
| `[general] english_min_position` | 候选 → 英文词最高排到第几位 | `1`–`9`，缺省 `1`（不限） |
| `[general] first_page_characters` | 候选 → 长拼音首页固定单字 | `true` / `false`，缺省 `true` |
| `[model] enabled` | 候选 → 本地整句模型 | `true` / `false` |
| `[model] name`、`zhiyi_tier` | 实验功能 → 用哪个模型、知意的档位 | `name`：`tongbian`（缺省）、`zhiwei`、`zhiyi`，选了没下载的按 `tongbian`；`zhiyi_tier`：`saver`、`balanced`（缺省）、`accurate` |
| `[model] context_memory_seconds` | 只在配置文件里 | 应用不给光标前文时，刚打的字算作前文的时长，缺省 60，0 为一直算 |
| `[general] english_mode` | 中英文 → 英文模式 | `false` 为固定中文 |
| `[shortcut] switch_mode` | 中英文 → 切换键 | 列表，如 `["shift", "control"]` |
| `[general] default_english`、`remember_mode_per_app` | 中英文 → 默认英文模式、按应用记住中英模式 | `true` / `false` |
| `[general] shift_letter` | 中英文 → Shift + 字母 | `passthrough`（交给应用，缺省）或 `compose`（进组句） |
| `[general] punct_commits` | 中英文 → 标点先上屏候选 | `true` / `false` |
| `[general] preedit` | 外观 → 拼音显示位置 | `both`、`inline`、`window` |
| `[general] page_keys` | 快捷键 → 翻页键 | `"[]"`、`",."`、`"-="` |
| `[shortcut] extra_page_keys` | 快捷键 → 额外翻页键 | 列表，如 `["[]", ",.", "-="]`，缺省为空 |
| `[shortcut] highlight_keys` | 快捷键 → 高亮移动键 | 上下或左右 |
| `[shortcut] toggle_punctuation` | 快捷键 → Ctrl + . 切换全半角标点 | 空串为关闭，`"ctrl+."` 为 Ctrl + 句号；也可写成修饰键加字母、数字或句号，如 `"ctrl+shift+."` |
| `[shortcut] toggle_traditional` | 快捷键 → Ctrl + Shift + F 切换简繁输出 | 空串为关闭（缺省），`"ctrl+shift+f"` 为 Ctrl + Shift + F；也可写成修饰键加字母或数字。只有 Windows 用 |
| `[shortcut] translate_selection` | 快捷键 → 翻译选中文字 | 如 `"alt+shift+t"` |
| `[shortcut] predict` | 快捷键 → 按需 AI 联想 | 完整的组合键，设置界面只改修饰键 |
| `[predict]` 一节 | 云服务 | 见下文 |
| `[general] input_log` | 高级 → 记录输入日志 | `true` / `false` |

### 英文候选的应用名单

英文模式只在哪些应用里给候选，由配置文件里 `apps` 一段的名单决定（「设置 → 中英文 → 只在写字的应用里给」打开时）。名单里每一项附有说明，按说明增减应用即可。

## AI 服务

`[predict]` 一节对应「设置 → 云服务」：

```toml
[predict]
enabled = true
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
api_key = "你的 API 密钥"
```

- `automatic`：自动联想，`false` 时只在按手动联想的组合键时请求。
- `debounce_ms`：停键等待，缺省 300。`min_interval_ms`：最短请求间隔，缺省 3000。
- `slots`：AI 候选占第一页末尾几格，0 至 4。
- 在设置窗口里填的密钥另存在数据文件夹的 `.env` 里，不写进 `config.toml`。Linux 上也可以在启动青简之前设置 `QINGJIAN_API_KEY` 环境变量。
- 改开关、地址、模型或密钥会自动切换并撤销旧请求。

从较早版本升级上来的 Linux 配置若仍写着旧的翻译快捷键，改为 `translate_selection = "alt+shift+t"` 并保存；安装更新不会覆盖你写过的设置。

## Linux

Linux 有青简设置程序，也可以直接改配置文件。几个常用写法：

- 换双拼：`[general]` 下 `scheme = "xiaohe"`。
- 繁体：`[general]` 下 `traditional_variant = "taiwan"`；从 0.2.1 及以前升级上来的配置里没有这一行，在 `[general]` 下加上即可。
- 翻页键：`[general]` 下 `page_keys = "-="`。修改 Fcitx5 的全局候选翻页热键不会替代它。
- 不显示译词：`[general]` 下 `learning_language = "off"`。
- 不想留下任何输入记录（例如 Fcitx4 认不出私密输入框时）：`[general]` 下 `input_log = false`、`learning = false`，改完重启青简。

```toml
[general]
scheme = "xiaohe"
page_keys = "-="
```
