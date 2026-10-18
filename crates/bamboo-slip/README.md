# bamboo-slip

自绘 UI 引擎：场景树（taffy 布局，每个节点画什么）→ 预乘 alpha 的 RGBA 位图，各平台只负责把位图贴到窗口上。

- 栅格：tiny-skia（纯 CPU）
- 文字：cosmic-text（fontdb + rustybuzz + swash），带回退链、彩色 emoji、SF 的光学字号与 `trak` 字距
- 字体：不扫系统字体目录，按平台清单只加载界面字体、中日文与 emoji 几个文件
- 效果：圆角、边框、渐变、九宫格图片 / SVG、投影与内阴影、整棵子树的不透明度
- 动画：跨帧按配对名做位置过渡，关键帧循环动画（只重画动的那一块）
- 控件与宿主（`host` 特性）：设置行、开关、下拉、输入框、对话框等控件，winit 开窗收事件、softbuffer 贴图

在青简仓库里有两个使用方：候选窗与状态条（`crates/qingjian-render`）、三平台共用的设置程序（`apps/settings`）。

## 许可

双许可，见 [LICENSE.md](LICENSE.md)。
