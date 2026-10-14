"""识别青简显式翻译请求；普通联想与词典兜底不触发模型。"""

import json


def completion_content(body: dict, translate) -> dict:
    if not isinstance(body, dict) or not isinstance(body.get("messages"), list):
        raise ValueError("请求缺少 messages")
    messages = body["messages"]
    content = next((message.get("content") for message in reversed(messages)
                    if isinstance(message, dict) and message.get("role") == "user"), None)
    if not isinstance(content, str):
        raise ValueError("请求缺少用户消息")
    try:
        payload = json.loads(content)
    except json.JSONDecodeError:
        # 设置页的连接测试只要求能收到非空响应。
        return {"status": "offline-ready"}
    if not isinstance(payload, dict):
        raise ValueError("用户消息必须为 JSON 对象")
    if "text" in payload and "target_language" in payload:
        if set(payload) != {"text", "target_language"}:
            raise ValueError("翻译请求只接受选中文字和目标语言")
        if not isinstance(payload["target_language"], str):
            raise ValueError("目标语言必须为字符串")
        return {"sentence": translate(payload["text"], payload["target_language"])}
    return {"words": [], "sentence": None, "answers": [], "items": []}


def completion_response(content: dict, model: str) -> dict:
    return {
        "id": "qingjian-offline", "object": "chat.completion", "created": 0,
        "model": model,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": json.dumps(content, ensure_ascii=False)},
                     "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
