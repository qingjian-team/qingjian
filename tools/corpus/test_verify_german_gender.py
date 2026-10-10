"""verify_german_gender.py 的离线单元测试（mock Wiktionary dump + mock JSONL）。"""
import json
import os
import tempfile
import unittest

import verify_german_gender as vgg


def kaikki_noun(word: str, arg1: str, forms=None) -> str:
    """造一行真实形态的 kaikki 德语名词 JSONL（de-noun 模板，可带 forms）。"""
    entry = {
        "pos": "noun",
        "word": word,
        "head_templates": [{"name": "de-noun", "args": {"1": arg1}}],
    }
    if forms is not None:
        entry["forms"] = [{"form": f, "tags": t} for f, t in forms]
    return json.dumps(entry, ensure_ascii=False)


def kaikki_proper_name(word: str, arg1: str, forms=None) -> str:
    """造一行真实形态的 kaikki 德语专名 JSONL（de-proper noun 模板）。"""
    entry = {
        "pos": "name",
        "word": word,
        "head_templates": [{"name": "de-proper noun", "args": {"1": arg1}}],
    }
    if forms is not None:
        entry["forms"] = [{"form": f, "tags": t} for f, t in forms]
    return json.dumps(entry, ensure_ascii=False)


class TestGenderCodes(unittest.TestCase):
    def test_plain_codes(self):
        self.assertEqual(vgg._gender_codes("m"), ["m"])
        self.assertEqual(vgg._gender_codes("f"), ["f"])
        self.assertEqual(vgg._gender_codes("n"), ["n"])

    def test_codes_with_declension_args(self):
        self.assertEqual(vgg._gender_codes("m,(e)s"), ["m"])
        self.assertEqual(vgg._gender_codes("n,,^er"), ["n"])
        self.assertEqual(vgg._gender_codes("n,s,-:s〔nonstandard〕"), ["n"])


class TestExtractArticle(unittest.TestCase):
    def test_de_noun_template(self):
        self.assertEqual(vgg.extract_article(kaikki_noun("Schule", "f")), "die")
        self.assertEqual(vgg.extract_article(kaikki_noun("Freund", "m,(e)s")), "der")
        self.assertEqual(vgg.extract_article(kaikki_noun("Haus", "n,,^er")), "das")

    def test_extract_all_articles_single(self):
        self.assertEqual(vgg.extract_all_articles(kaikki_noun("Schule", "f")), ["die"])

    def test_not_noun(self):
        line = json.dumps({"word": "entwickeln", "pos": "verb"})
        self.assertEqual(vgg.extract_all_articles(line), [])
        self.assertIsNone(vgg.extract_article(line))

    def test_generic_head_template_has_no_gender(self):
        # 缩写 / 屈折形式条目用通用 head 模板，无性别 → 空
        line = json.dumps({
            "pos": "noun",
            "word": "AA",
            "head_templates": [{"name": "head", "args": {"1": "de", "2": "noun"}}],
        })
        self.assertEqual(vgg.extract_all_articles(line), [])

    def test_no_head_templates(self):
        line = json.dumps({"word": "Xyz", "pos": "noun"})
        self.assertEqual(vgg.extract_all_articles(line), [])


