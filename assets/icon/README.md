# 图标

- `qingjian-mark.svg`：README 页头使用的透明竹简图标，与官网品牌图标一致。
- `logo.png`（1024×1024，带透明通道的竹简）：Linux 图标的源文件（见下面 `linux/hicolor`），官网等处也用这个透明版。
- `macos/app.svg`：macOS 应用图标源文件（输入法包与设置程序共用），图形同 Windows 的 `windows/app.svg`，按 macOS 图标网格摆
  （1024 画布居中 824 的圆角方块；不照网格画，macOS 26 会给图标垫一块灰底）。`macos/render-app-icon.sh` 栅格化成 `macos/app.png`
  随仓库提交，`apps/macos/scripts/bundle.sh` 打包时用 `sips` + `iconutil` 由它生成 `Qingjian.icns`。
- `fcitx4.png`：由 `logo.png` 缩放到 48×48，供 Fcitx4 经典面板及托盘组件使用。
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
- `windows/app.svg` / `app-16.svg`：Windows 应用图标源文件（安装器、开始菜单、Server exe、输入法列表共用的 `qingjian.ico`）。
  深绿圆角底衬白竹简：原图细长、透明底，缩到任务栏的 16px 只剩两根细绿条，深色任务栏上看不清；16px 那档单独画了像素对齐版。
  `windows/render-app-icon.sh` 栅格化成 16–256 八档写到 `apps/windows/tsf/resources/qingjian.ico`，改了 svg 重跑，生成物随仓库提交。
- `windows/app-mono-dark.svg` / `app-mono-light.svg`：Windows 任务栏单色图标源文件（可选风格，彩色的 `app.svg` 仍是默认）。
  线稿外框 + 官方竹简几何，左列中段保留品牌绿点缀，深浅任务栏都透底色。两版几何一致，只有外框线宽差一档
  （深 64 / 浅 72）：白线在深色任务栏上有光晕效应、看着比黑线粗约半像素，两档在真机实拍里基本等重。
  生成物 `apps/windows/tsf/resources/qingjian-mono-{dark,light}.ico` 与 `qingjian.ico` 同构（16–48 存 BMP，64 起存 PNG），
  生成物随仓库提交。
- `linux/hicolor/<尺寸>x<尺寸>/apps/qingjian.png`：Linux 托盘与应用图标，freedesktop hicolor 八档
  （16/22/24/32/48/64/128/256）。`linux/render-icons.sh` 从 `logo.png` 等比栅格化（需要 magick），生成物随仓库提交，
  源码安装与预编译包都不带栅格化依赖；`apps/linux/scripts/files.py` 安装时按目录尺寸逐档拷到 `$XDG_DATA_HOME/icons/hicolor/`。
  改了源图重跑脚本；不要把 1024px 源图直接放进尺寸目录，不做缩放的托盘（如 Waybar）会显示成巨图。

## Linux 发行版打包

八档 PNG 已栅格化并随仓库提交，打包时不需要 ImageMagick，逐档装进系统 hicolor 主题即可，文件名保持 `qingjian.png`：

```sh
for size in 16 22 24 32 48 64 128 256; do
  install -Dm644 "assets/icon/linux/hicolor/${size}x${size}/apps/qingjian.png" \
    "${pkgdir}/usr/share/icons/hicolor/${size}x${size}/apps/qingjian.png"
done
```

- 目录名的名义尺寸必须与 PNG 实际像素一致：不要把 1024px 的 `logo.png` 塞进某个尺寸目录，也不要只装最大一档，
  否则不做缩放的托盘（如 Waybar）会显示成巨图，或找不到 16px 小图标（#547）。
- 装进 `/usr/share/qingjian/resources/assets/` 的那份是程序资源副本，图标主题不读这个路径，不能拿它代替 hicolor 安装。
- 多数发行版的 `gtk-update-icon-cache` hook（或 file trigger）会自动刷新缓存；没有的话在打包安装钩子中执行
  `gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor`。

源码安装与官方预编译包走 `apps/linux/scripts/files.py`，装到 `$XDG_DATA_HOME/icons/hicolor/`（用户目录）；系统级发行版包不要复用该路径，应装 `/usr/share/icons/hicolor/`。
