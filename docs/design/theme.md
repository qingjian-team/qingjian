# 主题（预览版）

候选窗口与 Windows 悬浮状态条由青简渲染器（`crates/qingjian-render`）按主题画成位图，mac 与 Windows 用同一个渲染器、同一份主题，画出来逐像素一致。
这里记格式全貌与实现，写给维护的人；用户怎么写主题见 `docs/user/settings/themes.md`。本文只写已经做到的，做不到的列在末尾「预览版的限制」。

## 主题文件

- 一个主题一个目录：`theme.json`，加上它引用的 `images/…`（PNG、SVG）与 `fonts/…`（TTF / OTF / TTC）。包内路径一律相对 `theme.json`，拒绝 `..` 与绝对路径。
- 内置主题四个，随 crate 编进来（`crates/qingjian-render/themes/<id>/theme.json`）：`qingjian` 青简绿（缺省）、`system-blue` 系统蓝、`wechat` 微信绿、`sakura` 樱花；系统蓝、微信绿 `extends` 青简绿、只改颜色变量，樱花整套重写了窗口、组件与状态条（只有浅色）。
  内置主题的图片用 `include_bytes!` 编进程序（`theme/mod.rs` 的 `BUILTINS` 里每个主题一张「路径 → 内容」表，与用户主题走同一套解码），不带字体文件、只用系统字体；测试检查内置主题用到的图片都编进去了。用户主题 `extends` 内置主题时，主题目录里没有的图片到那个内置主题编进程序的文件里找（只看顶层 `extends` 那一层）。
- 用户主题放在 `<用户数据目录>/themes/<id>/`，由 `ThemeLibrary` 与内置主题合成一个列表（内置在前、用户按 id 排）。目录名必须等于 `meta.id`，不能与内置主题重名，
  不能是 `system` / `light` / `dark`；读不进来的跳过并记警告。设置界面（mac 偏好设置、Windows 设置程序「候选窗口」页）按显示名列出，写回 `[general] theme` 的 id。

### 为什么用 JSON

主题是嵌套很深的图层树：TOML 写深层嵌套要么是一长串表头、要么是不能换行的内联表；YAML 缩进错一格结构就变，还有隐式类型转换；自定义格式的解析器、高亮、补全都得自己做。
设置是平铺的键值，仍用 TOML（`config.toml`）。

手写时可以加注释（`//`、`/* */`）与尾逗号（JSONC，同 VS Code 的 `settings.json`）：`theme/jsonc.rs` 在解析前把它们换成空格、保留换行，报错的行号、列号对得上原文件。

## 格式（schema 1）

内置主题全部用下面的写法写成，与改造前写死的排版逐像素一致（快照测试）。

顶层：

| 键 | 内容 |
|---|---|
| `extends` | 以某个内置主题为底：对象逐键合并，数组与标量整个替换；目前只能继承内置主题。不写就以青简绿为底，继承链上每一层都一样（内置主题也是），只有青简绿自己是最底层 |
| `schema` | 格式版本，整数，只在不兼容改动时加一；比渲染器认得的新时记警告、按认得的部分画 |
| `meta` | `id`、`name`、`author`、`license`；`appearance: "light" \| "dark"` 锁定外观（见下） |
| `variables` | 颜色变量，`"#rrggbb[aa]"` 或 `{ "light", "dark" }`，节点里用 `@名字` 引用 |
| `text` | `gamma`（可分深浅）、`family`（缺省字族）、`styles`（命名样式：`size`、`line_height`、`weight`、`family`、`style`） |
| `fonts` | 随主题带的字体文件 `[{ "file": "fonts/x.ttf" }]` |
| `components` | 可复用的节点，`use` / `repeat` 按名字引用 |
| `windows` | `vertical` / `horizontal` 两个根节点，按用户选的排布取一个 |
| `status` | 悬浮状态条（只有 Windows 有）：字体、内边距、圆角、各处颜色、`effects`，排法固定；写了 `root` 就按节点树画（见「状态条」） |

节点：

