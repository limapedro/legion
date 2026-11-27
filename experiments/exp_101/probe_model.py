"""Small, read-only probe of LM Studio's OpenAI-compatible API.

Usage: python probe_model.py [model_id]
Prints: model inventory, streaming SSE evidence, tool-call evidence.
No downloads, no persistent state.
"""
import json
import sys
import urllib.request

BASE = "http://localhost:1234/v1"
MODEL = sys.argv[1] if len(sys.argv) > 1 else "qwen3.5-2b"


def request(path, payload=None, stream=False):
    data = json.dumps(payload).encode() if payload is not None else None
    headers = {"Content-Type": "application/json"}
    req = urllib.request.Request(BASE + path, data=data, headers=headers)
    return urllib.request.urlopen(req, timeout=60)


def main():
    with request("/models") as r:
        models = json.load(r)
    print("MODELS:")
    for m in models["data"]:
        print("  " + m["id"])

    stream_payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Reply with exactly: PONG"}],
        "max_tokens": 16,
        "stream": True,
    }
    print("STREAM:")
    try:
        with request("/chat/completions", stream_payload, True) as r:
            print("  content-type:", r.headers.get("Content-Type"))
            lines = []
            for raw in r:
                line = raw.decode("utf-8", "replace").rstrip()
                if line.startswith("data:"):
                    lines.append(line[:160])
                if len(lines) >= 4:
                    break
            for line in lines:
                print("  " + line)
    except Exception as exc:
        print("  ERROR:", type(exc).__name__, str(exc)[:200])

    tools = [{
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get current weather for a city",
            "parameters": {
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"],
            },
        },
    }]
    tool_payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": "What is the weather in Porto? Use the tool."}],
        "tools": tools,
        "tool_choice": "auto",
        "max_tokens": 128,
        "stream": False,
    }
    print("TOOLS:")
    try:
        with request("/chat/completions", tool_payload) as r:
            result = json.load(r)
        msg = result["choices"][0]["message"]
        print("  finish_reason:", result["choices"][0].get("finish_reason"))
        print("  tool_calls:", json.dumps(msg.get("tool_calls"), ensure_ascii=False)[:400])
        print("  content:", repr(msg.get("content"))[:200])
    except Exception as exc:
        print("  ERROR:", type(exc).__name__, str(exc)[:200])


if __name__ == "__main__":
    main()
