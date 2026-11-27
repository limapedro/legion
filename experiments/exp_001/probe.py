"""Small, dependency-free probe of the local LM Studio OpenAI-compatible API.

Writes probe_results.json next to this file. No models are downloaded.
"""
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

BASE = "http://localhost:1234/v1"
OUT = Path(__file__).with_name("probe_results.json")


def get_json(path, timeout=15):
    with urllib.request.urlopen(BASE + path, timeout=timeout) as r:
        return json.loads(r.read().decode("utf-8"))


def post(path, payload, timeout=180):
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        BASE + path,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    return urllib.request.urlopen(req, timeout=timeout)


def stream_chat(model, timeout=180):
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": "Reply with exactly: OK"}],
        "stream": True,
        "max_tokens": 24,
        "temperature": 0,
    }
    t0 = time.perf_counter()
    chunks = 0
    first = None
    text = []
    finish = None
    try:
        with post("/chat/completions", payload, timeout) as r:
            status = r.status
            for raw in r:
                line = raw.decode("utf-8", errors="replace").strip()
                if not line.startswith("data:"):
                    continue
                data = line[5:].strip()
                if data == "[DONE]":
                    finish = "[DONE]"
                    break
                obj = json.loads(data)
                chunks += 1
                if first is None:
                    first = time.perf_counter() - t0
                delta = obj.get("choices", [{}])[0].get("delta", {})
                if delta.get("content"):
                    text.append(delta["content"])
    except Exception as e:  # record the real failure
        return {"error": f"{type(e).__name__}: {e}", "status": None}
    return {
        "status": status,
        "ttft_s": round(first, 3) if first is not None else None,
        "total_s": round(time.perf_counter() - t0, 3),
        "chunks": chunks,
        "text": "".join(text)[:400],
        "finish": finish,
    }


TOOLS = [{
    "type": "function",
    "function": {
        "name": "read_file",
        "description": "Read a UTF-8 text file and return its contents.",
        "parameters": {
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"],
        },
    },
}]


def tool_chat(model, timeout=180):
    payload = {
        "model": model,
        "messages": [{
            "role": "user",
            "content": "Use the read_file tool to read C:/temp/report.txt. "
                       "Call the tool, do not just describe it.",
        }],
        "tools": TOOLS,
        "tool_choice": "auto",
        "max_tokens": 160,
        "temperature": 0,
        "stream": False,
    }
    t0 = time.perf_counter()
    try:
        with post("/chat/completions", payload, timeout) as r:
            obj = json.loads(r.read().decode("utf-8", errors="replace"))
    except Exception as e:
        return {"error": f"{type(e).__name__}: {e}"}
    msg = obj.get("choices", [{}])[0].get("message", {})
    return {
        "total_s": round(time.perf_counter() - t0, 3),
        "finish_reason": obj.get("choices", [{}])[0].get("finish_reason"),
        "content": (msg.get("content") or "")[:600],
        "tool_calls": msg.get("tool_calls"),
    }


def main():
    results = {"models": None, "chats": {}, "tools": {}}
    try:
        results["models"] = [m["id"] for m in get_json("/models")["data"]]
    except Exception as e:
        results["models_error"] = f"{type(e).__name__}: {e}"
        OUT.write_text(json.dumps(results, indent=2), encoding="utf-8")
        print(json.dumps(results, indent=2))
        return

    candidates = [
        m for m in results["models"]
        if "qwen" in m.lower() or "inkling-2-mini@q8_0" == m
    ][:3]
    for model in candidates:
        print(f"probing {model} ...", flush=True)
        results["chats"][model] = stream_chat(model)
        print(f"  stream: {results['chats'][model]}", flush=True)
        results["tools"][model] = tool_chat(model)
        print(f"  tools: {results['tools'][model]}", flush=True)
        OUT.write_text(json.dumps(results, indent=2), encoding="utf-8")

    OUT.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print("WROTE", OUT)


if __name__ == "__main__":
    main()