| `type` | 属性 |
|---|---|
| `frame` | `direction`（row / column）、`fill`、`border`、`radius`、`children`；带 `table: { row_height }` 时排成表格 |
| `text` | `bind` 或 `text`（固定文字）、`font`、`color`、`stroke` |
| `icon` | `icon`（cloud / gear）、`size`、`color`；盒子缺省与图标同大，图标垂直居中 |
| `preedit` | `font`、`typed` / `rest` / `struck` 三种拼音颜色、`caret: { width, color }`、`stroke` |
| `annotation` | `bind`、`font`、`gloss` / `fresh` / `faint` 三种颜色（`pos`、`separator` 可单独写，缺省同 `faint`）、`tones`（只画哪几种片段）、`senses`（只画前几个义项）、`stroke` |
| `use` | `component`：引用组件，这里写的盒子属性盖过组件根节点的 |
| `repeat` | `bind`（`candidates`）、`component`：每项候选实例化一份 |

所有节点都可写：`when`（显示条件）、`id` + `transition`（过渡）、`animation`（循环动画）、`margin` / `padding`（一个数或 `[上, 右, 下, 左]`，外边距可写 `"auto"`）、`gap`、
`width` / `height` / `min_width`、`position: "absolute"` + `inset`、`align_self`（start / end / center / stretch）、`span: "row"`（表格里横跨整行）、`opacity`（整棵子树）、`effects`。长度单位是点。

### 数据

- **条件**：`when` 是数据字段名，`!` 取反，`a|b` 任一成立。候选项里：`highlighted`、`cloud`、`annotation`、`first`、`last`；
  整帧：`preedit`、`trailing`、`trailing.cloud`、`page`、`candidates`、`annotations`（任一候选有译文）、`highlighted`、`highlighted.annotation`；
  输入状态：`mode.english`、`mode.traditional`、`mode.full_width`、`mode.scheme`（有方案名）。
- **绑定**：文字 `index`、`text`（候选项）、`page`、`trailing.text`、`mode.scheme`（方案显示名）；译文 `annotation`（候选项）、`highlighted.annotation`；列表 `candidates`。
- **条件颜色**：`{ "if": "highlighted", "then": "@hl_text", "else": "@text" }`，条件写法同 `when`。系统蓝、微信绿的高亮白字靠它；青简绿里 `hl_*` 与普通颜色同值。
- **译文片段**：壳把译文拼成一串带种类（`Tone`）的片段：`gloss` 译文、`fresh` 生词译文、`pos` 词性、`separator` 义项间的 ` · `、`faint` 假名注音、`code` 辅码。
  `tones` 按种类挑、`senses` 按分隔数截前几个义项，词性与译文分两行排就是两个节点；挑完是空的也留着节点，主题给它定高就照样占行。系统绘制的旧路径把 `pos` / `separator` 画成与 `faint` 同色。
- **输入状态**（`Frame::mode`）：渲染器只给事实（三个布尔与方案名），显示什么字由主题用 `when` 分支写，渲染器里没有「中 / 英」这类文案。
  mac 壳组装候选帧时填（中英看 Caps Lock，与菜单栏一致；简繁、标点、方案看配置；英文模式标点算半角）；
  Windows 由 Router 现算（与状态条同一来源），经 `CandidateSink::show` 交给 UI 线程，不进 Server ↔ DLL 协议。方案名 `qingjian_platform::scheme_name`：同状态条的 `scheme_label`，单开全拼时也写「全拼」。
- **设置里的字号**：`[general] candidate_font_size` / `annotation_font_size`（0 用主题的，不限范围）由壳经 `Theme::with_text_sizes` 盖到主题上：
  `candidate` 与 `annotation` 换成设置的字号，行高同比例；`index`、`preedit` 与表格 `row_height` 按候选字的比例缩放（`theme/text_sizes.rs`）。
  表格行高是下限（`minmax(row_height, auto)`），内容更高时撑开。
  状态条不逐个样式改字号：`render_status` 去掉设置的字号，把整条的倍数乘上候选字的比例，文字、图标、边距、点击边界一起缩放。
