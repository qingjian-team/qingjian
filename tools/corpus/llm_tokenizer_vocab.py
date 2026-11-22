"""从四家开源模型的 tokenizer 词表里取出中文词，合并成青简词库 TSV。

四家（都开源、都不需要 API key）：
- DeepSeek-V3  `tokenizer.json`（HF `deepseek-ai/DeepSeek-V3`）
- GLM-4.5      `tokenizer.json`（HF `zai-org/GLM-4.5`）
- Kimi-K2      `tiktoken.model`（HF `moonshotai/Kimi-K2-Instruct`，每行一个 token 的 base64）
- Qwen2.5      `tokenizer.json`（HF `Qwen/Qwen2.5-7B-Instruct`）

这些词表是 BPE：中文 token 在 `tokenizer.json` 里是 **byte-level 编码**（`你好` 显示成 `ä½ å¥½`），
要按 GPT-2 那份 bytes↔unicode 映射反解回字节再按 UTF-8 解码，才拿得到汉字。

BPE 的中文 token 有一大类是**跨词边界碎片**（`是我`、`这一`、`的在`、`者的`），直接收进词库
会让 `deyi` 打出「的一」。只丢**首字**是虚词的 token —— 尾字是虚词的不动（`目的`、`记得`、`高兴地`
都是真词），放宽着收（`--keep-fragments` 连首字碎片也留）。

产物：`词\t拼音\t词频`。**词频只记「几家词表里有它」（1–4）** —— tokenizer 本身没有词频，
不替它编别的数。拼音优先用 mozillazg 的 phrase-pinyin（词级），查不到就逐字用 pinyin-data。

用法：
    python tools/corpus/llm_tokenizer_vocab.py --dir data/generated/opensource/tokenizers \\
        --out data/generated/vocab-llm4.tsv [--jsonl data/generated/vocab-llm4.jsonl]
    # 再对着装机词库去重、打包：
    cargo run -p qingjian-gloss-gen -- export-vocab --input data/generated/vocab-llm4.jsonl \\
        --existing "C:/Program Files/Qingjian/data/generated/dict.qj" --name vocab-llm4
    cargo run -p qingjian-dict-convert -- --out-dir <临时目录> pack dict --input data/generated/vocab-llm4.tsv --name "四家开源模型词表"
"""

import argparse
import base64
import json
import re
from pathlib import Path

HAN = re.compile(r"^[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]+$")

# 四家的来源文件（顺序决定 `--sources` 省略时的缺省）
SOURCES = {
    "deepseek": "deepseek.json",
    "glm": "glm.json",
    "kimi": "kimi-tiktoken.model",
    "qwen": "qwen.json",
}

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
    out = []
    for ch in syllable:
        if ch in TONE_MARKS:
            out.append(TONE_MARKS[ch])
        elif ch.isdigit():
            continue
        else:
            out.append(ch.lower())
    return "".join(out)


def byte_decoder() -> dict[str, int]:
    """GPT-2 的 bytes ↔ unicode 映射反表：把 tokenizer.json 里的字符还原成字节。"""
    bs = (
        list(range(ord("!"), ord("~") + 1))
        + list(range(ord("¡"), ord("¬") + 1))
        + list(range(ord("®"), ord("ÿ") + 1))
    )
    cs = bs[:]
    n = 0
    for b in range(256):
        if b not in bs:
            bs.append(b)
            cs.append(256 + n)
            n += 1
    return {chr(c): b for b, c in zip(bs, cs)}


def decode_byte_level(token: str, decoder: dict[str, int]) -> bytes | None:
    out = bytearray()
    for ch in token:
        byte = decoder.get(ch)
        if byte is None:
            return None
        out.append(byte)
    return bytes(out)


def tokens_of_json(path: Path, decoder: dict[str, int]) -> set[str]:
    """`tokenizer.json`：`model.vocab` 的键是 byte-level 编码的 token。"""
    data = json.loads(path.read_text(encoding="utf-8"))
    vocab = data.get("model", {}).get("vocab") or {}
    words: set[str] = set()
    for token in vocab:
        raw = decode_byte_level(token, decoder)
        if raw is None:
            continue
        text = raw.decode("utf-8", errors="ignore")
        if HAN.match(text):
            words.add(text)
    return words


