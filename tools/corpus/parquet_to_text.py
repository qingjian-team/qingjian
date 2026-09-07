# /// script
# requires-python = ">=3.11"
# dependencies = ["pyarrow>=17", "opencc-python-reimplemented>=0.1.7"]
# ///
"""把 Hugging Face 上的中文语料 parquet 转成纯文本（每行一段，繁体转简体），供 `dict-convert bigram` 统计。

用法：
  uv run tools/corpus/parquet_to_text.py data/corpus/zhwiki-*.parquet -o data/corpus/zhwiki.txt
  uv run tools/corpus/parquet_to_text.py data/corpus/lccc-*.parquet --column dialog -o data/corpus/lccc.txt

已验证的数据源（都只用于开发测试，见 docs/design/landscape.md）：
  wikimedia/wikipedia 20231101.zh   列 text，CC BY-SA 4.0
  thu-coai/lccc（refs/convert/parquet，base/train）列 dialog（每行一个列表，一轮一句），MIT
"""

import argparse
import re
import sys

import pyarrow.parquet as pq
from opencc import OpenCC

# 多余空白
SPACES = re.compile(r"[ \t　]+")


def flatten(value):
    """列可能是字符串，也可能是字符串列表（对话的每一轮）。"""
    if value is None:
        return
    if isinstance(value, str):
        yield from value.split("\n")
        return
    for item in value:
        yield from flatten(item)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("inputs", nargs="+", help="parquet 文件")
    parser.add_argument("-o", "--output", required=True, help="输出的纯文本文件")
    parser.add_argument("--column", default="text", help="取哪一列（默认 text）")
    parser.add_argument("--min-chars", type=int, default=2, help="短于此的段落丢掉")
    args = parser.parse_args()

    converter = OpenCC("t2s")
    paragraphs = 0
    with open(args.output, "w", encoding="utf-8") as out:
        for path in args.inputs:
            table = pq.ParquetFile(path)
            for batch in table.iter_batches(columns=[args.column], batch_size=2048):
                for value in batch.column(args.column).to_pylist():
                    for line in flatten(value):
                        line = SPACES.sub(" ", line).strip()
                        if len(line) < args.min_chars:
                            continue
                        out.write(converter.convert(line))
                        out.write("\n")
                        paragraphs += 1
            print(f"{path}: 累计 {paragraphs} 段", file=sys.stderr)


if __name__ == "__main__":
    main()
