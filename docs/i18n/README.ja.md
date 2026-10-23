<p align="center"><a href="../../README.md">简体中文</a> | <a href="README.zh-Hant.md">繁體中文</a> | <a href="README.en.md">English</a> | <b>日本語</b> | <a href="README.es.md">Español</a> | <a href="README.vi.md">Tiếng Việt</a> | <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../../assets/icon/qingjian-mark.svg" alt="青簡の竹簡アイコン" height="108">
</p>

<h1 align="center">青簡 Qingjian</h1>

<p align="center"><strong>いつも通りに入力して、ついでに単語をひとつ覚える。</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux：Fcitx4 / 5、手動起動"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+：プレビュー版"></a>
</p>

青簡（Qingjian）は中国語の入力メソッドです。ピンインを入力し、候補を選び、文をまるごと書く――いつも通りに入力するだけです。候補の横に表示される訳語によって、日々の入力の中で自然に語学学習ができます。訳語はあくまで補助情報で、入力したい文字を邪魔することはありません。

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

動画では、macOS での文単位の入力、候補の並べ替え、候補の訳語を紹介しています。

## ダウンロードと使い始め

- **macOS、Windows**：[青簡をダウンロード](https://qingjian.app/download)。インストール手順は[ユーザードキュメント](https://qingjian.app/docs/getting-started/install)を参照してください。
- **Linux**：Fcitx5 と Fcitx4 はソースからインストールし、システム標準の候補パネルを使います。現時点ではバックグラウンドサービスを手動で起動する必要があります。詳しくは [Linux のインストール](https://qingjian.app/docs/platforms/linux)を参照してください。
- **Android**：プレビュー版で、デスクトップ版より機能が少なめです。arm64 の APK は [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest) からダウンロードできます。有効にする手順は [Android](https://qingjian.app/docs/platforms/android)、ソースからのビルドは [apps/android](../../apps/android/README.md) を参照してください。

**公式の配布先**：現時点では次のものだけです。ほかの場所にある「青簡」は公式の配布ではありません。

- [公式サイトのダウンロードページ](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- 各バージョンの更新履歴に載せている Quark ドライブのリンク

青簡はまだどのパッケージマネージャーにも公開していません。

macOS 版と Windows 版はまだベータ版です。インストール後に青簡を選択し、「環境設定 / 設定 → 一般」で学びたい言語を選べば、すぐに入力を始められます。初めての方は[はじめての入力](https://qingjian.app/docs/getting-started/first-input)からどうぞ。

## 入力中に表示されるもの

```text
1  开发        開発する
2  编程        プログラミングする
3  架构        アーキテクチャ
```

候補の横に表示されるのは、一度にひとつの学習言語の訳語だけです。現在は英語、日本語、スペイン語、ベトナム語、ドイツ語から選べ、訳語を非表示にすることもできます。ほかにも文単位の入力、簡略ピンイン、スペル補正、双拼、五筆などの入力方式に対応しています。ローカルの文モデルが、入力の区切りで文の候補を調整します。

「統計」ページでは、今日・直近 7 日間・累計の入力量と、学習言語の語彙記録も確認できます。詳しい使い方は[入力機能](https://qingjian.app/docs/input)、[訳語と新出語](https://qingjian.app/docs/learning/translation)、[ローカル統計](https://qingjian.app/docs/learning/statistics)、[キーとショートカット](https://qingjian.app/docs/getting-started/keys)を参照してください。

## データとプライバシー

ピンイン変換、辞書検索、ローカルモデル、入力習慣の学習はすべてお使いの端末上で行われます。入力量と語彙の統計もこの端末にのみ保存され、アップロードされることはありません。アカウントも不要です。入力ログと統計は別々に保存され、ログは設定でオフにしたり消去したりしても統計には影響しません。アップデートの確認では公式サイトにバージョン一覧を問い合わせます。これも設定でオフにできます。

オプションの**クラウド予測は既定でオフ**です。オンにすると、青簡は現在の入力と周辺の文字を、ご自身で設定した AI サービス事業者に直接送信し、候補や文の補完を取得します。リクエストは青簡のサーバーを経由しません。送信される範囲と端末に保存されるデータについては、[プライバシー](https://qingjian.app/docs/help/privacy)を参照してください。

## ドキュメント、フィードバック、開発への参加

- [ユーザードキュメント](https://qingjian.app/docs)：インストール、設定、入力、アンインストール、よくある質問。
- [問題の報告や提案](https://github.com/qingjian-team/qingjian/issues/new/choose)。[QQ ベータテストグループ](https://qm.qq.com/q/jBvn2gGTxm)にも参加できます。
- 開発に参加したい方は、[開発ドキュメント](../)と[開発ルール](../contributing.md)から始めてください。

青簡は[公式チャネル](https://qingjian.app/download)で無料で提供しています。コードは [GPL-3.0-or-later](../../LICENSE) ライセンスです。プロジェクト名とロゴはコードのライセンスに含まれません。同梱データにはそれぞれの出典とライセンスがあります。[データ出典一覧](../design/landscape.md)を参照してください。

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="青簡の GitHub スター推移" width="600">
    </picture>
  </a>
</p>