def tokens_of_tiktoken(path: Path) -> set[str]:
    """`tiktoken.model`：每行一个 token 的 base64。"""
    words: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            raw = base64.b64decode(line)
        except ValueError:
            continue
        text = raw.decode("utf-8", errors="ignore")
        if HAN.match(text):
            words.add(text)
    return words


# 首字是这些的多半是跨词边界碎片（`是我`、`这一`、`的在`、`者的`）；尾字不参与判断，
# `目的`、`记得`、`高兴地` 这些真词都得以虚词结尾。
HEAD_FRAGMENT = set("的了着过地得吗呢吧啊呀是我你他她它们个把被就都也又还很没在里下这那和与及或但而")

def looks_like_fragment(word: str) -> bool:
    """首字是虚词 → 当碎片丢掉。"""
    return word[0] in HEAD_FRAGMENT

def load_phrase_pinyin(path: Path) -> dict[str, list[str]]:
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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", default="data/generated/opensource/tokenizers", type=Path)
    parser.add_argument("--pinyin-dir", default="data/generated/opensource", type=Path)
    parser.add_argument("--out", default="data/generated/vocab-llm4.tsv", type=Path)
    parser.add_argument("--jsonl", type=Path, help="另写一份 gloss-gen 认的 JSONL（可对 .qj 去重）")
    parser.add_argument("--max-chars", default=8, type=int)
    parser.add_argument(
        "--keep-fragments",
        action="store_true",
        help="连首字是虚词的碎片一起收",
    )
    parser.add_argument(
        "--sources",
        nargs="*",
        default=list(SOURCES),
        help="要并哪几家：deepseek / glm / kimi / qwen",
    )
    args = parser.parse_args()

    decoder = byte_decoder()
    per_family: dict[str, set[str]] = {}
    for name in args.sources:
        filename = SOURCES[name]
        path = args.dir / filename
        if filename.endswith(".model"):
            words = tokens_of_tiktoken(path)
        else:
            words = tokens_of_json(path, decoder)
        per_family[name] = {w for w in words if 1 <= len(w) <= args.max_chars}
        print(f"{name:9} 中文 token {len(per_family[name]):6}")

    # 合并：记下每个词进了几家
    families: dict[str, int] = {}
    for words in per_family.values():
        for word in words:
            families[word] = families.get(word, 0) + 1

    phrases = load_phrase_pinyin(args.pinyin_dir / "phrase-pinyin.txt")
    chars = load_char_pinyin(args.pinyin_dir / "pinyin-data.txt")

    rows: list[tuple[str, str, int]] = []
    no_reading = 0
    fragments = 0
    for word, count in families.items():
        if not args.keep_fragments and looks_like_fragment(word):
            fragments += 1
            continue
        syllables = phrases.get(word)
        if syllables is None:
            syllables = [chars[c] for c in word if c in chars]
            if len(syllables) != len(word):
                no_reading += 1
                continue
        rows.append((word, " ".join(syllables), count))

    # 几家都有的排前面，同权重按词长、再按字面
    rows.sort(key=lambda row: (-row[2], -len(row[0]), row[0]))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="\n") as out:
        out.write(
            "# 由 tools/corpus/llm_tokenizer_vocab.py 生成："
            "DeepSeek-V3 / GLM-4.5 / Kimi-K2 / Qwen2.5 的 tokenizer 词表里的中文 token（各家 MIT / Apache-2.0），"
            "拼音来自 mozillazg 数据（MIT）。词\t拼音\t词频（几家词表里有它）\n"
        )
        for word, key, count in rows:
            out.write(f"{word}\t{key}\t{count}\n")

    if args.jsonl:
        args.jsonl.parent.mkdir(parents=True, exist_ok=True)
        with args.jsonl.open("w", encoding="utf-8", newline="\n") as out:
            words = [
                {"word": word, "pinyin": key.split(), "count": count}
                for word, key, count in rows
            ]
            out.write(json.dumps({"topic": "四家开源模型词表", "words": words}, ensure_ascii=False))
            out.write("\n")
        print(f"JSONL {len(rows)} 条 → {args.jsonl}")

    print(f"合并 {len(rows)} 条（碎片丢掉 {fragments} 条、没读音 {no_reading} 条）→ {args.out}")


if __name__ == "__main__":
    main()
