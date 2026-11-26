# Android 物理键盘

对应 [#1100](https://github.com/qingjian-team/qingjian/issues/1100)。面向 Android 标准 KeyEvent 与 InputMethodService，不按厂商或保护套型号分支。

## 输入路径

`service/input/physical/Keyboard` 只处理 Android 按键、修饰键状态和消费配对；`Composer.onPhysical` 把操作交给 Android Rust `Session::physical_key`，再将帧交给同一份 InputConnection 更新路径。排序、字典、方案解码和标点仍由既有 Engine API 完成。Android 不引入带 native-tls 的 qingjian-platform。

物理键盘使用当前语言、双拼/五笔/注音设置。九宫格是触屏布局，物理字母键使用全拼；回到触屏恢复九宫格。英文候选沿用「英文键盘也给候选」开关。密码、邮箱、数字、非文本字段透传；网址搜索字段可组字，标点保持半角。

`PhysicalPage` 维护每页 5 项、页起点与高亮；数字对应当前页，↑↓移动高亮，PageUp/PageDown翻页，←→/Home/End调用 Core 光标 API。内容变化回到第一页；同一内容的重画保留位置。表达式、码点、注音和双拼所需的字符优先按 Core 的字符判定进入缓冲区。

## 候选显示

复用 `qingjian-render::Layout::Vertical`、同一主题与字号。序号、高亮、页码、箭头都由桌面渲染器绘制。`StripPainter` 只切当前页，返回位图与点击区域；`PhysicalCandidatesView` 贴位图、补 Android 彩色 emoji、转发候选和箭头点击。触屏候选条仍用 Layout::Strip，两种显示各自复用像素缓冲。

显示承载为透明的 IME input view，仅容纳按内容测量的桌面候选位图，不创建系统 candidatesArea，避免隐藏视图仍占高度。物理候选有内容时通过 `showWindow(true)` 显示标准输入法窗口，避免 Android 16 客户端把已请求但未显示的状态去重；`onEvaluateInputViewShown` 同时允许物理候选，外层留导航栏安全距；Insets 不缩小或平移应用，触摸范围只包含候选小框。提交后清空小框再收窗，软键盘在下一次需要时创建；配置变化仍按当前模式重建。

通过 `requestCursorUpdates(IMMEDIATE | MONITOR)` 订阅当前输入连接；`onUpdateCursorAnchorInfo` 用应用给出的矩阵把插入光标换成屏幕坐标，宿主再减去自身屏幕原点。换输入框、重启连接时清空旧锚点并重新订阅，结束时取消；不可见光标隐藏候选，不支持位置上报的应用使用左下角回退。

`AnchoredFrame` 铺满透明 IME 窗口，保留 `isFullscreenMode=false`，不会进入全屏提取编辑。定位使用渲染器 `content_x/y/width/height`，从可见卡片位置扣掉位图的阴影留白；优先贴在输入行下方，空间不足上翻，右侧及系统栏/刘海边界内收。应用滚动或移动时依赖持续坐标通知更新；只给内容矩形触摸区域。沉浸式应用的导航条由 Android 管理，输入时可能暂时显示，提交后回到应用的系统栏状态。

## 上屏与模式提示

Rust Session 的 `physical_input` 区分触屏与物理输入。物理选词完成不生成上屏联想、联系人详情或空组字补全，实际触摸候选也沿用此模式；切回触屏恢复原设置，不改用户 follow_ups。Kotlin `Composer.physicalInput` 同步会话模式，物理状态下不触发触屏补全轮询。

切换语言时复用共享 `Renderer::render_status` 画单格中英提示，正确传入 Mode.english。提示不进入候选列表、不接收触摸，900 ms 自动清除；继续输入、换焦点或结束服务取消回调，旧期限不会关闭新候选窗。外观预览通过物理键盘连接状态选择同一渲染器的 Vertical/Strip 布局，缓存区分模式，连接变化时刷新当前预览及主题卡片。

## 生命周期与按键

输入设置在 onStartInput 应用；Engine 异步装好时补交，不等待 onCreateInputView。相同的输入方案不反复重设，避免视图重建清掉组字。onFinishInputView 在物理输入时不调用会结束组字的默认实现；onFinishInput 清会话。旋转或键盘配置变化重建显示并重新评估系统软键盘偏好。

消费的 down 对应 up 同样消费，按住重复不再透传给应用；从组字开始的 Backspace 重复删空后停止，重新按下可删应用文本。单击 Shift/Ctrl 只有同设备、同一次 downTime、未取消且未夹杂其他键才切换；修饰键本身 down/up 透传，使应用组合键完整。Ctrl/Alt/Meta 组合保留给应用，透传前落下当前原始组字，避免应用移动选区后下一键把旧组字覆盖回来。热插拔清理尚未抬起的消费与修饰键状态，重新评估显示。

快捷键设置使用与桌面相同的语义：可选单击 Shift、单击 Ctrl、Ctrl+Alt+Space；可交换 Space 与 Enter 的候选/原样上屏功能。切换当前语言与既有语言搭档，不另存一份英文模式。

## 验证

回归和人工步骤见 [物理键盘验收](../notes/android-physical-keyboard-testing.md)。
