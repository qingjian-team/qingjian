# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""把中文词表翻成青简的 `glossary-es.tsv`（`词\t[词性. ]译词\t…`），译文来自 Azure Translator。

词表、词性、英文释义三样都取自词库仓库（qingjian-dicts，与本仓库并排放）的 `official/glossary/en/glossary.tsv`：那张表是 LLM 生成的，
义项已经挑过一遍，直接复用等于免费多做一次义项消歧（动词尤其明显，见下面的翻译策略）。

产出词库仓库的 `official/glossary/es/glossary.tsv`，许可见那边的 dict.toml。
这张表**不经过** `tools/gloss-gen` 的 `export`（那只写 en / ja），是 tools/ 下这个脚本的
直接产物，可复现：有 Azure 免费 key 就能重跑。

Azure Translator F0 免费层每月 200 万字符，跑完这 23 万词用掉约 127 万（63%）。

用法：
    AZURE_TRANSLATOR_KEY=… AZURE_TRANSLATOR_REGION=eastus \\
        uv run tools/corpus/glossary_es.py
    uv run tools/corpus/glossary_es.py --limit 200                      # 小样本先看一眼
    uv run tools/corpus/glossary_es.py --verify ../qingjian-dicts/official/glossary/es/glossary.tsv

翻译策略（2026-09-14 用真实 API 做的三组对照实验，样本 20~60 词）：

    动词 v.    英文源 "to {英文释义} (verb)" 走 en->es。中文源只有 8/20 拿到原形，
               这个写法 19/20。
    形容词/副词 中文源 + 中文词性提示：认真->serio、复杂->complejo、便宜->barato。
    名词 n.    中文源裸词。提示实测净收益≈0（修单复数与个别错译，代价是同等量级的
               英文泄漏），而 n. 占全部词条的 71.5%，加提示要多烧 66 万字符。
    其余       中文源裸词。

已知问题（同样写在词库仓库 `official/glossary/README.md` 里）：名词约 13% 还是复数、个别错译
（椅子 -> presidente、工作 -> obra）、生僻字与专名准确率低、约 177 条全小写的英文短语没翻
（会买 -> will buy、你需要 -> you need）、7 条没有词性（源表就没有）、短语中间的词首字母
有时被无端大写（开发区 -> zona de Desarrollo）。上了之后有用户反馈可以再用 LLM 精修一轮。

要点（踩过的坑）：

  * 输出必须是 UTF-8 **无 BOM**、LF 换行。带 BOM 会让青简报
    `load glossary: line 1: missing senses`，并导致整个词库装配失败。
  * 释义表解析是严格的：任一行缺词或缺释义 → 整张表加载失败。所以写盘前逐行校验，
    校验不过的条目直接丢掉。
  * 单字词丢给翻译 API 偶尔会返回带解释的长串，必须清洗。
  * 中文源和英文源要分开成批，不能混在一个请求里（`from` 只能有一个）。
  * 输出按码点排序、去重（与 `glossary-en.tsv` 一致）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from pathlib import Path

# 释义表允许的词性（见 tools/gloss-gen 与 crates/qingjian-translate/src/glossary/mod.rs）
ALLOWED_POS = {
    "n.", "v.", "adj.", "adv.", "pron.", "prep.", "conj.", "num.",
    "m.", "part.", "int.", "phr.",
}

# 词性 -> 中文提示词
POS_HINT = {
    "n.": "名词", "v.": "动词", "adj.": "形容词", "adv.": "副词",
    "pron.": "代词", "prep.": "介词", "conj.": "连词", "num.": "数词",
    "m.": "量词", "part.": "助词", "int.": "感叹词", "phr.": "短语",
}

# 哪些词性加中文词性提示。名词不加，理由见模块 docstring。
HINT_POS = {"adj.", "adv."}

# 哪些词性改用英文源（`to {英文释义} (verb)` 走 en->es）。见模块 docstring。
EN_PIVOT_POS = {"v."}

