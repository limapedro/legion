# FINDINGS — exp_001: LM Studio test model and failure modes

Executed 2026-11-27 on the target machine (Windows, i5-12400F, 32 GB RAM,
GTX 1660 Super 6 GB). Scripts: `probe.py`, `probe2.py`; raw output:
`probe_results.json`, `probe2_results.json`. Commands: `python experiments\exp_001\probe.py`,
`python experiments\exp_001\probe2.py`, `lms ls`, plus inline urllib probes for
`reasoning_effort` and streaming.

## H1 — Model availability: CONFIRMED, then partly superseded

- At bootstrap `/v1/models` had **no Qwen3.5 0.6B**. `lms ls` confirms it is not
  downloaded; the machine has 9 models / 80.99 GB and the smallest Qwen is
  **`qwen3.5-2b`** (2B, 1.94 GB, arch `qwen35`, currently LOADED).
- By the time the probe ran, `qwen3.5-2b` had appeared in `/v1/models` (it was
  not in the first listing). `qwen3.5-35b-a3b` (35B-A3B, 12.44 GB) was also
  loaded. Exact ids at probe time are in `probe_results.json`.
- **Closest small Qwen available: `qwen3.5-2b`.** It is ~3.3x the parameter
  count of the requested 0.6B but still fits comfortably in VRAM and is fast.
- Negative: no `qwen3.5-0.6b` id, no 0.6B GGUF on disk, and no model was
  downloaded during this experiment (per brief).

## H2 — Streaming works: CONFIRMED

`/v1/chat/completions` with `stream: true` returned SSE `delta` chunks ending
with `[DONE]`. Measured with `qwen3.5-2b`, prompt "Reply with exactly: OK":

| model | TTFT | total | chunks |
| --- | --- | --- | --- |
| `qwen3.5-2b` | 2.099 s | 2.349 s | 25 |
| `inkling-2-mini@q8_0` | 2.112 s | 2.364 s | 24 |
| `qwen3.5-35b-a3b` | 56.123 s | 58.196 s | 25 |

Caveat: with default settings the first ~all chunks were **reasoning deltas**,
not answer text (see new finding below). With `reasoning_effort: "none"`, a
streamed "Count from 1 to 5" produced 10 content chunks in 2.27 s with text
`1\n2\n3\n4\n5`.

## H3 — Tool calling exposed but unreliable: PARTLY FALSIFIED

`tools` + `tool_choice: "auto"` are accepted. With `qwen3.5-2b`:

| case | result |
| --- | --- |
| valid prompt | `finish_reason: tool_calls`, `read_file({"path":"C:/temp/report.txt"})` — well-formed JSON, **not** malformed |
| missing required argument | no tool call, `finish_reason: length` after 4.8 s and 944 reasoning chars (ran out of budget while "thinking") |
| unknown tool requested | no tool call, prose refusal: "The `delete_everything` tool doesn't exist…" (desirable behaviour) |
| instruction to emit Ruby-style hash | no tool call, `finish_reason: length`, no malformed JSON emitted |

Negative result: this model did **not** provide a naturally malformed tool-call
corpus under these prompts; its common failure is to emit *no call* and burn the
token budget on reasoning. Therefore the retry/error-handling tests in M2 must
use a **mock model** with injected malformed tool calls in addition to the real
model. Keep the observed failures as fixtures.

`qwen3.5-35b-a3b` and `inkling-2-mini@q8_0` both produced a correct
`read_file` call with `{"path":"C:/temp/report.txt"}` in 14.35 s and 3.12 s
respectively.

## H4 — Throughput is CPU/bandwidth-bound for the big model: CONFIRMED

`qwen3.5-35b-a3b`: 56 s to first token, 58 s for 24 tokens on this machine —
unusable for an interactive UI or 8-way swarm. `qwen3.5-2b`: ~2 s TTFT. Any
parallel-worker throughput test must use the 2B model; the 35B-A3B model is
kept only as a worst-case latency fixture.

## NEW — Qwen3.5 reasoning deltas and `max_tokens`

The server streams model "thinking" in a `delta.reasoning_content` field
(non-standard but OpenAI-compatible superset). With `max_tokens: 24` or `256`
the entire budget was consumed by reasoning and `delta.content` stayed empty;
`finish_reason: length`. Probes at `max_tokens: 64`:

| request variant | result |
| --- | --- |
| default | `length`, content empty, 280 reasoning chars |
| `chat_template_kwargs: {"enable_thinking": false}` | **no effect** — `length`, content empty, 280 reasoning chars |
| `reasoning_effort: "none"` | `stop`, content `OK`, 0 reasoning chars, 2.12 s |
| `max_tokens: 1024` | `stop`, content `OK`, 888 reasoning chars, 4.66 s |

Implications for Legion: the model layer must (a) expose reasoning tokens
separately from answer tokens so the HUD can show a "thinking" state, and
(b) support `reasoning_effort: "none"` (or a large token budget) to guarantee a
visible answer. `chat_template_kwargs.enable_thinking` cannot be relied on.

## Keep/drop decisions

- **KEEP** `qwen3.5-2b` as the M1+ development/test model (closest available
  small Qwen). Record the 0.6B deviation in docs and README notes.
- **DROP** `qwen3.5-35b-a3b` for interactive UI and swarm throughput work; keep
  as a latency/error fixture only.
- **KEEP** a deterministic fake/mock model with injected malformed tool calls
  for M2 error-handling tests; the real 2B model alone does not produce them.
- **KEEP** reasoning/answer delta separation in the model interface, and
  `reasoning_effort: "none"` support, as a required adapter capability.
- **DROP** reliance on `chat_template_kwargs.enable_thinking` (no effect here).

Decisions logged in `ARCHITECTURE_LOG.md` as D007–D009 (provisional, since the
stack is locked only after the study verdict).
