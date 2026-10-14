"""命令行启动入口；运行阶段禁止向外部地址建立连接。"""

import argparse
import ipaddress
from pathlib import Path
import sys

from .engine import OfflineEngine
from .http import make_server


def deny_outbound(event, args):
    if event == "socket.connect":
        address = args[1]
        if isinstance(address, tuple):
            try:
                if ipaddress.ip_address(address[0]).is_loopback:
                    return
            except ValueError:
                pass
        raise OSError("离线翻译禁止连接外部地址")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--models", type=Path, required=True)
    parser.add_argument("--token-file", type=Path, required=True)
    parser.add_argument("--port", type=int, default=17863)
    args = parser.parse_args()
    sys.addaudithook(deny_outbound)
    engine = OfflineEngine(args.models)
    token = args.token_file.read_text(encoding="utf-8").strip()
    server = make_server(engine, token, args.port)
    print("青简离线翻译已就绪（中文 ↔ 英语）。", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