# 翻译 API 明说「不知道」的返回值，直接丢
JUNK_GLOSSES = {
    "desconocido", "desconocida", "desconocidos", "desconocidas",
    "sin traducción", "sin traduccion", "no disponible", "no hay traducción",
}

MAX_GLOSS_CHARS = 40       # 单个译词长度上限，再长就是解释不是译词
MAX_GLOSS_WORDS = 4        # 单词数上限
MAX_ELEMENTS_PER_REQ = 1000
MAX_CHARS_PER_REQ = 50000
RATE_CHARS_PER_MIN = 25000  # F0 上限 2,000,000/小时；留余量按 1.5M/小时

ENDPOINT = "https://api.cognitive.microsofttranslator.com/translate"
TARGET = "es"
SOURCE_ZH = "zh-Hans"
SOURCE_EN = "en"

PAREN = re.compile(r"[（(][^）)]*[）)]")
# 只剥这三个前缀。不要加 el/la/los/las——那是冠词，剥掉只会把名词洗得更像名词。
LEAD_STOPWORD = re.compile(r"^(?:para|to|a)\s+", re.IGNORECASE)


def build_request(word: str, pos: str, en_gloss: str) -> tuple[str, str]:
    """返回 `(源语言, 实际发去翻译的文本)`。"""
    if pos in EN_PIVOT_POS and en_gloss:
        return SOURCE_EN, f"to {en_gloss} (verb)"
    hint = POS_HINT.get(pos) if pos in HINT_POS else None
    return SOURCE_ZH, (f"{word}（{hint}）" if hint else word)


def request_text(word: str, pos: str) -> str:
    """只看中文源那条路径的请求文本（测试用）。"""
    return build_request(word, pos, "")[1]


def clean_gloss(raw: str, force_lower_first: bool = False) -> str | None:
    """把 API 返回的一坨清洗成释义表能吃的单个译词。

    `force_lower_first`：Azure 会把首字母大写（开发 -> `Desarrollo`），但词条释义只有
    专名才该大写。调用方用上游英语表的大小写来判定是不是专名。
    """
    if not raw:
        return None
    s = raw.strip()
    s = PAREN.sub("", s)  # 去掉括号注释：manzana (fruta) -> manzana
    s = s.replace("|", "").replace("\t", " ").replace("\n", " ").replace("\r", " ")
    s = re.sub(r"\s+", " ", s).strip()
    s = s.strip(" .,;:·。，、")
    if not s:
        return None
    if len(s) > MAX_GLOSS_CHARS:
        return None
    if len(s.split(" ")) > MAX_GLOSS_WORDS:
        return None
    # 明显是句子 / 解释的丢掉
    if re.search(r"[.!?]\s+\S", s):
        return None
    if force_lower_first:
        s = s[0].lower() + s[1:]
    return s


def clean_verb_gloss(s: str | None) -> str | None:
    """动词额外清洗：API 偶尔返回 `para resolver` / `to solve` 这类带前缀的形式。

    实测 `to develop (verb)` 这个写法 19/20 直接给原形，这个函数只是兜底。
    """
    if not s:
        return None
    prev = None
    while prev != s:
        prev = s
        s = LEAD_STOPWORD.sub("", s).strip()
    return s or None


def parse_en_table(path: Path) -> list[tuple[str, str, str]]:
    """读 `glossary-en.tsv`，取出 `(中文词, 词性, 英文释义)`，按文件顺序（已排序）去重。

    词性直接复用，省得让翻译 API 再猜；英文释义既当动词源的输入，其首字母大小写也
    用来判定专名（`v. chew` 小写 vs `n. Sayuri Yoshinaga` 大写），决定译词要不要压小写。
    """
    rows: list[tuple[str, str, str]] = []
    seen: set[str] = set()
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            word = parts[0].strip()
            if not word or word in seen:
                continue
            pos = ""
            gloss = ""
            if len(parts) > 1:
                first = parts[1].strip()
                m = re.match(r"^([a-z]+\.)\s+(.*)$", first)
                if m and m.group(1) in ALLOWED_POS:
                    pos, gloss = m.group(1), m.group(2).strip()
                else:
                    gloss = first
            seen.add(word)
            rows.append((word, pos, gloss))
    return rows