- **状态条**：`status.root` 是一棵与候选窗口同写法的节点树，`repeat` 绑定 `cells` 展开格子（Windows 现在是模式、标点、齿轮三格）。
  格子里的条件 `emphasized`（当前模式、生效的全角标点）、`gear`、`first`、`last`，文字绑定 `text`；整条能用 `mode.*`（`Renderer::render_status` 收一份 `Mode`，Windows 的 `StatusView` 带着，与候选窗口同一个 `Router::input_mode`）。
  点击按格：每格第一个产出的节点的右边界（根坐标），没画出来的格同前一格，最后一格延到内容右边；格子之间的装饰算前一格（`renderer/status/tree.rs`）。
  没写 `root` 走原来的固定排法（内置主题都是），Windows 的状态条快照不变。
- **表格**：`repeat` 出来的每份组件是一行，组件根节点的子节点依次是各列，每列取各行最宽，行高至少 `row_height`；末尾自动补一列吃掉剩余宽度，
  `span: "row"` 的格子（高亮条）因此能横跨整个表格。列间距用各格的外边距写。竖排三列对齐就靠它。
  一行里的各格（不含 `span`）按各自第一段文字的基线对齐：布局后把基线偏高的格子往下补上边距再排一遍，差不到一像素的不动（`scene/baseline.rs`）；
  所以格子的上边距只能把它往下压，不能让它高过同行的基线。

### 填充与效果

- **填充**：颜色（含条件颜色）；`{ "linear": 角度, "stops": [...] }`（CSS 角度）；`{ "radial": [cx, cy], "stops": [...] }`（圆心按比例、半径到最远角）；
  色标是颜色（均分）或 `[颜色, 位置]`；`{ "image": "images/x.png", "slice": [上, 右, 下, 左], "scale": 2 }`（slice 为图片像素的九宫格切边、不写就拉伸，scale 为一个点对几个图片像素）。
  `border: { width, color }` 画在内侧。图片随主题目录加载（`Theme::from_dir`），PNG 边长 ≤ 4096。
- **SVG**：路径以 `.svg` 结尾的图片用 resvg 解析（`theme/svg_image.rs`），单位当作点、不管 `scale`，`slice` 也按 SVG 单位写。加载时解析一次（文件 ≤ 4 MB、原始尺寸 ≤ 4096），
  画的时候才栅格：拉伸按盒子的像素尺寸直接栅格，九宫格按原始尺寸 × 倍数栅格再切，每张最多缓存 8 种尺寸，之后与 PNG 同一条贴图路径（图片框的离屏缓存也一样）。
  不读外部文件（`<image href>` 的字符串解析器换成一律返回空，否则 usvg 会按路径读盘），不开 `raster-images`（内嵌位图不解码）、不开 `text`（`<text>` 不画、不带字体库）。
- **效果**：`effects: [{ "type": "drop-shadow" | "inner-shadow", "x", "y", "blur", "spread", "color" }]`，按写的顺序画。
  形状取节点自己画出来的 alpha（圆角框、九宫格图片的透明边、文字），不含子节点；自己不画东西的容器（没填充的框、译文、拼音行）取子节点；`spread` 只对框生效。
  投影垫在节点底下、半透明填充会透出来；内阴影压在填充上、子节点下。
  窗口阴影就是根节点的投影：渲染器按投影伸出的距离在位图四周留边，壳按内容区对齐光标，mac 面板关掉系统阴影（系统绘制退路仍用系统阴影）。状态条的阴影写在 `status.effects`。
- **伸出窗口的装饰**：负 `inset` 的绝对定位节点（`"inset": [-24, -24, "auto", "auto"]`）。布局后把所有自己画东西的节点（连同投影、描边）并成画出范围，位图按它开，
  根节点的盒子是内容区（`Rendered::content_*`）；壳照旧按内容区对齐光标、夹进屏幕，装饰可以伸到屏幕外。
  鼠标整个穿透：mac 面板本来就 `ignoresMouseEvents`，Windows 候选窗加 `WS_EX_TRANSPARENT`。
- **锁定外观**：`meta.appearance` 写了就不跟随外观设置切换；用浅色图片做底的主题要锁浅色。

### 文字

- **字重**：`weight` 写 100–900 或 `thin` / `extralight` / `light` / `regular` / `medium` / `semibold` / `bold` / `extrabold` / `black`。
  mac 的 SF 是可变字体、苹方是多字重集合；Windows 另加载 Segoe UI 与雅黑 / 正黑的粗细体文件，缺的字重挑最近的一档。
