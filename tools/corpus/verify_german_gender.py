# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""Wiktionary 德语性别抽样校验：pilot 质量闸门。

用法：
    uv run tools/corpus/verify_german_gender.py \
        --jsonl data/generated/gloss-de-llm.jsonl \
        --wiktionary data/generated/kaikki.org-dictionary-German.jsonl \
        [--sample 200] [--seed 42] \
        [--out report.md]

Wiktionary 侧支持两种输入：
- kaikki.org 德语 JSONL dump：性别不在 genders 字段，而在 head_templates 里
  ``name == "de-noun"`` 模板的 ``args["1"]`` 首字符（m / f / n）；一词多性别
  在 dump 里是多个同名词条（如 See 同时有 m 与 f），加载时合并。用通用
  ``head`` 模板的条目是缩写 / 屈折形式等非独立 lemma，无性别，跳过。
- 预处理 TSV：``word\\tarticle``（der / die / das），一行一个。

退出码：一致率 ≥ 98% 且参与校验数 ≥ 150 → 0；否则 → 1。
"""
import argparse
import json
import random
import re
import sys
from pathlib import Path

ARTICLES = ("der", "die", "das")

# de-noun 模板 args["1"] 的首字符 → 定冠词。
GENDER_CODE_TO_ARTICLE = {
    "m": "der",
    "f": "die",
    "n": "das",
}

# de-proper noun 模板 args["1"] 的 article 后缀 → 定冠词；p = 复数专名（冠词唯 die）。
PROPER_CODE_TO_ARTICLE = {
    "m": "der",
    "f": "die",
    "n": "das",
    "p": "die",
}

# forms 里带冠词形式中，不允许与冠词前缀同现的标签。
_FORM_CASE_TAGS = {"definite", "indefinite", "genitive", "dative", "accusative"}


def _gender_codes(arg1: str) -> list[str]:
    """解析 de-noun 模板 args["1"]：首段逗号前的前导性别字符。

    常见取值："m" / "f" / "n,,^er" / "m,(e)s" / "n,s,-:s〔…〕"；
    多一个 ``p`` 表示 plural-only（如 ``np`` 的 Daten），词头自身只用复数；
    名词化形容词的「+」码没有性别码，返回空列表，性别改从 forms 取。
    """
    head = arg1.split(",", 1)[0].strip()
    codes = []
    for ch in head:
        if ch in GENDER_CODE_TO_ARTICLE or ch == "p":
            codes.append(ch)
        else:
            break
    return codes


def _nominative_forms(entry: dict) -> dict[str, list[str]]:
    """从 forms 提取定指主格词形 → 冠词（名词化形容词的弱变化词形不在词头上）。

    收 canonical 或 definite nominative 形式（``der Abgeordnete``、``das Innere``）。
    同一词形允许多冠词（Abgeordnete 同时对应 der 男 / die 女）。
    """
    out: dict[str, list[str]] = {}
    for form in entry.get("forms", []):
        text = (form.get("form") or "").strip()
        tags = set(form.get("tags") or [])
        canonical = "canonical" in tags
        definite_nominative = {"definite", "nominative"} <= tags
        if not (canonical or definite_nominative):
            continue
        if tags & (_FORM_CASE_TAGS - {"definite"}) and not canonical:
            continue
        for article in ARTICLES:
            if text.startswith(article + " "):
                word_form = text[len(article) + 1 :].strip()
                if word_form:
                    out.setdefault(word_form, [])
                    if article not in out[word_form]:
                        out[word_form].append(article)
    return out


def _article_from_forms(entry: dict) -> list[str]:
    """从 forms 的带冠词形式兜底提取冠词。

    适用模板给不出性别码的条目：名词化形容词（de-noun 的「+」，如 das Innere）
    与专名（如 canonical 形式 die USA）。只收 canonical 或定指主格形式。
    """
    articles: list[str] = []
    for form in entry.get("forms", []):
        text = (form.get("form") or "").strip()
        tags = set(form.get("tags") or [])
        canonical = "canonical" in tags
        definite_nominative = {"definite", "nominative"} <= tags
        if not (canonical or definite_nominative):
            continue
        if tags & (_FORM_CASE_TAGS - {"definite"}) and not canonical:
            continue
        for article in ARTICLES:
            if text.startswith(article + " ") and article not in articles:
                articles.append(article)
    return articles


def _articles_from_template(template: dict, entry: dict) -> list[str]:
    """从单个 head 模板提取冠词；模板性别码缺失时回退 forms。"""
    name = template.get("name")
    arg1 = ((template.get("args") or {}).get("1") or "").strip()
    if name == "de-noun":
        codes = _gender_codes(arg1)
        if "p" in codes:
            # plural-only 词头（如 Daten，码 np）：定冠词恒为 die
            return ["die"]
        if codes:
            return [GENDER_CODE_TO_ARTICLE[c] for c in codes]
        if arg1.startswith("+"):
            return _article_from_forms(entry)
    elif name == "de-proper noun":
        code = arg1.split(".", 1)[0].strip()
        if code in PROPER_CODE_TO_ARTICLE:
            return [PROPER_CODE_TO_ARTICLE[code]]
        return _article_from_forms(entry)
    return []


def extract_plural_forms(entry: dict) -> list[str]:
    """从 forms 提取裸复数形式（tags 恰为 ["plural"] 的完整词，非 ^er 构词规则）。"""
    out: list[str] = []
    for form in entry.get("forms", []):
        text = (form.get("form") or "").strip()
        tags = form.get("tags") or []
        if tags == ["plural"] and text and " " not in text and "^" not in text:
            out.append(text)
    return out


def extract_all_articles(line: str) -> list[str]:
    """从一行 Wiktionary JSONL 提取该词条的全部冠词（去重保序）。

    收普通名词（de-noun）与带 de-proper noun 模板的专名；缩写 / 屈折形式 /
    零冠词专名（无相应模板）返回空列表。
    """
    entry = json.loads(line)
    if entry.get("pos") not in ("noun", "name"):
        return []
    articles: list[str] = []
    for template in entry.get("head_templates", []):
        for article in _articles_from_template(template, entry):
            if article not in articles:
                articles.append(article)
    return articles


def extract_article(line: str) -> str | None:
    """提取首个冠词；非名词或无性别信息返回 None。"""
    articles = extract_all_articles(line)
    return articles[0] if articles else None


def load_wiktionary(path: str) -> tuple[dict[str, list[str]], set[str]]:
    """加载 Wiktionary 德语数据。

    返回 ``(冠词索引, 复数形式集合)``：冠词索引为 ``{word: [articles]}``，
    同名多性别合并去重；复数集合里的词是某名词的裸复数形式（定冠词恒为 die）。
    """
    path_obj = Path(path)
    mapping: dict[str, list[str]] = {}
    plurals: set[str] = set()
    if path_obj.suffix == ".jsonl":
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if entry.get("pos") not in ("noun", "name"):
                    continue
                word = entry.get("word", "")
                if not word:
                    continue
                articles = extract_all_articles(line)
                if articles:
                    merged = mapping.setdefault(word, [])
                    for article in articles:
                        if article not in merged:
                            merged.append(article)
                # 定指主格词形并入索引（弱变化词形不在词头上，如 der Abgeordnete）
                for word_form, form_articles in _nominative_forms(entry).items():
                    merged = mapping.setdefault(word_form, [])
                    for article in form_articles:
                        if article not in merged:
                            merged.append(article)
                if entry.get("pos") == "noun":
                    plurals.update(extract_plural_forms(entry))
    else:  # 假定 TSV：word\tarticle
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split("\t")
                if len(parts) >= 2:
                    word, article = parts[0], parts[1]
                    if article in ARTICLES:
                        mapping[word] = [article]
    return mapping, plurals


def check_agreement(
    entries: list[dict],
    wiktionary: dict[str, list[str]],
    plural_words: set[str] | None = None,
) -> dict:
    """核对 LLM 产出与 Wiktionary 性别是否一致。

    - 只看 ``pos == "n."`` 且首个译词以 der/die/das 开头的条目；
    - 无冠词名词计入 ``no_article`` 单列，不参与一致率；
    - Wiktionary 查无此词计入 ``unmatched`` 单列，不参与一致率；
    - 一词多性别时 LLM 给任一即算一致；
    - 译词名词是某词条的裸复数形式时，复数定冠词恒为 die：给 die 一致，其余不一致。
    """
    plural_words = plural_words or set()
    agree = 0
    disagree = 0
    no_article = 0
    unmatched = 0
    disagreements = []
    no_article_list = []
    unmatched_list = []
    for entry in entries:
        if entry.get("pos") != "n.":
            continue
        de = entry.get("de", [])
        if not de:
            continue
        first = de[0]
        match = re.match(r"^(der|die|das)\s+(.+)$", first)
        if not match:
            no_article += 1
            no_article_list.append((entry["word"], first))
            continue
        article, noun = match.group(1), match.group(2)
        wikt_articles = wiktionary.get(noun)
        if wikt_articles is None:
            if noun in plural_words:
                if article == "die":
                    agree += 1
                else:
                    disagree += 1
                    disagreements.append((entry["word"], first, ["die(复数)"]))
                continue
            unmatched += 1
            unmatched_list.append((entry["word"], first))
            continue
        if article in wikt_articles:
            agree += 1
        else:
            disagree += 1
            disagreements.append((entry["word"], first, wikt_articles))
    total_checked = agree + disagree
    rate = agree / total_checked if total_checked > 0 else 0.0
    return {
        "total_checked": total_checked,
        "agree": agree,
        "disagree": disagree,
        "no_article": no_article,
        "unmatched": unmatched,
        "rate": rate,
        "disagreements": disagreements,
        "no_article_list": no_article_list,
        "unmatched_list": unmatched_list,
    }


def main_core(
    entries: list[dict],
    wiktionary: dict[str, list[str]],
    sample: int | None,
    out_path: str | None,
    seed: int = 42,
    plural_words: set[str] | None = None,
) -> int:
    """核心逻辑：抽样、核对、输出报告，返回退出码。"""
    nouns = [e for e in entries if e.get("pos") == "n."]
    if sample is not None and len(nouns) > sample:
        random.seed(seed)
        nouns = random.sample(nouns, sample)
    result = check_agreement(nouns, wiktionary, plural_words)
    print(f"总名词数: {len(nouns)}")
    print(f"参与校验（有冠词且 Wiktionary 有该词）: {result['total_checked']}")
    print(f"一致: {result['agree']}")
    print(f"不一致: {result['disagree']}")
    print(f"LLM 未给冠词: {result['no_article']}")
    print(f"Wiktionary 查无此词: {result['unmatched']}")
    print(f"一致率: {result['rate']:.1%}")
    if result["disagreements"]:
        print("\n不一致清单:")
        for word, llm, wikt in result["disagreements"]:
            print(f"  {word}: LLM={llm}, Wiktionary={wikt}")
    if result["no_article_list"]:
        print("\nLLM 未给冠词清单:")
        for word, de in result["no_article_list"]:
            print(f"  {word}: {de}")
    if result["unmatched_list"]:
        print("\nWiktionary 查无此词清单:")
        for word, de in result["unmatched_list"]:
            print(f"  {word}: {de}")
    if out_path:
        report = [
            "# 德语冠词 pilot 校验报告",
            "",
            f"- 总名词数: {len(nouns)}",
            f"- 参与校验: {result['total_checked']}",
            f"- 一致: {result['agree']}",
            f"- 不一致: {result['disagree']}",
            f"- LLM 未给冠词: {result['no_article']}",
            f"- Wiktionary 查无此词: {result['unmatched']}",
            f"- 一致率: {result['rate']:.1%}",
            "",
        ]
        if result["disagreements"]:
            report.append("## 不一致清单")
            report.append("")
            for word, llm, wikt in result["disagreements"]:
                report.append(f"- {word}: LLM={llm}, Wiktionary={wikt}")
            report.append("")
        if result["no_article_list"]:
            report.append("## LLM 未给冠词清单")
            report.append("")
            for word, de in result["no_article_list"]:
                report.append(f"- {word}: {de}")
            report.append("")
        if result["unmatched_list"]:
            report.append("## Wiktionary 查无此词清单")
            report.append("")
            for word, de in result["unmatched_list"]:
                report.append(f"- {word}: {de}")
            report.append("")
        Path(out_path).write_text("\n".join(report), encoding="utf-8")
        print(f"\n报告已写入: {out_path}")
    if result["total_checked"] < 150:
        print("\n参与校验数 < 150，样本不足", file=sys.stderr)
        return 1
    if result["rate"] < 0.98:
        print("\n一致率 < 98%，质量闸门未通过", file=sys.stderr)
        return 1
    print("\n质量闸门通过（≥98%）")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(description="Wiktionary 德语性别抽样校验")
    parser.add_argument("--jsonl", required=True, help="gloss-de-llm.jsonl 路径")
    parser.add_argument(
        "--wiktionary", required=True, help="kaikki 德语 JSONL dump 或预处理 gender TSV"
    )
    parser.add_argument("--sample", type=int, default=None, help="抽样数（默认全部）")
    parser.add_argument("--seed", type=int, default=42, help="抽样随机种子（默认 42）")
    parser.add_argument("--out", default=None, help="输出报告 Markdown 路径")
    args = parser.parse_args()
    entries = []
    with open(args.jsonl, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    wiktionary, plurals = load_wiktionary(args.wiktionary)
    sys.exit(
        main_core(entries, wiktionary, args.sample, args.out, args.seed, plurals)
    )


if __name__ == "__main__":
    main()
