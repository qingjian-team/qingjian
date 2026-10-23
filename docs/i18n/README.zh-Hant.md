<p align="center"><a href="../../README.md">简体中文</a> | <b>繁體中文</b> | <a href="README.en.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.es.md">Español</a> | <a href="README.vi.md">Tiếng Việt</a> | <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../../assets/icon/qingjian-mark.svg" alt="青簡竹簡圖示" height="108">
</p>

<h1 align="center">青簡 Qingjian</h1>

<p align="center"><strong>好好打字，順便多認識一個詞。</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux Fcitx4 與 Fcitx5，手動啟動"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+，預覽版"></a>
</p>

青簡是一款輸入法。你可以像平常一樣打字：輸入拼音或注音、選擇候選字詞、寫完整句；候選字旁的一條譯詞，讓語言學習自然發生在日常打字之中。譯詞始終只是輔助資訊，不會蓋過你要輸入的文字。

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

影片示範了青簡在 macOS 上的整句輸入、候選重新排序與候選譯詞。

## 下載與開始使用

- **macOS、Windows**：[下載青簡](https://qingjian.app/download)；安裝步驟請見[使用說明](https://qingjian.app/docs/getting-started/install)。
- **Linux**：提供 Fcitx5 與 Fcitx4 原始碼安裝，使用系統預設的候選面板；目前需要手動啟動背景服務，詳見 [Linux 安裝說明](https://qingjian.app/docs/platforms/linux)。
- **Android**：預覽版，功能比桌面版少；arm64 的 APK 在 [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest) 下載，啟用步驟見 [Android 說明](https://qingjian.app/docs/platforms/android)，從原始碼建置見 [apps/android](../../apps/android/README.md)。

**官方發布管道**：目前只有以下幾個，其他地方的「青簡」都不是官方發布的。

- [官網下載頁](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- 每版更新日誌裡給出的夸克網盤連結

青簡目前沒有上架任何套件管理器。

macOS 與 Windows 版本仍在測試階段。安裝後先選取青簡，在「偏好設置 / 設置 → 通用」選擇想學習的語言，就可以開始打字。第一次使用可以從[第一次輸入](https://qingjian.app/docs/getting-started/first-input)讀起。

## 打字時，你會看到什麼

```text
1  開發        development
2  編程        programming
3  架構        architecture
```

候選字旁一次只顯示一種學習語言的譯詞。目前可以選擇英語、日語、西班牙語、越南語或德語，也可以關閉譯詞顯示。青簡也支援整句輸入、簡拼、拼字糾錯、雙拼、大千注音、五筆等輸入方式，並可[輸出繁體字](https://qingjian.app/docs/input/traditional)（台灣、香港或通用字形）；本機的整句模型會在你停頓後調整句子候選。

「統計」頁還會顯示今天、最近 7 天與累計的輸入量，以及學習語言的詞彙紀錄。詳細用法請見[輸入功能](https://qingjian.app/docs/input)、[譯詞與生詞](https://qingjian.app/docs/learning/translation)、[本機統計](https://qingjian.app/docs/learning/statistics)與[按鍵與快速鍵](https://qingjian.app/docs/getting-started/keys)。

## 資料與隱私

拼音轉換、詞庫查詢、本機模型與輸入習慣學習都在你的裝置上完成。輸入量與詞彙統計也只儲存在本機，不會上傳；青簡不需要帳號。輸入紀錄與統計分開儲存，紀錄可以在設定中關閉或清除，不影響統計。檢查更新時會向官網取得版本清單，可在設定中關閉。

選用的**雲端聯想預設為關閉**。開啟後，青簡會把目前的輸入與附近的文字直接傳送給你自行設定的 AI 服務供應商，以取得候選字詞或整句補全；請求不經過青簡的伺服器。傳送範圍與本機儲存的資料，請見[隱私](https://qingjian.app/docs/help/privacy)。

## 文件、回饋與參與開發

- [使用說明](https://qingjian.app/docs)：安裝、設定、輸入、解除安裝與常見問題。
- [回報問題或提出建議](https://github.com/qingjian-team/qingjian/issues/new/choose)；也可以加入 [QQ 內測交流群](https://qm.qq.com/q/jBvn2gGTxm)。
- 想參與開發？從[開發文件](../)與[開發約定](../contributing.md)開始。

青簡透過[官方管道](https://qingjian.app/download)免費提供。程式碼採用 [GPL-3.0-or-later](../../LICENSE) 授權；專案名稱與標誌不在程式碼授權範圍內。隨附資料各有其來源與授權，請見[資料來源清單](../design/landscape.md)。

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="青簡的 GitHub Star 成長曲線" width="600">
    </picture>
  </a>
</p>
