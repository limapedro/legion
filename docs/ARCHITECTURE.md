# Architecture

Current as of M1. Decisions and their rationale live in
[`ARCHITECTURE_LOG.md`](../ARCHITECTURE_LOG.md); measured evidence lives in
`docs/study/` and `experiments/`.

## Stack (provisional until explicit user sign-off on the study verdict)

- **Rust 1.99+, tokio.** Chosen from measured startup (4.9 ms), idle RSS
  (4.31 MB) and 3-cell redraw cost (0.0062 ms) on the target machine.
- **crossterm for console I/O only** (planned for M3). The renderer will be a
  thin custom cell grid with diff-only redraws, not a widget framework.
- **One process.** Orchestrator and workers are tokio tasks; tools run as child
  processes. Cancellation uses a hierarchical `CancellationToken`.
- **Typed config** (`legion.toml` → `Settings`), no sprawling CLI flags.
- **Model-agnostic** via `ModelProvider`, with an OpenAI-compatible streaming
  implementation and a deterministic mock.

## Module map

```
src/main.rs        entry point, REPL, plain line-mode output
src/config.rs      typed Settings, loaded from legion.toml, defaults documented
src/model.rs       ChatRequest, Message, ToolSpec, Delta, ModelProvider trait
src/sse.rs         incremental SSE decoder (unit tested with split chunks)
src/providers/
  openai.rs        LM Studio / OpenAI-compatible Chat Completions + SSE
  mock.rs          deterministic scripted stream for tests
src/agent.rs       agent loop: deltas -> text + tool calls, bounded history
```

## Data flow (M1)

```
stdin line
  -> Message::user pushed to bounded history
  -> ChatRequest (model, messages, max_tokens, reasoning_effort)
  -> ModelProvider::stream_chat(request, cancel)
  -> SSE frames -> Delta::{Text, Reasoning, ToolCall, Usage, Error}
  -> on_delta callback prints streaming text
  -> TurnOutcome { text, tool_calls, token usage }
  -> assistant message appended to history
```

Tool calls are parsed and assembled in M1 but not executed; M2 attaches the
tool registry and approval flow to the same `TurnOutcome`.

## Model interface notes

- `reasoning_effort: "none"` is the default because Qwen3.5 otherwise consumes
  its whole token budget on `reasoning_content` and emits no answer text
  (`experiments/exp_001/FINDINGS.md`). Reasoning deltas remain a first-class
  `Delta` variant for the HUD's "thinking" state.
- The LM Studio wire format never leaks above `providers/openai.rs`. Adding an
  Anthropic provider later means mapping its blocks to the same `Delta` types.
- Cancellation is passed explicitly into the provider; dropping the stream is
  not the only stop path.

## Planned evolution

- **M2:** tool registry, validation (schema + domain), approval policy,
  `Connector` trait, retries and malformed-call fixtures (see
  [`ASSISTANT.md`](ASSISTANT.md)).
- **M3:** crossterm capability detection (`TerminalCaps`), cell grid, diff
  renderer, HUD panels, no-colour/no-Unicode/reduced-motion fallbacks.
- **M4/M5:** task graph, tokio worker tasks, bounded channels, orchestrator,
  swarm panel.
- **M6:** JSONL session persistence, themes, single-exe release.

## Rendering budget

30 fps ambient / 60 fps during animation, no frames while idle, ≤1 ms for a
diff + escape encode, ≤2 % of one core while animating. Front/back buffers of
a 120×50 grid cost roughly 100–150 KB each.