- **行内改样式**：节点的 `font` 可以写 `{ "base": "index", "size": 12, "weight": "semibold", "style": "italic" }` 在某个样式上改几项，只改字号时行高等比缩放；字族不能在这里改，要换字族就定义命名样式。
- **字族与斜体**：`text.family` 是缺省回退链，样式上 `family`（字符串或数组）覆盖它，`style: "italic"` 斜体；`system` 是界面字体（用户选的字体，没选就是平台界面字体）。
  加载主题时把缺省链与各样式的链（按样式名排）去重成字族表（`theme/font/families.rs`），`FontSpec` / `TextStyle` 里只存序号，仍可按值复制。
  渲染器换主题时给每条链挑第一个字体库里有的字族，一个都没有用界面字体；字体里缺的字由 cosmic-text 按 locale 回退。没有斜体面的字体倾斜 14° 合成（汉字也是）。
- **字体加载**：字体库不扫系统字体目录（fontdb 全扫几百毫秒、几十 MB），所以主题写到的系统字族由壳查文件：`Renderer::load_theme_fonts(theme, family_files)`，
  mac 走 CoreText、Windows 走 DirectWrite（与用户在设置里选字体同一个查法），字体库里已有的字族不再查。随主题带的字体（单个 ≤ 32 MB）在换主题时先加载。
  同一个文件只加载一次，不卸载。
- **描边**：`text` / `annotation` / `preedit` 上写 `stroke: { width, color }`（颜色可写条件）。cosmic-text 取字形轮廓（`SwashCache::get_outline_commands`）拼成一条路径，
  圆角接头按两倍宽描一次、字形压在上面，露出的是向外的宽度，不占排版宽度；位图 emoji 没有轮廓不描。文字阴影、发光就是文字节点上的 `effects`（形状含描边）。

### 动画

- **过渡按 `id` 配对**（同 Figma 的 Smart Animate 按图层名配对）：新一帧里带 `id` 且写了 `transition` 的节点，在上一帧找同 `id` 的节点，从它的位置、大小、不透明度插值到新的；颜色、字号不过渡。
  `repeat` 展开出同一个 `id` 的多份时按出现顺序加序号（`word#0`、`word#1`）各自配对；只出现一次的不加，所以只在高亮行里出现的高亮条换行后仍配得上。上一帧没有同 `id` 的就直接出现。
- `transition: { "duration": 毫秒, "easing": … }`：`easing` 为 `linear`、`ease`、`ease-in`、`ease-out`、`ease-in-out`，或 `[x1, y1, x2, y2]`（同 CSS `cubic-bezier`）。
- **循环动画**：`animation: { "duration", "easing", "loop", "keyframes": [{ "at": 0–1, "x", "y", "rotate", "scale", "opacity" }] }`。每个属性只在写了它的关键帧之间插值，
  开头、结尾没写时按缺省值补（同 CSS 缺 0% / 100%）；`loop: false` 播一轮停在最后。变换作用于整棵子树、不影响布局（同 CSS `transform`）。时钟从窗口出现算起，打字过程中不重置。
- **运行方式**：`Renderer` 留住上一帧的场景树与布局；`Rendered::next_frame` 告诉壳多久后要下一帧（没有动画为 `None`），壳到点调 `tick`，只按插值重画、不重建树、不重排版。
  新数据到了，新内容立刻画出，只有带过渡的节点从旧位置出发，按键到候选出现的延迟不变。画出范围按动画能到的最远处算（过渡取起止的并，循环取最大平移、旋转后的外接圆），动画中途位图不变大小。
  过渡 60 帧、循环动画上限 30 帧；窗口隐藏时停下，没有动画时不开定时器。系统开了「减少动态效果」或设置里关了 `animations`（`Theme::with_animations`）时过渡直接跳到终点、循环动画停在第一帧。
  壳：mac 用主线程 `NSTimer`，Windows 在 UI 线程 `SetTimer`。
