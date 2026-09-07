# 图标

`logo.png`（866×866，带透明通道）是唯一的源文件。`apps/macos/scripts/bundle.sh` 打包时用 `sips` + `iconutil`
生成 `Qingjian.icns`（应用图标）和 `qingjian-menu.tiff`（输入法菜单 / 菜单栏用的 16pt 双分辨率图标），
生成物不进仓库。
