# /// script
# requires-python = ">=3.10"
# ///
"""生僻字表：把「规范字表外扩充汉字」与「台湾教育部罕用字 C 表」合成 `assets/lexicon/01_characters/rare_chars.tsv`。

用法：uv run tools/corpus/rare_chars.py <规范字表外扩充汉字_79782.tsv> <台湾教育部罕用字_C表_18318.tsv>

按字去重，两表读音取并集（扩充表的大陆读音在前）；8105 规范字与没有读音的字不收。
扩充表读音来自 Unihan 17（Unicode License），台湾表读音来自 CNS 全字库（政府資料開放授權條款）。
词库生成（`dict-convert lexicon`）再去掉词库已有的字、丢掉不是合法音节的读音，按最低词频收进基础词库。
"""

import argparse
import csv
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/lexicon/01_characters/rare_chars.tsv"
STANDARD = ROOT / "assets/lexicon/01_characters/standard_8105.tsv"

# 两张表都按 Unicode 11 / GB 18030-2022 原版取字，扩展 G 以后的字只能手工补（kMandarin 读音）
MANUAL = {"𰻝": ["biang"], "𰻞": ["biang"]}

# 不在 8105 里但日常常用的字，词库生成时按规范字处理（语料词频、进语言模型），其余生僻字词频 0。
# 语料里生僻字的次数大多来自日文新字体与繁体残留（戦 発 砲 妳），不能拿来挑，这里逐字人工定
EVERYDAY = {"诶", "囧"}


def readings(path: Path, char_field: str) -> dict[str, list[str]]:
    with path.open(encoding="utf-8", newline="") as f:
        rows = csv.DictReader(f, delimiter="\t")
        result = {}
        for row in rows:
            ch = row[char_field].strip()
            plain = [s for s in row["拼音无声调"].split(";") if s]
            if len(ch) == 1 and plain:
                result[ch] = plain
        return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("extension", type=Path, help="规范字表外扩充汉字_79782.tsv")
    parser.add_argument("taiwan", type=Path, help="台湾教育部罕用字_C表_18318.tsv")
    args = parser.parse_args()

    standard = {line.split("\t", 1)[0] for line in STANDARD.read_text(encoding="utf-8").splitlines()}
    sources = [("ext", readings(args.extension, "字条")), ("tw", readings(args.taiwan, "字条")), ("manual", MANUAL)]
    merged: dict[str, tuple[list[str], list[str]]] = {}
    for name, table in sources:
        for ch, syllables in table.items():
            if ch in standard:
                continue
            found, origin = merged.setdefault(ch, ([], []))
            found.extend(s for s in syllables if s not in found)
            origin.append(name)
    for ch in EVERYDAY:
        merged[ch][1].append("everyday")

    with OUT.open("w", encoding="utf-8") as out:
        out.write("# 生僻字：tools/corpus/rare_chars.py 生成。字\t读音（空格分隔，无声调）\t来源（ext 扩充表 / tw 台湾 C 表 / manual 手工；everyday 是按规范字处理的常用字）\n")
        for ch in sorted(merged):
            found, origin = merged[ch]
            out.write(f"{ch}\t{' '.join(found)}\t{','.join(origin)}\n")
    print(f"{len(merged)} 字 → {OUT}", file=sys.stderr)


if __name__ == "__main__":
    main()
