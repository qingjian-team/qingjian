---
title: 主题
order: 3
description: 四个内置主题（青简绿、系统蓝、微信绿、樱花）的选择方法，以及自制主题的存放位置、写法（颜色、渐变与图片、阴影、字号与粗细、字体与斜体、文字描边、伸出窗口的装饰、过渡动画、循环动画）与生效方式。
---

主题决定候选窗口的颜色；macOS 与 Windows 上同一个主题显示效果一致。主题只对青简渲染器生效（「偏好设置 → 候选窗口 → 渲染引擎」选「青简渲染器」）。

## 内置主题

| 主题 | 高亮的候选 |
|---|---|
| 青简绿（缺省） | 浅绿色底，文字颜色不变 |
| 系统蓝 | 系统蓝色底，白字 |
| 微信绿 | 微信绿色底，白字；云端候选为蓝色 |
| 樱花 | 粉紫玻璃底、粉色高亮块，宋体候选，词性与释义分两行；左上角趴着的人物、樱花挂件与缎带。只有浅色 |

在「偏好设置 → 候选窗口 → 主题」中选择。青简绿、系统蓝、微信绿都有浅色与深色两套，按同一页的「外观」切换（跟随系统 / 浅色 / 深色）；樱花的底色是浅色图案，始终用浅色。

## 自制主题

自制主题放在用户数据目录的 `themes` 文件夹中（用户数据目录的位置见[数据与日志](../help/data-and-logs.md)），一个主题一个文件夹，文件夹里放一个 `theme.json`：

```text
themes/
└── pink/
    └── theme.json
```

最简单的写法是以一个内置主题为底，只改几种颜色。下面是一个粉色高亮的主题：

```json
{
  "extends": "qingjian",
  "schema": 1,
  "meta": { "id": "pink", "name": "粉色" },
  "variables": {
    "accent": { "light": "#ffb7d5", "dark": "#b0507a" },
    "hl_text": "#ffffff"
  }
}
```

- `theme.json` 里可以写注释（`//` 到行尾，或 `/* … */`），对象和列表的最后一项后面多一个逗号也没关系。
- `extends` 写底子主题：`qingjian`（青简绿）、`system-blue`（系统蓝）、`wechat`（微信绿）或 `sakura`（樱花）。没写的部分都沿用底子主题；底子主题的图片不用拷进自己的文件夹。不写 `extends` 就以青简绿为底。
- `meta.id` 必须与文件夹名相同，且不能与内置主题重名，也不能是 `system`、`light`、`dark`；`meta.name` 是主题列表里显示的名字。
- 颜色写作 `#rrggbb` 或带不透明度的 `#rrggbbaa`；写成 `{ "light": …, "dark": … }` 时浅色与深色外观各用一个。

可改的颜色：

| 名字 | 用在 |
|---|---|
| `surface` | 窗口背景 |
| `accent` | 高亮候选的底色 |
| `text` | 候选词 |
| `gloss` | 译词、拼音行 |
| `faint` | 序号、词性、页码 |
| `fresh` | 生词的译词 |
| `cloud` | 云端候选与云朵 |
| `hl_text`、`hl_gloss`、`hl_faint`、`hl_fresh`、`hl_cloud` | 高亮那一行里对应的颜色 |

## 渐变与图片

主题的窗口、高亮等部分除了纯色，还可以用渐变或图片填充，图片放在主题文件夹里，支持 PNG 与 SVG。例如把窗口背景换成一张可拉伸的边框图：

```json
"windows": {
  "vertical": {
    "fill": { "image": "images/frame.png", "slice": [16, 16, 16, 16], "scale": 2 }
  }
}
```

- `slice` 是图片四边（上、右、下、左）不拉伸的宽度，单位是图片像素：窗口变大时四角保持原样，只拉伸中间；不写则整张拉伸。
- `scale` 表示图片是几倍图，按 2 倍尺寸绘制的图片写 2。
- SVG 图片在任何缩放比例下都清晰，不用写 `scale`。它的宽高就是显示的大小（单位是点），`slice` 也按这个单位写；按实际大小来画，画得过大会多占内存。
- SVG 里的文字不显示，导出前请转成轮廓；SVG 里嵌入的位图和引用的外部文件也不显示，位图请单独用 PNG。
- 渐变写作 `{ "linear": 90, "stops": ["#ffb7d5", "#ffffff"] }`（角度 90 为从左到右）。
- 背景是浅色图片时，深色外观下的文字会看不清，可在 `meta` 中加 `"appearance": "light"`，让主题始终使用浅色。

## 阴影

窗口、高亮等部分可以加投影（`drop-shadow`）或内阴影（`inner-shadow`），写在 `effects` 里，可以叠几层。例如把窗口阴影改淡、给高亮条加一层粉色投影：

```json
"windows": {
  "vertical": {
    "effects": [{ "type": "drop-shadow", "y": 4, "blur": 10, "color": "#00000030" }]
  }
}
```

