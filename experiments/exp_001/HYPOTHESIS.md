# HYPOTHESIS — exp_001: LM Studio test model and failure modes

**Written before the experiment.**

## Context

Legion's development/test model is supposed to be **Qwen3.5 0.6B**, served by
LM Studio's OpenAI-compatible API at `http://localhost:1234/v1`. A first
environment check during bootstrap returned these model ids:

`ternary-bonsai-2-27b`, `inkling-2-mini-uncensored`,
`thinkingmachines/inkling-2-mini`, `inkling-2-mini@q8_0`,
`inkling-2-mini@iq3_xxs`, `qwen3.5-35b-a3b`, two `qwen3.8-27b` variants, and
`text-embedding-nomic-embed-text-v1.5`.

No `qwen3.5-0.6b` id was present. Per the brief, if the exact id differs we use
the closest small Qwen and note the difference.

## Hypotheses

1. **H1 — Model availability.** Qwen3.5 0.6B is not currently loaded in this LM
   Studio server; the closest available Qwen is `qwen3.5-35b-a3b` (a MoE model
   with roughly 3B active parameters). Confirm via `/v1/models` and note that it
   is much larger than the intended 0.6B in memory footprint.
2. **H2 — Streaming works.** `/v1/chat/completions` with `stream: true` returns
   an SSE stream of `delta` chunks and a final `[DONE]`, so streaming UI can be
   built against LM Studio now.
3. **H3 — Tool calling is exposed but unreliable.** The endpoint accepts an
   OpenAI-style `tools` array, but a small Qwen will often produce malformed or
   semantically wrong tool calls, and will sometimes answer in prose instead.
   This gives us deterministic material for retry/graceful-failure tests.
4. **H4 — Throughput is CPU-bound on this machine.** The larger MoE Qwen will
   be slow to first token and generate few tokens/second with the 6 GB GPU, while
   a genuinely small model (`inkling-2-mini` variant) will be much faster. The
   swarm tests therefore need a small model to be meaningful.

## Method

- Query `GET /v1/models`, record exact ids and timestamps.
- Send one short chat completion with `stream: true` and measure time-to-first-
  token, total time, and chunk count.
- Send one tool-calling request with a simple `read_file` tool definition and
  record the exact raw arguments returned, including failures.
- Repeat with at least two model ids. Keep prompts tiny; cap generation with
  `max_tokens`; never download models.
- Record every failure verbatim in `FINDINGS.md`, including HTTP errors and
  timeouts.

## Keep/drop criteria

- Keep a model id for M1+ only if it responds with a usable SSE stream.
- Keep the tool-call failure corpus if it contains at least one malformed call
  worth testing the retry layer against; otherwise find another model or a
  mock-based failure injection.
