"""把开源词表（jieba dict + mozillazg 拼音数据）转成青简词库 TSV。

数据源（都是 MIT，见各仓库 LICENSE）：
- jieba `dict.txt`：`词 频次 词性`，34.9 万条，只有词频没有拼音
- phrase-pinyin-data `pinyin.txt`：`词: 拼音`，4.7 万条，带声调
- pinyin-data `pinyin.txt`：`U+XXXX: yī, yí` 单字读音，给没有词级拼音的兜底

产物：`词\t音节（空格分隔、去声调、ü 写 v）\t词频`，可直接 `dict-convert pack dict` 打成 .qj。

`--jsonl` 另写一份 `gloss-gen` 认的 JSONL（一个 topic 装全部词），这样能借
`gloss-gen export-vocab --existing <装机 .qj>` 对着 `.qj` 词库去重（Python 不解析 `.qj` 容器）。

用法：
    python tools/corpus/opensource_vocab.py --dir data/generated/opensource \
        --out data/generated/vocab-opensource.tsv \
        [--jsonl data/generated/vocab-opensource.jsonl] [--existing 已有.tsv …]
    cargo run -p qingjian-gloss-gen -- export-vocab --input data/generated/vocab-opensource.jsonl \
        --existing "C:/Program Files/Qingjian/data/generated/dict.qj" --name vocab-opensource
"""

import argparse
import re
from pathlib import Path

HAN = re.compile(r"^[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]+$")
TONE_MARKS = {
    "ā": "a", "á": "a", "ǎ": "a", "à": "a",
    "ē": "e", "é": "e", "ě": "e", "è": "e",
    "ī": "i", "í": "i", "ǐ": "i", "ì": "i",
    "ō": "o", "ó": "o", "ǒ": "o", "ò": "o",
    "ū": "u", "ú": "u", "ǔ": "u", "ù": "u",
    "ǖ": "v", "ǘ": "v", "ǚ": "v", "ǜ": "v", "ü": "v",
    "ń": "n", "ň": "n", "ǹ": "n", "ḿ": "m",
}


def strip_tone(syllable: str) -> str:
    """去掉声调符号、ü 写 v、丢掉声调数字，返回纯 a-z。"""
    out = []
    for ch in syllable:
        if ch in TONE_MARKS:
            out.append(TONE_MARKS[ch])
        elif ch.isdigit():
            continue
        else:
            out.append(ch.lower())
    return "".join(out)


def load_phrase_pinyin(path: Path) -> dict[str, list[str]]:
    """`词: yī dīng bù shí` → {词: [yi, ding, bu, shi]}。"""
    table: dict[str, list[str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        word, _, reading = line.partition(":")
        word = word.strip()
        syllables = [strip_tone(s) for s in reading.split()]
        if word and syllables and len(syllables) == len(word):
            table[word] = syllables
    return table


def load_char_pinyin(path: Path) -> dict[str, str]:
    """`U+4E00: yī, yí` → {一: yi}（取第一个读音）。"""
    table: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        point, _, rest = line.partition(":")
        rest = rest.split("#", 1)[0].strip()
        try:
            char = chr(int(point.strip()[2:], 16))
        except ValueError:
            continue
        first = rest.split(",")[0].strip()
        if first:
            table[char] = strip_tone(first)
    return table


def load_existing(paths: list[Path]) -> set[str]:
    """已有词库里的词：`.qj` 读字符串池不可行，这里只处理 TSV；`.qj` 由 CLI 另行过滤。"""
    words: set[str] = set()
    for path in paths:
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#"):
                continue
            word = line.split("\t", 1)[0]
            if word:
                words.add(word)
    return words


def weight_of(count: int) -> int:
    """词频就用 jieba 给的次数，原样搬过来。

    不换算：换算是另一套标定（要拿青简自己的语料拟合），这里只要求「相对高低照原样」；
    数据源没给的量就不编。
    """
    return max(1, count)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", default="data/generated/opensource", type=Path)
    parser.add_argument("--out", default="data/generated/vocab-opensource.tsv", type=Path)
    parser.add_argument("--existing", nargs="*", default=[], type=Path)
    parser.add_argument("--jsonl", type=Path, help="另写一份 gloss-gen 认的 JSONL（可对 .qj 去重）")
    parser.add_argument("--min-count", default=3, type=int, help="jieba 词频下限（放宽着收）")
    parser.add_argument("--max-chars", default=8, type=int, help="词最长几个汉字")
    args = parser.parse_args()

    phrases = load_phrase_pinyin(args.dir / "phrase-pinyin.txt")
    chars = load_char_pinyin(args.dir / "pinyin-data.txt")
    existing = load_existing(args.existing)

    # 先读一遍 jieba，取最大词频当刻度
    entries: list[tuple[str, int]] = []
    for line in (args.dir / "jieba-dict.txt").read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        word, count = parts[0], parts[1]
        if not word.isdigit():
            try:
                count = int(count)
            except ValueError:
                continue
        if not HAN.match(word) or len(word) > args.max_chars or count < args.min_count:
            continue
        entries.append((word, count))

    rows: list[tuple[str, str, int]] = []
    no_reading = 0
    for word, count in entries:
        if word in existing:
            continue
        syllables = phrases.get(word)
        if syllables is None:
            syllables = [chars[c] for c in word if c in chars]
            if len(syllables) != len(word):
                no_reading += 1
                continue
        rows.append((word, " ".join(syllables), weight_of(count)))

    rows.sort(key=lambda row: (-row[2], row[0]))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# 由 tools/corpus/opensource_vocab.py 生成：jieba dict（MIT）+ mozillazg 拼音数据（MIT）。词\t拼音\t词频（jieba 原值）\n")
        for word, key, weight in rows:
            out.write(f"{word}\t{key}\t{weight}\n")
    if args.jsonl:
        import json

        args.jsonl.parent.mkdir(parents=True, exist_ok=True)
        with args.jsonl.open("w", encoding="utf-8", newline="\n") as out:
            words = [
                {"word": word, "pinyin": key.split(), "count": weight}
                for word, key, weight in rows
            ]
            out.write(json.dumps({"topic": "开源词表 jieba", "words": words}, ensure_ascii=False))
            out.write("\n")
        print(f"JSONL {len(rows)} 条 → {args.jsonl}")
    print(
        f"词表 {len(rows)} 条（跳过已有 {len(existing)} 条、没读音 {no_reading} 条）→ {args.out}"
    )


if __name__ == "__main__":
    main()
