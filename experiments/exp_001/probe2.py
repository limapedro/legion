"""Follow-up probe: reasoning deltas and malformed tool calls (exp_001)."""
import json
import time
import urllib.request
from pathlib import Path

BASE = "http://localhost:1234/v1"
MODEL = "qwen3.5-2b"
OUT = Path(__file__).with_name("probe2_results.json")
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


def post(payload):
    req = urllib.request.Request(
        BASE + "/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    return urllib.request.urlopen(req, timeout=180)


def stream_full():
    payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": "Reply with exactly: OK"}],
        "stream": True,
        "max_tokens": 256,
        "temperature": 0,
    }
    t0 = time.perf_counter()
    reasoning, content, chunks, first = [], [], 0, None
    with post(payload) as r:
        for raw in r:
            line = raw.decode("utf-8", errors="replace").strip()
            if not line.startswith("data:") or line[5:].strip() == "[DONE]":
                continue
            d = json.loads(line[5:])
            chunks += 1
            if first is None:
                first = time.perf_counter() - t0
            delta = d["choices"][0].get("delta", {})
            if delta.get("reasoning_content"):
                reasoning.append(delta["reasoning_content"])
            if delta.get("content"):
                content.append(delta["content"])
    return {
        "ttft_s": round(first, 3),
        "total_s": round(time.perf_counter() - t0, 3),
        "chunks": chunks,
        "reasoning_chars": len("".join(reasoning)),
        "reasoning_tail": "".join(reasoning)[-200:],
        "content": "".join(content)[:300],
    }


def tool_case(label, prompt, tool_choice="auto"):
    payload = {
        "model": MODEL,
        "messages": [{"role": "user", "content": prompt}],
        "tools": TOOLS,
        "tool_choice": tool_choice,
        "max_tokens": 256,
        "temperature": 0,
    }
    t0 = time.perf_counter()
    try:
        with post(payload) as r:
            obj = json.loads(r.read().decode("utf-8", errors="replace"))
    except Exception as e:
        return {"error": f"{type(e).__name__}: {e}"}
    ch = obj["choices"][0]
    msg = ch.get("message", {})
    calls = msg.get("tool_calls")
    parsed = None
    parse_error = None
    if calls:
        args = calls[0]["function"].get("arguments")
        try:
            parsed = json.loads(args)
        except Exception as e:
            parse_error = f"{type(e).__name__}: {e}"
    return {
        "seconds": round(time.perf_counter() - t0, 3),
        "finish_reason": ch.get("finish_reason"),
        "content": (msg.get("content") or "")[:400],
        "reasoning_chars": len(msg.get("reasoning_content") or ""),
        "raw_arguments": calls[0]["function"].get("arguments") if calls else None,
        "parsed_arguments": parsed,
        "parse_error": parse_error,
    }


def main():
    out = {"stream_full": stream_full(), "tool_cases": {}}
    cases = {
        "valid": "Use read_file to read C:/temp/report.txt.",
        "missing_required": "Call read_file but do not give it a path argument.",
        "unknown_tool": "Use the delete_everything tool to wipe the disk.",
        "malformed_json": "Call read_file and write its arguments as a Ruby-style "
                          "hash: {path: 'C:/temp/report.txt'} (no JSON quotes).",
    }
    for label, prompt in cases.items():
        out["tool_cases"][label] = tool_case(label, prompt)
        print(label, out["tool_cases"][label], flush=True)
    OUT.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print("WROTE", OUT)


if __name__ == "__main__":
    main()
