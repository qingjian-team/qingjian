# 图标

- `logo.png`（866×866，带透明通道）：应用图标源文件。`apps/macos/scripts/bundle.sh` 打包时用 `sips` + `iconutil`
  生成 `Qingjian.icns`，生成物不进仓库。
- `menu.svg`：macOS 输入法图标源文件，黑色键帽镂空四片竹简（模板图，系统只取 alpha）。`menu.pdf` 是它导出的
  22×16pt 矢量版，打包时拷成 `qingjian-menu.pdf`，Info.plist 的图标键都指向它。为什么是这个形式和尺寸见
  `docs/design/architecture.md`「Info.plist 约定」。改了 svg 重新导出：

  ```sh
  rsvg-convert -f pdf --page-width 22pt --page-height 16pt -w 22pt -h 16pt assets/icon/menu.svg -o assets/icon/menu.pdf
  ```
- `windows/mode-zh.svg` / `mode-en.svg` / `mode-caps.svg`：Windows 任务栏的中 / 英 / A 图标源文件（16×16 画布，单色）。
  `windows/render-mode-icons.sh` 用 rsvg-convert + magick 栅格化成 16 / 20 / 24 / 32 四档的 8 位 alpha 蒙版，
  写到 `apps/windows/tsf/resources/mode/`，DLL 用 `include_bytes!` 嵌入、运行时按任务栏深浅色填色（`com/mode/icon.rs`）。
  改了 svg 重跑脚本，生成物随仓库提交。
