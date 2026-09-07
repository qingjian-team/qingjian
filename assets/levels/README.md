# 词汇等级表

`levels-<语言>.tsv`：`词\t等级`，`# levels` 行给出等级从易到难的顺序。偏好设置「统计」页按级数用户见过 / 看熟 / 上屏过的译词
（`qingjian-translate::LevelTable` 读，`qingjian-learning::VocabularyBook` 汇总），不进候选窗口。

| 文件 | 来源 | 许可 |
| --- | --- | --- |
| `levels-en.tsv`（A1–C2，8,845 词） | The CEFR-J Wordlist Version 1.5，Yukio Tono（Tokyo University of Foreign Studies），<http://www.cefr-j.org/download.html>（A1–B2）；Octanove Vocabulary Profile C1/C2 1.0，<https://github.com/openlanguageprofiles/olp-en-cefrj> | CEFR-J：研究与商业用途免费，须按上面的写法署名；Octanove：CC BY-SA 4.0 |
| `levels-ja.tsv`（N5–N1，7,757 词） | JLPT 词表，Jonathan Waller，<http://www.tanos.co.uk/jlpt/>；经 <https://github.com/elzup/jlpt-word-list> 整理为 CSV | Tanos：CC BY（须署名并链接）；elzup 整理：MIT |

重新生成（原始 CSV 放 `data/levels/`，gitignore）：

```bash
uv run tools/corpus/levels.py en data/levels/cefrj-vocabulary-profile-1.5.csv \
    data/levels/octanove-vocabulary-profile-c1c2-1.0.csv -o assets/levels/levels-en.tsv
uv run tools/corpus/levels.py ja data/levels/jlpt-n5.csv data/levels/jlpt-n4.csv \
    data/levels/jlpt-n3.csv data/levels/jlpt-n2.csv data/levels/jlpt-n1.csv -o assets/levels/levels-ja.tsv
```

转换规则：英文斜杠分隔的拼写变体各算一条、统一小写，同一个词多个词性取最低等级；日文 `; ` 分隔的写法各算一条，去掉 `～` 与括号说明，
出现在多级取最易的一级。查询时日文查不到再试去掉词尾的 する / な / だ（释义表里是 `開発する`）。
署名同时列在偏好设置「关于」页与仓库 README。