- `x`、`y` 是偏移（向右、向下为正），`blur` 是模糊程度，`spread` 让阴影向外扩（内阴影为向里缩），单位都是点；不写为 0。
- 阴影跟着形状走：圆角窗口是圆角阴影，边框图片按图片的透明边投影，文字也可以加投影。
- 窗口阴影就是窗口本身的投影，写 `"effects": []` 可以去掉。

## 字号与粗细

`text.styles` 里有三种文字：`candidate`（候选词）、`annotation`（译词与拼音行）、`index`（序号与页码）。每种可以改字号 `size`、行高 `line_height` 与粗细 `weight`：

```json
"text": {
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "weight": "medium" }
  }
}
```

粗细写 `light`（细）、`regular`（常规）、`medium`（中等）、`semibold`（半粗）、`bold`（粗），或 100–900 的数字。系统字体没有某一档粗细时，用最接近的一档。

偏好设置里填了「候选字号」「译文字号」时，盖过主题里 `candidate`、`annotation` 的字号，行高按同样比例缩放；`index`、`preedit` 与表格行高跟着候选字号一起缩放，同一行才对得齐。表格一行里的各格按文字底线自动对齐，主题不用为不同字号调边距。悬浮状态条按候选字号的比例整体缩放，主题不用管。主题里其它自己起名的样式不受影响。

## 字体与斜体

每种文字可以用不同的字体，写 `family`；斜体写 `"style": "italic"`。写在 `text` 下的 `family` 是没单独写字体的文字都用的：

```json
"text": {
  "family": ["PingFang SC", "system"],
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "family": ["Songti SC", "STSong", "system"] },
    "annotation": { "size": 12, "line_height": 15, "family": "Georgia", "style": "italic" }
  }
}
```

- `family` 写一个字体名，或按顺序写几个：用第一个装了的。`system` 表示「偏好设置 → 候选窗口 → 字体」里选的字体（没选就是系统字体）；一个都没装时也用它。
- 字体名写字族名，即系统字体列表里显示的英文名，如 `Songti SC`、`Microsoft YaHei`、`Georgia`。macOS 与 Windows 的字体不同，两边都要好看时把两边的名字都写上。
- 字体里没有的字（比如英文字体遇到汉字）自动用系统字体补上。
- 字体没有斜体时，把正体倾斜显示；汉字的斜体也是这样来的。

字体也可以随主题带：放进主题文件夹，在 `theme.json` 顶层列出来，`family` 写字体本身的字族名：

```json
"fonts": [{ "file": "fonts/LXGWWenKai-Regular.ttf" }],
"text": { "styles": { "candidate": { "size": 18, "line_height": 22, "family": "LXGW WenKai" } } }
```

支持 TTF、OTF、TTC，单个文件不超过 32 MB。只带许可允许再分发的字体（如 OFL 许可的霞鹜文楷），系统自带的字体只写名字、不要拷进主题。

## 文字描边

候选词、译词、拼音行的文字可以加描边，写在对应的文字上：`"stroke": { "width": 1.5, "color": "#c2386b" }`，`width` 单位是点。
描边向字的外侧加宽，不改变文字占的位置。颜色可以按是否高亮区分，例如只给高亮的候选描边：

```json
"stroke": { "width": 1.5, "color": { "if": "highlighted", "then": "#c2386b", "else": "#00000000" } }
```

文字阴影与发光用阴影的写法：在文字上加 `effects`，例如 `[{ "type": "drop-shadow", "blur": 4, "color": "#e85a8c" }]` 就是一圈粉色光晕。
彩色 emoji 不描边。

## 伸出窗口的装饰

装饰图可以探出候选窗口的边缘（比如趴在窗口角上的小角色）。在窗口的 `children` 里加一个绝对定位的框，`inset`（上、右、下、左到窗口边的距离）写负数就伸到窗口外面：

```json
{ "type": "frame", "position": "absolute", "inset": [-24, -24, "auto", "auto"], "width": 48, "height": 48,
  "fill": { "image": "images/cat.png", "scale": 2 } }
```

上例是右上角探出 24 点。候选窗口仍按窗口本身贴着光标摆放，装饰不影响位置，贴近屏幕边缘时装饰可能露在屏幕外。装饰不挡鼠标。

## 词性与释义分开排

译文默认排成一行（`n. pure letters · …`）。想让词性和释义各占一行，在候选里写两个译文节点，用 `tones` 指定每个只画哪几种内容，`senses` 指定只画前几个义项：

```json
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["pos"], "senses": 1, "height": 15,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" },
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["gloss", "fresh"], "senses": 1,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" }
```

- `tones` 可写：`pos`（词性）、`gloss`（释义）、`fresh`（生词的释义）、`separator`（义项之间的 ` · `）、`faint`（日文的假名注音）、`code`（辅码）。不写全画。
- 有的候选没有词性（比如 emoji），给词性那一行写上 `height`，空着也占住位置，几列才对得齐。
- 词性和分隔符的颜色可以单独写 `pos`、`separator`，不写用 `faint`。

