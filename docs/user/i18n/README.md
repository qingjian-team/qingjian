# 用户文档的译本

`docs/user/` 下是中文（简体）原文；这里放各语言译本，官网按语言读取。某页没有译本时显示替补版本并提示「还没有翻译」：
繁体看简体原文；日、西、越、德先看英文版，英文也没有再看中文原文。

这个 README 是维护约定，不渲染。

## 结构

```text
docs/user/i18n/
├── zh-Hant/   # 繁體中文
├── en/        # English
├── ja/        # 日本語
├── es/        # Español
├── vi/        # Tiếng Việt
└── de/        # Deutsch
```

语言代码与官网一致，固定为上面六个（注意繁体是 `zh-Hant`，不是 `zh-TW` / `zh-HK`），每个目录里的 README 写着对应的地址前缀。
加一种新语言需要先在官网仓库加上这门语言，再在这里建同名目录。

- **目录结构与原文一一对应**：`docs/user/input/wubi.md` 的英文版是 `docs/user/i18n/en/input/wubi.md`，分组的 `index.md` 也要译。
  文件名不译；官网地址是 `/en/docs/input/wubi`。
- **frontmatter 三个键都保留**：`title`、`description` 译成该语言，`order` 与原文相同。
- **链接与图片照抄原文的相对路径**：译本和原文目录结构相同，`[安装](../getting-started/install.md)` 在译本里还是 `../getting-started/install.md`，
  链到同语言的那一页。图片只放在原文目录里，译本引用同样的相对路径，官网按原文目录找图。
- 原文里没有的页，译本里不要加：官网的页面集合以原文为准。

## 原文改了怎么办

- 改了原文的行为说明，能顺手改译本就一起改；改不了的，在提交说明里写「译本待更新：<页面>」，不要让译本说出和原文相反的话。
- 新增一页原文时不必同时出译本，官网会先显示替补版本。

## 译法

- 写法跟原文的约定一致（[../README.md](../README.md)「写法」一节）：说明书体，面向普通用户，不出现实现词。
- 用各语言的自然说法，不逐字直译；句子可以重组，但不增删事实。
- **设置程序与菜单的界面目前只有中文**，界面上的字照原样写中文，第一次出现时括号里附译名，让读者能在界面上对上：
  `设置 → 外观 (Settings → Appearance)`（西文用半角括号，日文用全角），之后同一页里可以只写中文。繁体译本把这些界面字转成繁体字形即可（「設置 → 外觀」），不改用台湾的说法。
- 候选、拼音、按键示例原样保留：`kaifa`、`开发`、`Shift`、`⌃⌥T` 不译。
- 平台自己的界面（系统设置、开始菜单）用该语言系统里的正式名称：macOS 的「系统设置 → 隐私与安全性」英文写 System Settings → Privacy & Security。

## 术语表

| 中文 | 繁體中文 | English | 日本語 | Español | Tiếng Việt | Deutsch |
|---|---|---|---|---|---|---|
| 候选 / 候选词 | 候選 / 候選字詞 | candidate | 候補 | candidato | từ gợi ý | Kandidat |
| 候选窗口 | 候選視窗 | candidate window | 候補ウィンドウ | ventana de candidatos | cửa sổ gợi ý | Kandidatenfenster |
| 拼音行 | 拼音列 | pinyin line | ピンイン行 | línea de pinyin | dòng pinyin | Pinyin-Zeile |
| 上屏 | 送字 | commit | 確定 | confirmar | chọn chữ | bestätigen |
| 译词 | 譯詞 | gloss | 訳語 | traducción | bản dịch | Übersetzung |
| 学习语言 | 學習語言 | learning language | 学習言語 | idioma de aprendizaje | ngôn ngữ học | Lernsprache |
| 生词 | 生詞 | new word | 新しい単語 | palabra nueva | từ mới | neues Wort |
| 生词本 | 生詞本 | vocabulary book | 単語帳 | cuaderno de vocabulario | sổ từ mới | Vokabelheft |
| 整句输入 | 整句輸入 | sentence input | 文単位の入力 | entrada por frases | nhập cả câu | Satzeingabe |
| 简拼 | 簡拼 | abbreviated pinyin | 略式ピンイン | pinyin abreviado | pinyin viết tắt | abgekürztes Pinyin |
| 模糊音 | 模糊音 | fuzzy pinyin | あいまい音 | pinyin difuso | pinyin gần đúng | unscharfes Pinyin |
| 双拼 | 雙拼 | double pinyin (shuangpin) | 双拼 | pinyin doble (shuangpin) | song bính (shuangpin) | Shuangpin |
| 五笔 | 五筆 | Wubi | 五筆 | Wubi | Ngũ Bút (Wubi) | Wubi |
| 辅码 | 輔碼 | auxiliary code | 補助コード | código auxiliar | mã phụ | Hilfscode |
| 中英切换 | 中英切換 | Chinese/English switch | 中英切り替え | cambio chino/inglés | chuyển Trung/Anh | Chinesisch/Englisch-Umschaltung |
| 英文模式 | 英文模式 | English mode | 英語モード | modo inglés | chế độ tiếng Anh | Englischmodus |
| 状态条 | 狀態列 | status bar | ステータスバー | barra de estado | thanh trạng thái | Statusleiste |
| 词库 | 詞庫 | dictionary | 辞書 | diccionario | từ điển | Wörterbuch |
| 自定义短语 | 自訂短語 | custom phrases | カスタムフレーズ | frases personalizadas | cụm từ tùy chỉnh | eigene Phrasen |
| 主题 | 主題 | theme | テーマ | tema | giao diện | Design |
| 云联想 | 雲端聯想 | cloud suggestions | クラウド予測 | sugerencias en la nube | gợi ý qua đám mây | Cloud-Vorschläge |
| AI 服务 | AI 服務 | AI services | AI サービス | servicios de IA | dịch vụ AI | KI-Dienste |
| 服务商 | 服務商 | provider | サービス事業者 | proveedor | nhà cung cấp | Anbieter |
| 繁体输出 | 繁體輸出 | Traditional Chinese output | 繁体字出力 | salida en chino tradicional | xuất chữ phồn thể | Ausgabe in Langzeichen |
| 组合键 | 組合鍵 | key combination | キーの組み合わせ | combinación de teclas | tổ hợp phím | Tastenkombination |

## 怎么贡献一种语言

1. 从英文版（`en/`）或中文原文起步都可以；以原文为准，英文版只是参考。可以只译几页，没译的页官网照常显示替补版本。
2. 标题的层级与顺序和原文保持一致（`##` 对 `##`、同样多个）：页内锚点按「第几个标题」对应。
3. 链接里的锚点先照抄原文的中文写法，译完运行一次：

   ```sh
   node tools/docs/i18n-anchors.mjs docs/user <语言>
   ```

   它把 `#选错了词` 这类中文锚点换成译文标题对应的锚点，并列出标题数对不上、找不到目标的链接。
4. 术语按上面的术语表；表里没有的词，同一门语言里前后保持一致即可。
5. 提 PR 时写明译了哪些页、依据的是原文哪个提交。官网构建时会校验每个锚点，对不上会构建失败。