- **局部重画**：循环动画用脏矩形（`renderer/partial.rs`）：动画节点把树序切成几段，不动的各段画进图层缓存，每帧只在动画节点上一帧与这一帧占的区域里按原先后叠「段、动画节点、段……」，
  与整张重画只差半透明叠加的取整（测得最多 3 / 255）。动画节点的上级有半透明容器、拿子节点当阴影形状的容器或别的动画节点，或同时有过渡在播时，退回整张重画。
  效果遮罩与图片填充的框按「节点 + 位置大小 + 画布大小」缓存（`scene/cache_key.rs`）；九宫格里原图同色的格子按纯色填。

## 实现

```
theme.json + Frame 数据
  → 实例化（renderer/build）：按 when 取舍、按 bind 填数据，use / repeat 展开组件
  → 布局：Taffy（flexbox）算出每个节点的矩形，表格自己按列取最宽
  → 绘制：tiny-skia 按树序画，阴影在离屏遮罩上做；文字 cosmic-text 整形 + swash 栅格
  → 位图 + 内容区 + 各候选 / 状态条格子的命中区域
```

- 内部全用像素：主题里的点数进来先乘缩放倍数。文字节点的盒子顶边就是行框顶边，字形在行高里垂直居中。
- **容错**：加载时检查颜色变量、文字样式、组件引用，找不到的记警告；渲染时颜色退回透明、样式退回 16/19、组件不画。不认识的键直接忽略。
  读不进来的图片不画，读不进来的字体按回退链往后找。正在用的主题整个读不进来时用青简绿。
- **热加载**：目录戳（各 `theme.json` 的修改时间与大小）变了就重读。mac 在激活期间每秒的配置检查里 `refresh()`；Windows Server 在热加载轮询里比戳，变了把设置重发给 UI 线程重读。保存后约一秒生效。
- **配置键**：`[general] appearance` 是外观（system / light / dark），`[general] theme` 是主题 id（缺省 `qingjian`）。2026-09-18 之前外观写在 `theme` 里：
  没写 `appearance` 且 `theme` 是这三个词之一时按外观读、主题用内置的，所以这三个词不能当主题 id。读取一律走 `GeneralConfig::appearance()` / `theme_id()`。
  Server ↔ DLL 帧协议里的字段在 Rust 里改名为 `appearance`，线上仍叫 `theme`，旧 DLL 照常能解析，协议版本不变。
- **性能**（2026-09-18，mac、release、2 倍屏、字形缓存已热）：内置主题新帧约 6.6 ms；能力展示主题（九宫格 + 十几处阴影）新帧约 15.5 ms、过渡帧约 7.3 ms、
  循环动画帧约 0.2 ms（第一帧切段约 7 ms）。

## 测试与预览

- 快照测试 `crates/qingjian-render/tests/snapshots.rs`：样例帧（`tests/scenes/`）按浅 / 深色画成位图，与 `tests/snapshots/<os>/` 的基准逐像素比；字体环境与基准不一致时跳过。
- 能力展示主题 `crates/qingjian-render/tests/themes/showcase/`（程序生成的图片）覆盖九宫格、阴影、伸出装饰、过渡与循环动画。
- 主题功能 `tests/theme/`：
  - `fonts.rs`：系统字族经壳查文件加载、回退链、随主题带的字体；
  - `status.rs`：状态条节点树按 `cells` 展开、点击边界、`mode.*` 分支；
  - `svg.rs`：九宫格四角按倍数、超出主题目录的路径不读；`theme/svg_image.rs` 里测了不读外部文件。
- 离线预览：`cargo run --release -p qingjian-render --example preview -- --theme <主题目录>`，把样例帧画成 PNG；主题写的系统字族在 `--font-dir` 里按名字找。

## 预览版的限制

- 状态条的节点树画法不播动画；mac 没有悬浮状态条（菜单栏图标），主题的状态条只在 Windows 上显示。
- 翻页只能显示页码文字；画出来的翻页按钮点不了。
- 图片是 PNG 与 SVG；SVG 里的文字、内嵌位图不画；没有 APNG 帧动画。
- 没有窗口出现 / 消失动画、混合模式、图层模糊、渐变字、变体覆盖（用 `when` 分支与条件颜色代替）。
- 背景毛玻璃做不到：自绘位图读不到窗口后面的内容。
- 主题只能以目录形式安装，`extends` 只能继承内置主题。
- Linux 上候选窗由 Fcitx5 / IBus 画，主题不生效。