## 显示输入状态

候选窗口里可以显示当前是中文还是英文、简体还是繁体、标点全角还是半角，以及输入方案名。显示什么字由主题自己写：
用 `when` 让两个文字只出现一个，方案名用 `"bind": "mode.scheme"`：

```json
{ "type": "text", "text": "中", "when": "!mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "英", "when": "mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "繁", "when": "mode.traditional", "font": "index", "color": "@gloss" },
{ "type": "text", "bind": "mode.scheme", "font": "index", "color": "@faint" }
```

- `mode.english`：英文模式；`mode.traditional`：繁体输出；`mode.full_width`：标点是全角。前面加 `!` 表示「不是」。
- `mode.scheme` 是方案名，如「全拼」「小鹤双拼」「五笔（86） + 全拼」；写在 `when` 里表示「有方案名」。
- 这几个节点放在窗口的 `children` 里；`children` 要整组写出来（只写新加的会把原来的内容整个换掉），可以从内置主题的 `theme.json` 复制一份再改。

## 悬浮状态条（Windows）

Windows 屏幕右下角的悬浮状态条也可以按主题画，写法与候选窗口相同：在 `status` 下写 `root`，格子用 `repeat` 绑定 `cells` 一格一格展开（现在是「中 / 英」「标点」「设置齿轮」三格，点哪格就是切换哪一项）：

```json
"components": {
  "status-cell": { "type": "frame", "padding": [4, 8], "children": [
    { "type": "text", "bind": "text", "font": "candidate",
      "color": { "if": "emphasized", "then": "@cloud", "else": "@gloss" } },
    { "type": "icon", "icon": "gear", "when": "gear", "size": 15, "color": "@gloss" }
  ] }
},
"status": {
  "root": { "type": "frame", "fill": "@surface", "radius": 8, "children": [
    { "type": "frame", "width": 24, "height": 24, "fill": { "image": "images/sakura.svg" } },
    { "type": "repeat", "bind": "cells", "component": "status-cell" }
  ] }
}
```

- 每一格里可用：`text`（这一格的文字）、`emphasized`（当前生效的模式或全角标点）、`gear`（设置齿轮那一格）、`first` / `last`。
- 整条状态条也能用「显示输入状态」里的 `mode.english` 等，自己决定画什么。
- 格子之间可以夹装饰，点到装饰算前一格。
- 不写 `root` 时状态条按原来的样子画，只用 `status` 里的颜色与尺寸。macOS 没有悬浮状态条。

## 过渡动画

高亮换到另一个候选时，高亮条可以滑过去，而不是直接跳（三个内置主题都是这样）。给高亮条起个名字（`id`）并加上 `transition`：

```json
{ "type": "frame", "id": "highlight", "transition": { "duration": 120, "easing": "ease-out" },
  "when": "highlighted", "fill": "@accent", "radius": 4 }
```

- 前后两次显示中同名的部分会从旧的位置、大小、不透明度过渡到新的；颜色不过渡。
- `duration` 是时长（毫秒）；`easing` 是快慢曲线：`linear`（匀速）、`ease`、`ease-in`（先慢后快）、`ease-out`（先快后慢）、`ease-in-out`，
  或写四个数 `[x1, y1, x2, y2]`（同网页的 `cubic-bezier`）。
- 偏好设置里关了「过渡动画」，或系统开了「减少动态效果」（macOS：系统设置 → 辅助功能 → 显示；Windows：设置 → 辅助功能 → 视觉效果 → 动画效果关闭）时不播动画。

## 循环动画

装饰可以一直轻轻地动，比如飘动的花瓣、呼吸的光晕。在节点上加 `animation`，用关键帧写它在一轮里怎么动：

```json
"animation": {
  "duration": 2400, "easing": "ease-in-out",
  "keyframes": [
    { "at": 0, "y": 0, "rotate": 0 },
    { "at": 0.5, "y": 3, "rotate": 20 },
    { "at": 1, "y": 0, "rotate": 0 }
  ]
}
```

- `duration` 是一轮的时长（毫秒），`at` 是关键帧在一轮里的位置（0 到 1）。
- 可以动的有：`x`、`y`（平移，点）、`rotate`（旋转角度，绕自身中心）、`scale`（缩放）、`opacity`（不透明度）。关键帧里没写的就保持不动。
- 动的是这个部分连同它里面的所有内容，不会挤动别的部分。
- 缺省一直循环；写 `"loop": false` 只播一次，停在最后的样子。
- 候选窗口出现时从头播，打字过程中不重播；关了「过渡动画」或开着「减少动态效果」时不动。

## 生效方式

- 新放入的主题：重新打开偏好设置后出现在「主题」列表中。
- 修改正在使用的主题：保存 `theme.json` 后，在输入状态下约一秒内生效，可以边改边看。
- 主题文件写错时，该主题不出现在列表中；正在使用的主题读不进来时，候选窗口改用青简绿。原因记录在运行日志中（见[数据与日志](../help/data-and-logs.md)）。
