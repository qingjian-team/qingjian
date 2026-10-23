<p align="center"><a href="../../README.md">简体中文</a> | <a href="README.zh-Hant.md">繁體中文</a> | <b>English</b> | <a href="README.ja.md">日本語</a> | <a href="README.es.md">Español</a> | <a href="README.vi.md">Tiếng Việt</a> | <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../../assets/icon/qingjian-mark.svg" alt="Qingjian bamboo-slip icon" height="108">
</p>

<h1 align="center">Qingjian</h1>

<p align="center"><strong>Type as usual, and pick up a word along the way.</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux: Fcitx4 / 5, manual start"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+: preview"></a>
</p>

Qingjian is an input method. You type the way you normally would: enter pinyin, choose a candidate, write a whole sentence. A gloss beside each candidate lets language learning happen naturally while you type. The gloss is always auxiliary information; it never takes the place of the text you are entering.

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

The video shows sentence input, candidate reordering and candidate glosses on macOS.

## Download and get started

- **macOS, Windows**: [Download Qingjian](https://qingjian.app/download); for installation steps see the [user docs](https://qingjian.app/docs/getting-started/install).
- **Linux**: Fcitx5 and Fcitx4 are installed from source and use the system's default candidate panel; the background service currently has to be started by hand — see [Linux installation](https://qingjian.app/docs/platforms/linux).
- **Android**: a preview with fewer features than the desktop versions; download the arm64 APK from [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest), see [Android](https://qingjian.app/docs/platforms/android) for how to enable it, and [apps/android](../../apps/android/README.md) to build from source.

**Official release channels**: these are the only ones for now; a "Qingjian" from anywhere else isn't an official release.

- [The download page on the website](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- The Quark Drive link given in each version's changelog

Qingjian isn't published in any package manager yet.

The macOS and Windows builds are still in beta. After installing, select Qingjian, choose the language you want to learn under "Preferences / Settings → General", and start typing. For a first run, read [First input](https://qingjian.app/docs/getting-started/first-input).

## What you see while typing

```text
1  开发        development
2  编程        programming
3  架构        architecture
```

Only one learning language is shown beside the candidates. You can choose English, Japanese, Spanish, Vietnamese or German today, or turn the gloss off. Qingjian also supports sentence input, abbreviated pinyin, spelling correction, double pinyin, Wubi and more; a local sentence model revises sentence candidates once you pause.

The Statistics page also shows today's, last-seven-day and lifetime input volume, and the vocabulary records for the language you are learning. For details see [Input](https://qingjian.app/docs/input), [Glosses and new words](https://qingjian.app/docs/learning/translation), [Local statistics](https://qingjian.app/docs/learning/statistics) and [Keys and shortcuts](https://qingjian.app/docs/getting-started/keys).

## Data and privacy

Pinyin conversion, dictionary lookup, the local model and typing-habit learning all happen on your device. Input volume and vocabulary statistics are stored only on this machine as well, and are never uploaded; Qingjian needs no account. The input log and the statistics are stored separately: the log can be turned off or cleared in Settings without affecting the statistics. Update checks ask the website for the release list and can be turned off in Settings.

The optional **cloud prediction is off by default**. Once it is on, Qingjian sends your current input and nearby text directly to the AI provider you configure yourself, to get candidates or sentence completions; requests do not go through Qingjian's servers. For what is sent and what is stored on this machine, see [Privacy](https://qingjian.app/docs/help/privacy).

## Docs, feedback and contributing

- [User docs](https://qingjian.app/docs): installation, settings, input, uninstall and FAQ.
- [Report a problem or send a suggestion](https://github.com/qingjian-team/qingjian/issues/new/choose); you can also join the [QQ beta group](https://qm.qq.com/q/jBvn2gGTxm).
- Want to contribute? Start from the [developer docs](../) and the [contributing guide](../contributing.md).

Qingjian is free through the [official channel](https://qingjian.app/download). The code is licensed under [GPL-3.0-or-later](../../LICENSE); the project name and logo are not covered by the code license. Bundled data has its own sources and licenses — see the [data provenance list](../design/landscape.md).

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="Qingjian GitHub star history" width="600">
    </picture>
  </a>
</p>
