#!/usr/bin/env python3
"""从青简自建的越南语释义表派生输入候选词表。"""

from argparse import ArgumentParser
from collections import Counter
from pathlib import Path


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("glossary_output", type=Path)
    args = parser.parse_args()

    words: Counter[str] = Counter()
    chinese: dict[str, Counter[str]] = {}
    with args.input.open(encoding="utf-8") as source:
        for line in source:
            fields = line.rstrip("\n").split("\t")
            source_word = fields[0].strip()
            for field in fields[1:]:
                text = field.split(". ", 1)[-1].split("|", 1)[0].strip().lower()
                if len(text) >= 2 and all(char.isalpha() or char in " -'" for char in text):
                    words[text] += 1
                    chinese.setdefault(text, Counter())[source_word] += 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="\n") as output:
        output.write("# 由 assets/glossary/glossary-vi.tsv 派生；第三列是释义表出现次数。\n")
        for word, frequency in sorted(words.items()):
            output.write(f"{word}\t{word}\t{frequency}\n")

    args.glossary_output.parent.mkdir(parents=True, exist_ok=True)
    with args.glossary_output.open("w", encoding="utf-8", newline="\n") as output:
        output.write("# 由 assets/glossary/glossary-vi.tsv 反转派生；每词最多两个中文释义。\n")
        for word, senses in sorted(chinese.items()):
            ranked = sorted(senses.items(), key=lambda item: (-item[1], item[0]))[:2]
            output.write(word + "\t" + "\t".join(text for text, _ in ranked) + "\n")


if __name__ == "__main__":
    main()
