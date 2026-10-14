"""仅绑定本机回环地址的兼容接口；不记录请求原文或译文。"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hmac
import json

from .requests import completion_content, completion_response


def make_server(engine, token: str, port: int = 17863) -> ThreadingHTTPServer:
    if len(token) < 24:
        raise ValueError("请使用安装脚本生成的本地访问令牌")

    class ApiHandler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def setup(self):
            super().setup()
            self.connection.settimeout(15)

        def reply(self, code: int, payload: dict):
            data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(data)
            self.close_connection = True

        def do_GET(self):
            if self.path == "/health":
                self.reply(200, {"ready": True, "offline": True, "languages": ["zh", "en"]})
            else:
                self.reply(404, {"error": {"message": "接口不存在"}})

        def do_POST(self):
            if self.path != "/v1/chat/completions":
                self.reply(404, {"error": {"message": "接口不存在"}})
                return
            authorization = self.headers.get("Authorization", "")
            if not hmac.compare_digest(authorization.encode("utf-8"), ("Bearer " + token).encode("utf-8")):
                self.reply(401, {"error": {"message": "本地访问令牌不匹配"}})
                return
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if length <= 0 or length > 131_072:
                    self.reply(413, {"error": {"message": "请求过大或为空"}})
                    return
                body = json.loads(self.rfile.read(length))
                content = completion_content(body, engine.translate)
                self.reply(200, completion_response(content, "qingjian-offline-zh-en"))
            except (ValueError, TypeError, UnicodeError):
                self.reply(400, {"error": {"message": "请提供有效的中英文文字与目标语言"}})
            except (BrokenPipeError, ConnectionResetError):
                pass
            except Exception:
                self.reply(500, {"error": {"message": "本地翻译失败，原文未更改"}})

    return ThreadingHTTPServer(("127.0.0.1", port), ApiHandler)
