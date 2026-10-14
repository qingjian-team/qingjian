"""验证显式请求过滤、认证、错误返回与原文换行保留。"""

import json
import http.client
from pathlib import Path
import sys
import threading
import unittest
import urllib.error
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from qingjian_offline.__main__ import deny_outbound
from qingjian_offline.http import make_server
from qingjian_offline.requests import completion_content


class ApiTests(unittest.TestCase):
    def setUp(self):
        self.calls = []
        test = self

        class Engine:
            def translate(self, text, target):
                test.calls.append((text, target))
                if not isinstance(text, str) or not text.strip() or target not in ("zh", "en"):
                    raise ValueError("invalid translation")
                return "Hello.\nGood morning."

        self.token = "test-token-with-at-least-24-characters"
        self.server = make_server(Engine(), self.token, 0)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.base = f"http://127.0.0.1:{self.server.server_port}"

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()

    def post(self, payload, token=None):
        message = json.dumps(payload, ensure_ascii=False)
        body = json.dumps({"model": "test", "messages": [{"role": "user", "content": message}]}).encode()
        request = urllib.request.Request(self.base + "/v1/chat/completions", data=body,
                                         headers={"Authorization": "Bearer " + (token or self.token)})
        return urllib.request.urlopen(request, timeout=5)

    def test_translation_keeps_multiline_result_in_compatible_response(self):
        with self.post({"text": "你好。\n早上好。", "target_language": "en"}) as response:
            result = json.load(response)
        content = json.loads(result["choices"][0]["message"]["content"])
        self.assertEqual(content["sentence"], "Hello.\nGood morning.")
        self.assertEqual(self.calls, [("你好。\n早上好。", "en")])

    def test_background_prediction_and_gloss_never_translate(self):
        for payload in ({"letters": "nihao", "before": "私人内容"}, {"words": ["开发"]}):
            with self.post(payload) as response:
                result = json.load(response)
            content = json.loads(result["choices"][0]["message"]["content"])
            self.assertIsNone(content["sentence"])
        self.assertEqual(self.calls, [])

    def test_wrong_token_does_not_run_translation(self):
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.post({"text": "你好", "target_language": "en"}, token="wrong")
        self.assertEqual(error.exception.code, 401)
        self.assertEqual(self.calls, [])

    def test_extra_context_in_translation_is_rejected(self):
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.post({"text": "你好", "target_language": "en", "before": "不应发送的文字"})
        self.assertEqual(error.exception.code, 400)
        self.assertEqual(self.calls, [])

    def test_unsupported_target_returns_error_instead_of_empty_commit(self):
        with self.assertRaises(urllib.error.HTTPError) as error:
            self.post({"text": "你好", "target_language": "ja"})
        self.assertEqual(error.exception.code, 400)

    def test_health_reports_offline_mode(self):
        with urllib.request.urlopen(self.base + "/health") as response:
            self.assertEqual(json.load(response)["offline"], True)

    def test_body_limit_rejects_request_before_inference(self):
        # 只发超限声明，确认服务不等客户端上传大量正文就拒绝。
        connection = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=5)
        connection.request("POST", "/v1/chat/completions", headers={
            "Authorization": "Bearer " + self.token, "Content-Length": "131073",
        })
        self.assertEqual(connection.getresponse().status, 413)
        connection.close()
        self.assertEqual(self.calls, [])

    def test_non_ascii_authorization_is_rejected_without_crashing(self):
        connection = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=5)
        connection.request("POST", "/v1/chat/completions", headers={"Authorization": "Bearer ö"})
        self.assertEqual(connection.getresponse().status, 401)
        connection.close()
        self.assertEqual(self.calls, [])

    def test_runtime_policy_allows_loopback_and_rejects_external_connection(self):
        deny_outbound("socket.connect", (None, ("127.0.0.1", 17863)))
        with self.assertRaises(OSError):
            deny_outbound("socket.connect", (None, ("8.8.8.8", 443)))

    def test_malformed_json_is_not_sent_to_engine(self):
        with self.assertRaises(ValueError):
            completion_content({"messages": [{"role": "user", "content": "[]"}]}, lambda *_: self.fail())


if __name__ == "__main__":
    unittest.main()
