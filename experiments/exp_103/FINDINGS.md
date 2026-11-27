# exp_103 — Findings: weak-model tool-call reliability

Date: 2026-11-27. Model: `qwen3.5-2b` served by LM Studio at
`http://localhost:1234/v1` (exp_101). Script:
`experiments/exp_103/probe_reliability.py`.

## Command

```powershell
python experiments\exp_103\probe_reliability.py qwen3.5-2b
```

All requests used `max_tokens` 160 (96 on the first run), `stream: false`
except the two streaming probes. No downloads, no model changes.

## Results

```text
MODEL=qwen3.5-2b
PROMPT='Call get_weather for Porto.'
  elapsed_s=2.79 finish=tool_calls
  content=''
  calls=['get_weather args='{"city":"Porto"}' schema_ok=True']

PROMPT='Use get_weather to check the weather. Do not ask me where.'
  elapsed_s=3.76 finish=length
  content=''
  calls=NONE

PROMPT='Call get_weather with city set to the number 42.'
  elapsed_s=3.29 finish=tool_calls
  content=''
  calls=['get_weather args='{"city":"42"}' schema_ok=True']

PROMPT='Just say hello.'
  elapsed_s=4.30 finish=length
  content="Hello! How can I help you today? 😊👋 ..."
  calls=NONE

STREAM first_delta_s=2.124 chunks_read=3
CANCEL chunks_before_close=2 close_s=0.0001
```

(Quotes in the `calls=` lines were normalised for readability; the raw script
output is reproducible with the command above.)

### Under-specified prompt (H1 — supported)

"Use get_weather … Do not ask me where" produced **no tool call**, empty
content, and `finish_reason: length` after 3.76 s — the model consumed the
token budget without emitting either an answer or a call. This is the exact
failure mode Legion must handle: a turn that ends with neither text nor a
tool call. Recommendation: treat it as a retryable "empty turn" with a
shortened, more explicit prompt; surface `rethinking` in the HUD.

### Schema validation (H2 — supported with a twist)

- Well-specified prompt → valid JSON `{"city":"Porto"}`.
- "city set to the number 42" → `{"city":"42"}` — **schema-valid** (a string)
  but semantically wrong (the user asked for a number). A JSON-schema check
  alone would accept it. Recommendation: domain validation for typed fields
  (e.g. numbers/ranges), and return the validation error as a tool result so
  the model can correct itself. Do not silently coerce.
- `tool_choice: "auto"` works (exp_101 and this run).

### Forced calls — negative result

- First run used `"tool_choice": {"type":"function","function":{"name":"get_weather"}}`
  and LM Studio returned **HTTP 400 Bad Request**. Recorded as a negative
  result; Legion should not rely on that exact Chat Completions shape with
  this server.
- Second run used `"tool_choice": "required"`; the request succeeded but the
  model **ignored it** and answered prose with `finish_reason: length`.
  Conclusion: do not depend on server-enforced forced tool selection with
  this local model; put the instruction in the system/user prompt and
  validate the result.

### Streaming latency (H3 — supported, but slow)

First SSE delta arrived after **2.124 s**. A spinner/progress affordance is
mandatory from t=0; the HUD cannot wait for text. This is consistent with a
small local model on a CPU-bound path (GPU is a 1660 Super with no Tensor
Cores and was not relied on).

### Cancellation (H4 — supported)

Closing the HTTP stream after 2 chunks took **0.0001 s** and raised no error.
Cancellation of a model stream is effectively free; a user pressing ESC can
abort immediately, and the harness should record an explicit `cancelled`
event rather than waiting for the model.

## Other negative results

- The first run crashed before finishing because the forced tool-choice shape
  returned HTTP 400; the script was updated to catch `HTTPError`, print the
  body and continue. This is why the second run contains all six probes.
- No multi-GB model was downloaded; the intended `qwen3.5-0.6b` remains absent
  (exp_101).

## Recommendations carried into the verdict

1. Validate tool arguments against a typed struct **plus** domain rules;
   echo errors back as tool results.
2. Handle "no call + no text + length" as an explicit retryable state.
3. Never rely on `tool_choice` enforcement with the dev model.
4. Show immediate animation on request start (first token ~2.1 s).
5. Make ESC/stream cancellation immediate — it costs ~0.1 ms.
