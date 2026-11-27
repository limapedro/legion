"""Small reliability probe for qwen3.5-2b on LM Studio's OpenAI-compatible API.

Usage: python probe_reliability.py [model_id]
No downloads; each request uses <= 128 max_tokens.
"""
import json
import sys
import time
import urllib.request
import urllib.error

BASE = "http://localhost:1234/v1/chat/completions"
MODEL = sys.argv[1] if len(sys.argv) > 1 else "qwen3.5-2b"
TOOL = {
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
}


def post(payload, stream=False, timeout=35):
    req = urllib.request.Request(
        BASE,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
    )
    return urllib.request.urlopen(req, timeout=timeout)


def call(prompt, tool_choice="auto"):
    payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": prompt}],
        "tools": [TOOL],
        "tool_choice": tool_choice,
        "max_tokens": 160,
        "stream": False,
    }
    t0 = time.perf_counter()
    try:
        with post(payload) as r:
            data = json.load(r)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", "replace")[:300]
        print(f"PROMPT={prompt!r}")
        print(f"  HTTP_ERROR={exc.code} body={body!r}")
        return []
    dt = time.perf_counter() - t0
    msg = data["choices"][0]["message"]
    calls = msg.get("tool_calls") or []
    summary = []
    for c in calls:
        fn = c.get("function", {})
        raw = fn.get("arguments", "")
        try:
            parsed = json.loads(raw)
            valid = isinstance(parsed.get("city"), str)
            summary.append(f"{fn.get('name')} args={raw[:90]!r} schema_ok={valid}")
        except Exception as exc:
            summary.append(f"{fn.get('name')} args={raw[:90]!r} JSON_ERROR={type(exc).__name__}")
    print(f"PROMPT={prompt!r}")
    print(f"  elapsed_s={dt:.2f} finish={data['choices'][0].get('finish_reason')}")
    print(f"  content={repr(msg.get('content'))[:100]}")
    print(f"  calls={summary or 'NONE'}")
    return summary


def stream_latency():
    payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Reply with exactly: PONG"}],
        "max_tokens": 32,
        "stream": True,
    }
    t0 = time.perf_counter()
    with post(payload) as r:
        first = None
        chunks = 0
        for raw in r:
            if raw.startswith(b"data:"):
                chunks += 1
                if first is None:
                    first = time.perf_counter() - t0
                if chunks >= 3:
                    break
    print(f"STREAM first_delta_s={first:.3f} chunks_read={chunks}")


def cancel_stream():
    payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Write a long list of numbers, at least 200 words."}],
        "max_tokens": 128,
        "stream": True,
    }
    r = post(payload)
    chunks = 0
    for raw in r:
        if raw.startswith(b"data:"):
            chunks += 1
            if chunks >= 2:
                break
    t0 = time.perf_counter()
    try:
        r.close()
    except Exception as exc:
        print(f"CANCEL close_error={type(exc).__name__}: {str(exc)[:80]}")
    print(f"CANCEL chunks_before_close={chunks} close_s={time.perf_counter() - t0:.4f}")


def main():
    print(f"MODEL={MODEL}")
    call("Call get_weather for Porto.")
    call("Use get_weather to check the weather. Do not ask me where.")
    call("Call get_weather with city set to the number 42.")
    call("Just say hello.", tool_choice="required")
    stream_latency()
    cancel_stream()


if __name__ == "__main__":
    main()
