<p align="center"><b>简体中文</b> | <a href="docs/i18n/README.zh-Hant.md">繁體中文</a> | <a href="docs/i18n/README.en.md">English</a> | <a href="docs/i18n/README.ja.md">日本語</a> | <a href="docs/i18n/README.es.md">Español</a> | <a href="docs/i18n/README.vi.md">Tiếng Việt</a> | <a href="docs/i18n/README.de.md">Deutsch</a></p>

<p align="center">
  <img src="assets/icon/qingjian-mark.svg" alt="青简竹简图标" height="108">
</p>

<h1 align="center">青简 Qingjian</h1>

<p align="center"><strong>好好输入，顺便多认识一个词。</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux Fcitx4 和 Fcitx5，手动启动"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+，预览版"></a>
</p>

青简是一款输入法。你可以像平常一样打字：输入拼音、选择候选、写完整句；候选旁的一条译词，让语言学习自然发生在日常输入里。译词始终只是辅助信息，不会盖过你要输入的文字。

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

视频演示了青简在 macOS 上的整句输入、候选重排和候选译词。

## 下载与开始使用

- **macOS、Windows**：[下载青简](https://qingjian.app/download)；安装步骤见[使用文档](https://qingjian.app/docs/getting-started/install)。
- **Linux**：提供 Fcitx5 和 Fcitx4 源码安装，使用系统默认候选面板；目前需要手动启动后台服务，详见 [Linux 安装说明](https://qingjian.app/docs/platforms/linux)。
- **Android**：预览版，功能比桌面版少；arm64 的 APK 在 [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest) 下载，启用步骤见 [Android 说明](https://qingjian.app/docs/platforms/android)，从源码构建见 [apps/android](apps/android/README.md)。

**官方发布渠道**：目前只有以下几个，其他地方的「青简」都不是官方发布的。

- [官网下载页](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- 每版更新日志里给出的夸克网盘链接

青简目前没有上架任何包管理器。

macOS 与 Windows 版本仍处于测试阶段。安装后先选中青简，在「偏好设置 / 设置 → 通用」选择想学习的语言，就可以开始输入。第一次使用可从[第一次输入](https://qingjian.app/docs/getting-started/first-input)读起。

## 输入时，你会看到什么

```text
1  开发        development
2  编程        programming
3  架构        architecture
```

候选旁一次只显示一种学习语言的译词。目前可以选择英语、日语、西班牙语、越南语或德语，也可以关闭译词显示。青简还支持整句输入、简拼、拼写纠错、双拼、五笔等输入方式；本地整句模型会在停顿后调整句子候选。

「统计」页还会显示今天、最近 7 天和累计的输入量，以及学习语言的词汇记录。详细用法见[输入功能](https://qingjian.app/docs/input)、[译词与生词](https://qingjian.app/docs/learning/translation)、[本地统计](https://qingjian.app/docs/learning/statistics)和[按键与快捷键](https://qingjian.app/docs/getting-started/keys)。

## 数据与隐私

拼音转换、词库查询、本地模型和输入习惯学习在你的设备上完成。输入量与词汇统计也只保存在本机，不会上传；青简不需要账号。输入日志与统计分开保存，日志可在设置中关闭或清空，不影响统计。检查更新会向官网请求版本列表，可在设置中关闭。

可选的**云联想默认关闭**。开启后，青简会把当前输入和附近的文字直接发送给你自行填写的 AI 服务商，以获取候选或整句补全；请求不经过青简的服务器。发送范围与本机保存的数据，见[数据与日志](https://qingjian.app/docs/help/privacy)。

## 文档、反馈与参与开发

- [使用文档](https://qingjian.app/docs)：安装、设置、输入、卸载与常见问题。
- [反馈问题或建议](https://github.com/qingjian-team/qingjian/issues/new/choose)；也可以加入 [QQ 内测交流群](https://qm.qq.com/q/jBvn2gGTxm)。
- 想参与开发？从[开发文档](docs/)和[开发约定](docs/contributing.md)开始。

青简在[官方渠道](https://qingjian.app/download)免费提供。代码采用 [GPL-3.0-or-later](LICENSE) 许可；项目名称与 logo 不包含在代码授权中。随包数据有各自的来源与许可，见[数据来源清单](docs/design/landscape.md)。

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="青简的 GitHub Star 增长曲线" width="600">
    </picture>
  </a>
</p>