def wants_lower_first(pos: str, en_gloss: str) -> bool:
    """要不要把译词首字母压小写。

    动词 / 形容词 / 副词永远不是专名，直接压；其余看英文释义的首字母——
    上游英语表对专名保留大写（`Sayuri Yoshinaga`），普通词是小写的（`chew`）。
    """
    if pos in ("v.", "adj.", "adv."):
        return True
    if not en_gloss:
        return False
    return en_gloss[0].islower()


class RateLimiter:
    """按字符数限速，保证不超 F0 的每小时额度。"""

    def __init__(self, chars_per_min: int) -> None:
        self.limit = chars_per_min
        self.used = 0
        self.window = time.monotonic()

    def spend(self, chars: int) -> None:
        now = time.monotonic()
        if now - self.window >= 60:
            self.window = now
            self.used = 0
        if self.used + chars > self.limit:
            sleep_for = 60 - (now - self.window)
            if sleep_for > 0:
                print(f"    限速：睡 {sleep_for:.0f}s（本分钟已用 {self.used} 字符）", flush=True)
                time.sleep(sleep_for)
            self.window = time.monotonic()
            self.used = 0
        self.used += chars


def translate_batch(
    texts: list[str],
    key: str,
    region: str,
    source: str = SOURCE_ZH,
    endpoint: str = ENDPOINT,
    retries: int = 5,
    verbose: bool = True,
    base_delay: float = 3.0,
) -> list[str | None]:
    import urllib.error
    import urllib.request

    body = json.dumps([{"Text": t} for t in texts], ensure_ascii=False).encode("utf-8")
    url = f"{endpoint}?api-version=3.0&from={source}&to={TARGET}"
    headers = {
        "Ocp-Apim-Subscription-Key": key,
        "Ocp-Apim-Subscription-Region": region,
        "Content-Type": "application/json; charset=UTF-8",
    }
    delay = base_delay
    for attempt in range(retries):
        req = urllib.request.Request(url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            out: list[str | None] = []
            for item in data:
                tr = item.get("translations") or []
                out.append(tr[0].get("text") if tr else None)
            if len(out) != len(texts):
                raise ValueError(f"返回条数不匹配 {len(out)} != {len(texts)}")
            return out
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", "replace")[:300]
            if e.code in (429, 500, 502, 503, 504):
                if verbose:
                    print(
                        f"    HTTP {e.code}，{delay:.0f}s 后重试（{attempt + 1}/{retries}）{detail}",
                        flush=True,
                    )
                time.sleep(delay)
                delay = min(delay * 2, 60)
                continue
            raise RuntimeError(f"HTTP {e.code}: {detail}") from e
        except Exception as e:  # noqa: BLE001
            if verbose:
                print(f"    网络异常 {e}，{delay:.0f}s 后重试（{attempt + 1}/{retries}）", flush=True)
            time.sleep(delay)
            delay = min(delay * 2, 60)
    raise RuntimeError("重试耗尽")


def verify_table(path: Path) -> tuple[int, list[str]]:
    """按 `crates/qingjian-translate/src/glossary/mod.rs::parse` 的规则逐行校验。"""
    errors: list[str] = []
    ok = 0
    with path.open(encoding="utf-8") as f:
        for i, raw in enumerate(f, start=1):
            line = raw.rstrip("\n").rstrip("\r")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            fields = [x.strip() for x in line.split("\t")]
            word = fields[0]
            if not word:
                errors.append(f"line {i}: missing word")
                continue
            senses = [s for s in fields[1:] if s]
            if not senses:
                errors.append(f"line {i}: missing senses  ({word})")
                continue
            bad = False
            for s in senses:
                m = re.match(r"^([a-z]+\.)\s*(.+)$", s)
                body = m.group(2).strip() if m and m.group(1) in ALLOWED_POS else s
                if not body:
                    errors.append(f"line {i}: empty sense body ({word})")
                    bad = True
            if not bad:
                ok += 1
    return ok, errors


def parse_pos_set(spec: str) -> set[str]:
    """`"adj.,adv."` -> `{"adj.", "adv."}`；`all` 全要，`none` 空集。"""
    s = spec.strip().lower()
    if s == "all":
        return set(POS_HINT)
    if s in ("none", ""):
        return set()
    return {
        p.strip() if p.strip().endswith(".") else p.strip() + "."
        for p in spec.split(",")
        if p.strip()
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--en-table", default="../qingjian-dicts/official/glossary/en/glossary.tsv",
                    help="词表与英文释义的来源（缺省上游那张英译表）")
    ap.add_argument("--out", default="../qingjian-dicts/official/glossary/es/glossary.tsv")
    ap.add_argument("--progress", default="data/generated/gloss-es-progress.jsonl",
                    help="续跑用的 JSONL，中断了再跑会跳过已处理的词")
    ap.add_argument("--key", default=os.environ.get("AZURE_TRANSLATOR_KEY", ""))
    ap.add_argument("--region", default=os.environ.get("AZURE_TRANSLATOR_REGION", ""))
    ap.add_argument("--limit", type=int, default=0, help="只处理前 N 个词（小样本验证用）")
    ap.add_argument("--batch", type=int, default=200, help="每请求词数")
    ap.add_argument("--endpoint", default=ENDPOINT, help="翻译接口地址（默认 Azure Translator）")
    ap.add_argument("--rate", type=int, default=RATE_CHARS_PER_MIN,
                    help="每分钟字符上限（F0 建议 25000）")
    ap.add_argument("--retry-failed", action="store_true",
                    help="续跑时连上次没得到合法译文的词一起重试（默认跳过，避免白烧额度）")
    ap.add_argument("--hint-pos", default=None,
                    help="对这些词性附加中文词性提示；all=全部，none=不加（缺省 adj.,adv.）")
    ap.add_argument("--en-pivot-pos", default=None,
                    help="对这些词性改用英文源 'to X (verb)'；none=不用英文源（缺省 v.）")
    ap.add_argument("--verify", metavar="FILE", help="只校验已有释义表，不调 API")
    args = ap.parse_args()

    if args.verify:
        ok, errors = verify_table(Path(args.verify))
        print(f"合法行: {ok}")
        print(f"错误行: {len(errors)}")
        for e in errors[:20]:
            print("  " + e)
        return 0 if not errors else 1

    if not args.key or not args.region:
        print("缺少 --key / --region（或用环境变量 AZURE_TRANSLATOR_KEY / _REGION）", file=sys.stderr)
        return 2

    global HINT_POS, EN_PIVOT_POS
    if args.hint_pos is not None:
        HINT_POS = parse_pos_set(args.hint_pos)
    if args.en_pivot_pos is not None:
        EN_PIVOT_POS = parse_pos_set(args.en_pivot_pos)
    print(f"中文源+词性提示: {sorted(HINT_POS) if HINT_POS else '（无）'}")
    print(f"英文源(动词轴): {sorted(EN_PIVOT_POS) if EN_PIVOT_POS else '（无）'}")

    rows = parse_en_table(Path(args.en_table))
    if args.limit:
        rows = rows[: args.limit]
    print(f"待翻译词数: {len(rows):,}")

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    prog_path = Path(args.progress)
    prog_path.parent.mkdir(parents=True, exist_ok=True)

    done: dict[str, str] = {}
    seen: set[str] = set()
    if prog_path.exists():
        with prog_path.open(encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                zh = rec.get("zh")
                if not zh:
                    continue
                seen.add(zh)
                if rec.get("es"):
                    done[zh] = rec["es"]
        print(f"断点续跑：进度文件已有 {len(seen):,} 条，其中有效 {len(done):,} 条")

    # 默认把「已尝试过但没得到合法译文」的词也视为已完成：翻译是确定性的，
    # 同样的输入必然得到同样的坏结果，重试只会白烧额度。要重试就加 --retry-failed。
    if args.retry_failed:
        todo = [(w, p, g) for w, p, g in rows if w not in done]
    else:
        todo = [(w, p, g) for w, p, g in rows if w not in seen]

    # 按源语言分股：一次请求只能有一个 from 参数，混批会串味。
    jobs: list[tuple[str, str, str, str, str]] = []
    for w, p, g in todo:
        src, text = build_request(w, p, g)
        jobs.append((w, p, g, src, text))
    streams = [(src, [j for j in jobs if j[3] == src]) for src in (SOURCE_ZH, SOURCE_EN)]
    streams = [(src, js) for src, js in streams if js]
    total_jobs = len(jobs)
    print(f"本次需翻译: {total_jobs:,}  ("
          + " / ".join(f"{src}: {len(js):,}" for src, js in streams) + ")")

    limiter = RateLimiter(args.rate)
    prog_f = prog_path.open("a", encoding="utf-8")
    t0 = time.time()
    processed = 0

    for source, stream in streams:
        i = 0
        while i < len(stream):
            chunk: list[tuple[str, str, str, str]] = []
            chars = 0
            while i < len(stream) and len(chunk) < min(args.batch, MAX_ELEMENTS_PER_REQ):
                w, p, g, _src, text = stream[i]
                if chunk and chars + len(text) > MAX_CHARS_PER_REQ:
                    break
                chunk.append((w, p, g, text))
                chars += len(text)
                i += 1

            limiter.spend(chars)
            results = translate_batch([t for *_, t in chunk], args.key, args.region,
                                      source=source, endpoint=args.endpoint)

            for (w, pos, en, _t), raw in zip(chunk, results):
                es = clean_gloss(raw or "", wants_lower_first(pos, en))
                if pos in EN_PIVOT_POS:
                    es = clean_verb_gloss(es)
                if not es or es.lower() == w.lower() or es.lower() in JUNK_GLOSSES:
                    prog_f.write(json.dumps({"zh": w, "es": ""}, ensure_ascii=False) + "\n")
                    continue
                gloss = f"{pos} {es}".strip()
                done[w] = gloss
                prog_f.write(json.dumps({"zh": w, "es": gloss}, ensure_ascii=False) + "\n")

            prog_f.flush()
            processed += len(chunk)
            elapsed = time.time() - t0
            eta = (total_jobs - processed) / max(processed / elapsed, 1e-9) if elapsed > 0 else 0
            print(f"  [{source}] {processed:,}/{total_jobs:,}  有效 {len(done):,}  "
                  f"用时 {elapsed:.0f}s  剩余约 {eta:.0f}s", flush=True)

    prog_f.close()

    # 写最终表：无 BOM、LF、按码点排序（与 glossary-en.tsv 一致）
    with out_path.open("w", encoding="utf-8", newline="\n") as f:
        f.write("# 由 tools/corpus/glossary_es.py 生成（Azure Translator F0）。词	[词性. ]译词	[词性. ]译词\n")
        for w in sorted(done):
            f.write(f"{w}\t{done[w]}\n")

    ok, errors = verify_table(out_path)
    print()
    print("=" * 50)
    print(f"输出文件     : {out_path}")
    print(f"源词总数     : {len(rows):,}")
    print(f"有西语释义   : {len(done):,}  ({100 * len(done) / max(len(rows), 1):.1f}%)")
    print(f"自检合法行   : {ok:,}")
    print(f"自检错误行   : {len(errors):,}")
    for e in errors[:10]:
        print("  " + e)
    head = out_path.read_bytes()[:3]
    print(f"文件头字节   : {head.hex()}  "
          + ("(无 BOM，正常)" if head != b"\xef\xbb\xbf" else "(有 BOM，必须修掉！)"))
    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
