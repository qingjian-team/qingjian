# CLAUDE.md

自绘 UI 引擎 bamboo-slip。公开 API 只从 `lib.rs` 重新导出，各模块本身私有。代码组织、注释、提交信息照仓库根的 `docs/contributing.md`。

## 边界

- 引擎里不出现任何具体应用的概念（候选、拼音、主题文件格式、帧数据、配置键）。使用方自己的数据格式与建树留在使用方，
  转成引擎的类型（`Scene` / `Visual` / `Effect` / `Keyframes` / `TextStyle`）再交进来；需要使用方定制的画法走 trait（如 `Icon`）。
  不依赖任何 `qingjian-*` crate：以后要单独发布，用 `git subtree split --prefix crates/bamboo-slip` 带着历史拆出去。
- 场景树里一律是像素；点 → 像素的倍数由使用方乘。
- 每个节点都是定位容器：`position: absolute` 的节点按直接父节点摆（`Scene::node` 把 taffy 缺省的 `static` 改成 `relative`）。
  不支持 `static` 那种越过父节点找祖先的定位；叠层要出父节点的范围就挂到根节点上（`Ui::overlay`，悬停说明、对话框都这么做）。
- 改动后除了本 crate 的测试，还要跑 `cargo test -p qingjian-render`（快照测试逐像素比对候选窗截图，是引擎的回归基准）
  与 `cargo test -p qingjian-settings`（离屏点遍设置程序的控件）。

## 常用命令

```bash
cargo test -p bamboo-slip
cargo clippy -p bamboo-slip --all-targets --all-features -- -D warnings
cargo clippy -p bamboo-slip --target x86_64-pc-windows-gnu --all-targets -- -D warnings   # DirectWrite 部分只在 Windows 编
cargo run -p bamboo-slip --example window --features host                                 # 控件演示窗口
```
