# /// script
# requires-python = ">=3.10"
# dependencies = ["wordfreq>=3.1"]
# ///
"""给英文词表配词频：读 english.tsv（词\\t编码[\\t词频]），用 wordfreq 的英文 Zipf 频率（×1000 取整）写出 `编码\\t词频`。

用法：uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv
然后重新生成词表时带上：cargo run --release -p qingjian-dict-convert -- english ... --frequency data/generated/english-frequency.tsv

wordfreq 的代码是 MIT，数据带 CC-BY-SA 等许可，这里只取频率数字做排序。
"""

import argparse
import sys

from wordfreq import zipf_frequency


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("word_list")
    parser.add_argument("-o", "--output", required=True)
    args = parser.parse_args()
    rows = 0
    with open(args.word_list, encoding="utf-8") as src, open(args.output, "w", encoding="utf-8") as out:
        out.write("# 编码\t词频（wordfreq 英文 Zipf × 1000）\n")
        for line in src:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            word = fields[0]
            code = fields[1] if len(fields) > 1 else word.lower()
            zipf = zipf_frequency(word, "en")
            out.write(f"{code}\t{int(zipf * 1000)}\n")
            rows += 1
    print(f"wrote {rows} rows to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
