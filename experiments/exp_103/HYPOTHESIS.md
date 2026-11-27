# exp_103 — Weak local model: tool-call reliability, streaming and cancellation

Status: hypothesis (written before the experiment)

## Questions

Using the actually-served `qwen3.5-2b` (exp_101) as a stand-in for the intended
`qwen3.5-0.6b`:

1. How often does a deliberately under-specified prompt produce malformed or
   missing tool arguments?
2. Can the model be forced to call a tool and return JSON that fails a typed
   schema (e.g. wrong type, missing required field)?
3. How quickly does the first streamed delta arrive (HUD responsiveness)?
4. What happens when the client cancels a stream mid-arguments — is the close
   clean and fast?

## Hypotheses

- H1: Under-specified prompts will sometimes produce missing/placeholder
  arguments; validation errors must be reflected back to the model.
- H2: `tool_choice` can force calls; the raw `arguments` string may violate the
  schema, so Legion must validate before execution.
- H3: First delta arrives well under 2 s for a 2B local model on this machine.
- H4: Closing the HTTP stream mid-generation returns promptly (< 1 s) and does
  not throw an unhandled error; a 50 ms ESC-style UI timeout is unrelated.
- H5: No request should take > 30 s; if one does, record it and stop.

## Method

Small Python script using `urllib.request` against
`http://localhost:1234/v1/chat/completions`, `max_tokens` ≤ 128, 2–4 prompts,
one forced call, one streaming latency measurement, one mid-stream cancel.
No downloads; no model changes.

## Pass criteria

Concrete examples (redacted to short strings) of good/bad tool calls, measured
first-token latency, and cancel latency, plus a recommendation for the error-
recovery path in the harness.