class TestLoadWiktionary(unittest.TestCase):
    def test_load_kaikki_jsonl(self):
        lines = [
            kaikki_noun("Schule", "f"),
            kaikki_noun("Freund", "m,(e)s"),
            json.dumps({"word": "entwickeln", "pos": "verb"}),
            json.dumps({
                "pos": "noun", "word": "AA",
                "head_templates": [{"name": "head", "args": {"1": "de", "2": "noun"}}],
            }),
        ]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            f.write("\n".join(lines))
            path = f.name
        try:
            mapping, plurals = vgg.load_wiktionary(path)
            self.assertEqual(mapping["Schule"], ["die"])
            self.assertEqual(mapping["Freund"], ["der"])
            self.assertNotIn("entwickeln", mapping)
            self.assertNotIn("AA", mapping)
            self.assertEqual(plurals, set())
        finally:
            os.unlink(path)

    def test_homograph_genders_merge(self):
        # See 在 dump 里是两个同名词条（湖 m / 海 f），加载时合并
        lines = [kaikki_noun("See", "m"), kaikki_noun("See", "f")]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            f.write("\n".join(lines))
            path = f.name
        try:
            mapping, _ = vgg.load_wiktionary(path)
            self.assertEqual(sorted(mapping["See"]), ["der", "die"])
        finally:
            os.unlink(path)

    def test_load_tsv(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".tsv", delete=False, encoding="utf-8") as f:
            f.write("Schule\tdie\nFreund\tder\n")
            path = f.name
        try:
            mapping, _ = vgg.load_wiktionary(path)
            self.assertEqual(mapping["Schule"], ["die"])
            self.assertEqual(mapping["Freund"], ["der"])
        finally:
            os.unlink(path)

    def test_proper_noun_templates(self):
        # de-proper noun 的 m/f/n.article 与 p.article（复数专名冠词唯 die）
        lines = [
            kaikki_proper_name("Sowjetunion", "f.article"),
            kaikki_proper_name("USA", "p.article"),
            kaikki_proper_name("Rhein", "m.article"),
        ]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            f.write("\n".join(lines))
            path = f.name
        try:
            mapping, _ = vgg.load_wiktionary(path)
            self.assertEqual(mapping["Sowjetunion"], ["die"])
            self.assertEqual(mapping["USA"], ["die"])
            self.assertEqual(mapping["Rhein"], ["der"])
        finally:
            os.unlink(path)

    def test_plural_only_head_code(self):
        # np 码：底层中性但 plural-only，词头自身定冠词恒 die（Daten）
        line = kaikki_noun(
            "Daten", "np",
            forms=[("Daten", ["definite", "nominative", "plural"])],
        )
        self.assertEqual(vgg.extract_all_articles(line), ["die"])

    def test_adjectival_noun_weak_forms_indexed(self):
        # 名词化形容词成对词条：Abgeordneter(m) 的弱变化主格是 der Abgeordnete，
        # Abgeordnete(f) 是 die Abgeordnete；词形索引合并后两种冠词都合法
        lines = [
            kaikki_noun("Abgeordneter", "+", forms=[
                ("der Abgeordnete", ["definite", "nominative"]),
                ("Abgeordnete", ["plural"]),
            ]),
            kaikki_noun("Abgeordnete", "+", forms=[
                ("die Abgeordnete", ["definite", "nominative"]),
                ("Abgeordnete", ["plural"]),
            ]),
        ]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            f.write("\n".join(lines))
            path = f.name
        try:
            mapping, plurals = vgg.load_wiktionary(path)
            self.assertEqual(sorted(mapping["Abgeordnete"]), ["der", "die"])
            self.assertIn("Abgeordnete", plurals)
        finally:
            os.unlink(path)

    def test_plus_code_gender_from_forms(self):
        # de-noun 的「+」码（名词化形容词）：性别从 forms 的定指主格形式取
        line = kaikki_noun(
            "Inneres", "+",
            forms=[("das Innere", ["definite", "nominative"]), ("Inneren", ["genitive"])],
        )
        self.assertEqual(vgg.extract_all_articles(line), ["das"])

    def test_proper_noun_gender_from_canonical_form(self):
        # 专名模板没有 article 后缀时，从 canonical 带冠词形式兜底
        line = kaikki_proper_name(
            "USA", "p.article",
            forms=[("die USA", ["canonical", "plural"]), ("USA", ["definite", "nominative", "plural"])],
        )
        articles = vgg.extract_all_articles(line)
        self.assertIn("die", articles)

    def test_plain_proper_name_without_template_skipped(self):
        # 国名人名等零冠词专名没有 de-proper noun 模板：不收（LLM 侧本就无冠词）
        line = json.dumps({
            "pos": "name", "word": "China",
            "head_templates": [{"name": "head", "args": {"1": "de", "2": "proper noun"}}],
        }, ensure_ascii=False)
        self.assertEqual(vgg.extract_all_articles(line), [])

    def test_plural_index_from_forms(self):
        # Datum 的裸复数形式 Daten 进复数索引；Haus 的 ^er 是构词规则不是具体形式
        lines = [
            kaikki_noun("Datum", "n,,Daten", forms=[("Daten", ["plural"]), ("Datum", ["canonical"])]),
            kaikki_noun("Haus", "n,,^er", forms=[("Häuser", ["plural"])]),
        ]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            f.write("\n".join(lines))
            path = f.name
        try:
            mapping, plurals = vgg.load_wiktionary(path)
            self.assertEqual(mapping["Datum"], ["das"])
            self.assertIn("Daten", plurals)
            self.assertIn("Häuser", plurals)
        finally:
            os.unlink(path)


