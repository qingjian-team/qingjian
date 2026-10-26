# /// script
# requires-python = ">=3.10"
# ///
"""生成符号面板的数据：`catalog.tsv`（分组\\t分类\\t条目\\t名字）与 `keywords.tsv`（词\\t条目），编进 qingjian-core。

用法：
  uv run tools/corpus/symbol_panel.py \\
    --symbols <字词库>/07_常用符号库 --emoji-list <字词库>/08_常用Emoji与颜文字/00_表情总表.tsv \\
    --kana <字词库>/06_日文字词库/01_假名字库 \\
    --cldr <cldr>/common/annotations/zh.xml --cldr-derived <cldr>/common/annotationsDerived/zh.xml \\
    --emoji-test emoji-test.txt -o crates/qingjian-core/data/panel

符号按维护者整理的符号库分类（Unicode 字符）；emoji 按 Unicode emoji-test.txt 的官方分组，只收完全限定、不带肤色、
版本不超过 --max-emoji-version 的（再新的旧系统显示不出来）；名字与检索词取 CLDR 中文 annotations（Unicode License v3）。
"""

import argparse
import csv
import re
import unicodedata
import xml.etree.ElementTree as ET
from pathlib import Path

# 符号库的分类文件（去掉序号前缀）按这个顺序出；汉语拼音另由拉丁字母里的带调字母补齐
SYMBOL_ORDER = [
    ("00_常用精选", "常用"),
    ("01_中文标点", "中文标点"),
    ("02_英文标点", "英文标点"),
    ("03_数学符号", "数学"),
    ("04_箭头", "箭头"),
    ("05_希腊字母", "希腊字母"),
    ("08_序号与数字", "序号"),
    ("14_货币与单位", "货币单位"),
    ("17_分数", "分数"),
    ("07_上标与下标", "上标下标"),
    ("13_网络与装饰符号", "装饰"),
    ("15_几何与制表", "几何制表"),
    ("16_办公与杂项", "办公杂项"),
    ("10_汉语拼音", "汉语拼音"),
    ("11_注音符号", "注音"),
    ("平假名", "平假名"),
    ("片假名", "片假名"),
    ("09_国际音标", "国际音标"),
    ("06_拉丁字母", "拉丁字母"),
    ("12_竖排标点", "竖排标点"),
    ("18_全角英文与数字", "全角字母"),
    ("19_数学字母样式", "数学字母"),
]

EMOJI_GROUPS = {
    "Smileys & Emotion": "笑脸",
    "People & Body": "人物",
    "Animals & Nature": "动物自然",
    "Food & Drink": "食物",
    "Travel & Places": "旅行地点",
    "Activities": "活动",
    "Objects": "物品",
    "Symbols": "标志",
    "Flags": "旗帜",
}

# 网络：敲网址、邮箱用的整段，符号库里没有，写死在这里，排在英文标点后面
NETWORK = [".", "/", "@", "-", "_", ":", ".com", ".cn", ".net", ".org", ".com.cn", ".edu",
           "www.", "http://", "https://", "ftp://", "news.", "?", "=", "&", "#", "%", "~", "+"]

COMMON_EXCLUDED = {"06", "09", "10", "11", "12", "18", "19"}

SKIN_TONES = set(range(0x1F3FB, 0x1F400))
PINYIN_VOWELS = "aeiouü"


def is_emoji(text: str) -> bool:
    return any(ord(c) >= 0x1F000 or ord(c) in (0xFE0F, 0x200D, 0x20E3) for c in text)


