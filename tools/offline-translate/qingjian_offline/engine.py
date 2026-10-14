"""中英双向的本地模型推理；不下载模型，不调用在线服务。"""

import json
import os
from pathlib import Path
import re
import threading


class OfflineEngine:
    """加载 Argos 的 CTranslate2 模型并保留段落换行。"""

    MAX_CHARACTERS = 10_000
    MAX_INPUT_TOKENS = 180

    def __init__(self, models: Path):
        import ctranslate2
        import sentencepiece

        self._pairs = {}
        self._lock = threading.Lock()
        for metadata_file in models.rglob("metadata.json"):
            metadata = json.loads(metadata_file.read_text(encoding="utf-8"))
            direction = (metadata.get("from_code"), metadata.get("to_code"))
            if direction not in (("zh", "en"), ("en", "zh")):
                continue
            directory = metadata_file.parent
            # 原生推理库的 Windows 文件接口不能可靠打开中文路径，交给 Python 读取字节。
            tokenizer = sentencepiece.SentencePieceProcessor(
                model_proto=(directory / "sentencepiece.model").read_bytes()
            )
            model_files = {path.name: path.read_bytes() for path in (directory / "model").iterdir() if path.is_file()}
            translator = ctranslate2.Translator(
                "qingjian-" + "-".join(direction), device="cpu", compute_type="int8", files=model_files,
                inter_threads=1, intra_threads=min(4, os.cpu_count() or 1),
            )
            self._pairs[direction] = (tokenizer, translator)
        if set(self._pairs) != {("zh", "en"), ("en", "zh")}:
            raise RuntimeError("缺少中英双向翻译模型，请先运行安装脚本")

    def translate(self, text: str, target: str) -> str:
        if not isinstance(text, str) or not text.strip():
            raise ValueError("原文不能为空")
        if len(text) > self.MAX_CHARACTERS:
            raise ValueError("每次最多翻译 10000 个字符，请分段选择")
        if target not in ("zh", "en"):
            raise ValueError("目前离线模型支持中文和英语")
        tokenizer, translator = self._pairs[("zh" if target == "en" else "en", target)]
        output = []
        with self._lock:
            for paragraph in re.split(r"(\r\n|\r|\n)", text):
                if not paragraph.strip() or paragraph in ("\r\n", "\r", "\n"):
                    output.append(paragraph)
                    continue
                leading = paragraph[:len(paragraph) - len(paragraph.lstrip())]
                trailing = paragraph[len(paragraph.rstrip()):]
                tokens = tokenizer.encode(paragraph.strip(), out_type=str)
                chunks = [tokens[index:index + self.MAX_INPUT_TOKENS]
                          for index in range(0, len(tokens), self.MAX_INPUT_TOKENS)]
                results = translator.translate_batch(
                    chunks, beam_size=4, max_decoding_length=512, max_input_length=0,
                    repetition_penalty=1.05,
                )
                pieces = [tokenizer.decode(result.hypotheses[0]).replace("▁", " ").strip() for result in results]
                if not pieces or any(not piece for piece in pieces):
                    raise RuntimeError("本地模型没有返回译文")
                output.append(leading + (" " if target == "en" else "").join(pieces) + trailing)
        return "".join(output)