class TestCheckAgreement(unittest.TestCase):
    def test_full_agreement(self):
        wiktionary = {"Schule": ["die"], "Freund": ["der"]}
        entries = [
            {"word": "学校", "pos": "n.", "de": ["die Schule"]},
            {"word": "朋友", "pos": "n.", "de": ["der Freund"]},
        ]
        result = vgg.check_agreement(entries, wiktionary)
        self.assertEqual(result["total_checked"], 2)
        self.assertEqual(result["agree"], 2)
        self.assertEqual(result["disagree"], 0)
        self.assertEqual(result["rate"], 1.0)

    def test_partial_agreement(self):
        wiktionary = {"Schule": ["die"], "Freund": ["der"]}
        entries = [
            {"word": "学校", "pos": "n.", "de": ["die Schule"]},
            {"word": "朋友", "pos": "n.", "de": ["das Freund"]},
        ]
        result = vgg.check_agreement(entries, wiktionary)
        self.assertEqual(result["total_checked"], 2)
        self.assertEqual(result["agree"], 1)
        self.assertEqual(result["disagree"], 1)
        self.assertEqual(result["rate"], 0.5)

    def test_homograph_agreement(self):
        # LLM 给 die See，Wiktionary 有 der/die 两性 → 一致
        wiktionary = {"See": ["der", "die"]}
        entries = [{"word": "海", "pos": "n.", "de": ["die See"]}]
        result = vgg.check_agreement(entries, wiktionary)
        self.assertEqual(result["agree"], 1)

    def test_no_article_excluded(self):
        wiktionary = {"Schule": ["die"]}
        entries = [{"word": "学校", "pos": "n.", "de": ["Schule"]}]
        result = vgg.check_agreement(entries, wiktionary)
        self.assertEqual(result["total_checked"], 0)
        self.assertEqual(result["no_article"], 1)

    def test_unmatched_excluded(self):
        # Wiktionary 查无此词：单列，不参与一致率
        wiktionary = {"Schule": ["die"]}
        entries = [{"word": "学校", "pos": "n.", "de": ["die NeueErfindung"]}]
        result = vgg.check_agreement(entries, wiktionary)
        self.assertEqual(result["total_checked"], 0)
        self.assertEqual(result["unmatched"], 1)

    def test_non_noun_excluded(self):
        entries = [{"word": "开发", "pos": "v.", "de": ["entwickeln"]}]
        result = vgg.check_agreement(entries, {})
        self.assertEqual(result["total_checked"], 0)

    def test_plural_form_agrees_with_die_only(self):
        # Daten 是 Datum 的复数：复数定冠词唯 die，LLM 给 die 一致、给 der 不一致
        entries_die = [{"word": "数据", "pos": "n.", "de": ["die Daten"]}]
        result = vgg.check_agreement(entries_die, {}, {"Daten"})
        self.assertEqual(result["agree"], 1)
        self.assertEqual(result["disagree"], 0)

        entries_der = [{"word": "数据", "pos": "n.", "de": ["der Daten"]}]
        result = vgg.check_agreement(entries_der, {}, {"Daten"})
        self.assertEqual(result["agree"], 0)
        self.assertEqual(result["disagree"], 1)


class TestMainExitCode(unittest.TestCase):
    def _make_entries(self, agree_count: int, disagree_count: int):
        """造名词数据：agree_count 条 der Wort{i} 一致、disagree_count 条不一致。"""
        entries = [
            {"word": f"词{i}", "pos": "n.", "de": [f"der Wort{i}"]}
            for i in range(agree_count)
        ]
        entries += [
            {"word": f"错词{i}", "pos": "n.", "de": [f"das Irrtum{i}"]}
            for i in range(disagree_count)
        ]
        return entries

    def test_pass(self):
        entries = self._make_entries(160, 0)
        wiktionary = {f"Wort{i}": ["der"] for i in range(160)}
        code = vgg.main_core(entries, wiktionary, sample=None, out_path=None)
        self.assertEqual(code, 0)

    def test_fail(self):
        entries = self._make_entries(150, 5)
        wiktionary = {f"Wort{i}": ["der"] for i in range(150)}
        wiktionary.update({f"Irrtum{i}": ["der"] for i in range(5)})
        code = vgg.main_core(entries, wiktionary, sample=None, out_path=None)
        self.assertEqual(code, 1)

    def test_too_few_checked(self):
        wiktionary = {"Schule": ["die"]}
        entries = [{"word": "学校", "pos": "n.", "de": ["die Schule"]}]
        code = vgg.main_core(entries, wiktionary, sample=None, out_path=None)
        self.assertEqual(code, 1)

    def test_sample_reproducible(self):
        # 固定 seed 时抽样结果可复现
        entries = self._make_entries(200, 0)
        import random
        random.seed(42)
        s1 = random.sample(entries, 100)
        random.seed(42)
        s2 = random.sample(entries, 100)
        self.assertEqual(s1, s2)


if __name__ == "__main__":
    unittest.main()