def read_tsv(path: Path) -> list[dict]:
    with path.open(encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def load_cldr(*paths: Path) -> dict[str, dict]:
    """码点序列（去掉 FE0F）→ {'tts': 名字, 'keywords': [词…]}。"""
    table: dict[str, dict] = {}
    for path in paths:
        for node in ET.parse(path).getroot().iter("annotation"):
            cp = node.get("cp", "").replace("️", "")
            entry = table.setdefault(cp, {"tts": "", "keywords": []})
            words = [w.strip() for w in (node.text or "").split("|") if w.strip()]
            if node.get("type") == "tts":
                entry["tts"] = entry["tts"] or (words[0] if words else "")
            else:
                entry["keywords"].extend(w for w in words if w not in entry["keywords"])
    return table


def pinyin_letters(latin_rows: list[dict]) -> list[str]:
    """拉丁字母里的汉语拼音带调字母：a e i o u ü 的一二三四声（小写在前）。"""
    marks = {"MACRON": 0, "ACUTE": 1, "CARON": 2, "GRAVE": 3}
    found = {}
    for row in latin_rows:
        name = row["Unicode名称"]
        m = re.fullmatch(r"LATIN (SMALL|CAPITAL) LETTER ([AEIOU])(?: WITH DIAERESIS)?(?: AND)? WITH (MACRON|ACUTE|CARON|GRAVE)", name)
        if not m:
            continue
        base = m.group(2).lower()
        if "DIAERESIS" in name:
            base = "ü"
        found[(m.group(1) != "SMALL", PINYIN_VOWELS.index(base), marks[m.group(3)])] = row["符号"]
    return [found[k] for k in sorted(found)] + ["ü", "Ü"]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbols", type=Path, required=True)
    parser.add_argument("--emoji-list", type=Path, required=True)
    parser.add_argument("--kana", type=Path, required=True)
    parser.add_argument("--cldr", type=Path, required=True)
    parser.add_argument("--cldr-derived", type=Path, required=True)
    parser.add_argument("--emoji-test", type=Path, required=True)
    parser.add_argument("--max-emoji-version", type=float, default=15.1)
    parser.add_argument("-o", "--out-dir", type=Path, required=True)
    args = parser.parse_args()

    cldr = load_cldr(args.cldr, args.cldr_derived)
    emoji_rows = read_tsv(args.emoji_list)
    common_emoji = [r for r in emoji_rows if r["类型"] == "Emoji"]
    kaomoji = [r for r in emoji_rows if r["类型"] == "颜文字"]

    # 分组 → [(分类, [条目…])]
    sections: list[tuple[str, str, list[str]]] = []
    names: dict[str, str] = {}
    keywords: dict[str, list[str]] = {}

    def keyword(word: str, item: str) -> None:
        word = word.strip()
        if word and item not in keywords.setdefault(word, []):
            keywords[word].append(item)

    symbol_rows = {}
    for path in sorted(args.symbols.glob("*.tsv")):
        if path.stem[:2].isdigit():
            symbol_rows[path.stem] = read_tsv(path)
    kana = {name: read_tsv(args.kana / f"{name}.tsv") for name in ("平假名", "片假名")}
    emoji_texts = {r["表情"].replace("️", "") for r in common_emoji}

    for stem, title in SYMBOL_ORDER:
        if stem in kana:
            items = [r["字符"] for r in kana[stem]]
            for r in kana[stem]:
                names.setdefault(r["字符"], "")
        else:
            rows = symbol_rows[stem]
            # 装饰符号里混着的 emoji 归表情页
            rows = [r for r in rows if not (is_emoji(r["符号"]) or r["符号"].replace("️", "") in emoji_texts)]
            if stem == "00_常用精选":
                # 常用只留标点、数学、箭头、货币这些；字母、音标、注音各有自己的分类
                rows = [r for r in rows if r["分类"].split(";")[0][:2] not in COMMON_EXCLUDED]
            items = [r["符号"] for r in rows]
            for r in rows:
                for word in r["中文检索词"].split():
                    if len(word) >= 2:
                        keyword(word, r["符号"])
            if stem == "10_汉语拼音":
                items = pinyin_letters(symbol_rows["06_拉丁字母"]) + items
        seen = set()
        items = [i for i in items if not (i in seen or seen.add(i))]
        sections.append(("符号", title, items))
        if stem == "02_英文标点":
            sections.append(("符号", "网络", NETWORK))

    sections.append(("表情", "常用", [r["表情"] for r in common_emoji]))
    sections.append(("表情", "颜文字", [r["表情"] for r in kaomoji]))
    for r in emoji_rows:
        for word in r["中文叫法"].split("/"):
            keyword(word, r["表情"])
        if r["类型"] == "颜文字":
            names[r["表情"]] = r["中文叫法"].split("/")[0]

    group = None
    grouped: dict[str, list[str]] = {}
    for line in args.emoji_test.read_text(encoding="utf-8").splitlines():
        if line.startswith("# group:"):
            group = EMOJI_GROUPS.get(line.split(":", 1)[1].strip())
            continue
        m = re.match(r"^([0-9A-F ]+?)\s*;\s*fully-qualified\s*#\s*\S+\s+E(\d+\.\d+)", line)
        if not m or group is None:
            continue
        codes = [int(c, 16) for c in m.group(1).split()]
        if SKIN_TONES & set(codes) or float(m.group(2)) > args.max_emoji_version:
            continue
        grouped.setdefault(group, []).append("".join(map(chr, codes)))
    for title in EMOJI_GROUPS.values():
        sections.append(("表情", title, grouped.get(title, [])))

    every = {item for _, _, items in sections for item in items}
    for item in every:
        entry = cldr.get(item.replace("️", ""))
        if entry:
            names[item] = names.get(item) or entry["tts"]
            for word in entry["keywords"] + [entry["tts"]]:
                keyword(word, item)

    args.out_dir.mkdir(parents=True, exist_ok=True)
    with (args.out_dir / "catalog.tsv").open("w", encoding="utf-8") as f:
        for group_name, title, items in sections:
            for item in items:
                assert "\t" not in item and "\n" not in item, item
                f.write(f"{group_name}\t{title}\t{item}\t{names.get(item, '')}\n")
    with (args.out_dir / "keywords.tsv").open("w", encoding="utf-8") as f:
        for word in sorted(keywords):
            for item in keywords[word]:
                if item in every:
                    f.write(f"{word}\t{item}\n")
    for group_name, title, items in sections:
        print(group_name, title, len(items))
    print("keywords", len(keywords))


if __name__ == "__main__":
    main()
