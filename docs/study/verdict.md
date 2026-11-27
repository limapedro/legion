# Legion study verdict

Status: **verdict for the lead engineer's review**. This document deliberately
does not edit `ARCHITECTURE_LOG.md`; locking D004 and any new decisions is the
lead engineer's job.

## Executive verdict

| Question | Recommendation |
| --- | --- |
| Language | **Rust 1.99+** (stable, MSVC target on Windows). Alternatives evaluated below. |
| TUI | **Thin custom renderer**: `crossterm` only for console I/O (VT enable, raw mode, mouse/resize/events, alt screen); our own double-buffered cell grid, diff-only redraw and typed capability/fallback layer. Do **not** start with ratatui/cursive/Spectre/Textual. |
| Concurrency | **One process, tokio async tasks** for orchestrator + workers (`CancellationToken` hierarchy, bounded mpsc channels, semaphore of 3 workers / 2 model requests). Shell commands and untrusted tools run as **child processes** for isolation. |
| Model API | **OpenAI-compatible Chat Completions client behind a typed `ModelProvider` trait**: SSE streaming, typed `Delta` stream, JSON-schema + domain validation of tool arguments, 2–3 retries with jitter, explicit cancel, token accounting feeding the same counter used for compaction and the HUD `ctx` readout. Dev endpoint: LM Studio `http://localhost:1234/v1`, model `qwen3.5-2b`. |
| Config | One typed settings file/struct (workers, budgets, timeouts, palette, reduced motion, provider URLs). No sprawling CLI flags. |

Rationale in one sentence: on this exact machine Rust starts in ~4.9 ms with
4.3 MB idle RSS and a 6 µs small-frame redraw, crossterm costs essentially no
binary weight, and the two real harnesses we inspected both keep agents in
one process and isolate only tools — so Rust + tokio + a custom cell renderer
is the lowest-footprint design that still gives clean cancellation and a
single owner of the Windows console.

## 1. Evidence base

- `docs/study/harness-fundamentals.md` — source-level review of opencode
  (`anomalyco/opencode@5d1f788`), aider (`Aider-AI/aider@5dc9490`) and Codex CLI
  (`openai/codex@045a11f`), plus Anthropic/OpenAI tool-use docs.
- `docs/study/swarm-design.md` — two architecture options compared; typed
  task/message schema sketch.
- `docs/study/terminal-ui-windows.md` — VT/ANSI, host differences, library
  survey, rendering and fallback plan.
- `experiments/exp_101/` — machine, toolchain, host and LM Studio inventory
  (all measured).
- `experiments/exp_102/` — startup, memory, idle CPU and 60 fps redraw
  benchmarks (all measured).
- `experiments/exp_103/` — weak-model tool-call reliability, stream latency,
  cancellation (all measured).

Shallow clones were made only in `C:\Users\multi\AppData\Local\Temp\opencode\`
and removed afterwards; the repo worktree only contains notes and small probe
scripts. No `src/` implementation was added.

## 2. Measured benchmarks (exp_102)

Machine: Windows 11 Pro 10.0.26300, i5-12400F (6C/12T), 32 GiB, GTX 1660
Super (unused). Full commands, raw outputs and negative results are in
`experiments/exp_102/FINDINGS.md`.

### Startup (median of launches, lower is better)

| Program | Runs | Median | Min |
| --- | --- | --- | --- |
| Rust std `hello.exe` (LTO, stripped) | 60 | **4.875 ms** | 4.191 |
| Rust + crossterm probe | 40 | **4.770 ms** | 4.410 |
| `cmd /c exit 0` (Windows process-creation floor) | 30 | 10.004 ms | 8.814 |
| Python 3.10 `python -c "pass"` | 60 | 63.668 ms | 60.148 |
| .NET 10 console apphost | 40 | 53.887 ms | 48.664 |

The ~4.2 ms floor is `CreateProcessW`, not language startup; crossterm linking
adds <0.2 ms. Python is ~13× and .NET ~11× slower to launch.

### Idle memory and CPU (sleeping process, 5 s sample)

| Program | RSS | Private bytes | CPU |
| --- | --- | --- | --- |
| Rust | **4.31 MB** | 0.65 MB | 0 % of one core |
| Python | 12.11 MB | 7.00 MB | 0 % |
| .NET | 23.47 MB | 6.62 MB | 0 % |

All three park cleanly; memory is the differentiator for an always-on HUD.

### Redraw cost (120×50 grid, 6 000 cells, diff + escape encode)

| Diff/frame | Rust | C#/.NET | Python | Bytes/frame |
| --- | --- | --- | --- | --- |
| 3 cells | **0.0062 ms** | 0.0622 ms | 0.4229 ms | ~114 |
| 30 cells | **0.0119 ms** | 0.0644 ms | 0.3749 ms | ~952 |
| 6 000 cells (full repaint) | 0.1942 ms | 0.0830 ms | 1.7237 ms | ~67 538 |

Honest caveat: C#'s `StringBuilder` beat the Rust `write!`-macro encoder on
full repaint (0.083 vs 0.194 ms) — a formatting-style artefact, not a language
limit. Rust wins decisively on the common small-diff case and on startup/RSS.
A 3-cell diff costs ~7 KB/s of escape output at 60 fps; idle frames should be
zero.

### Binary weight

Rust std binary 109–150 KB; Rust + crossterm **112.5 KB**; .NET apphost
158.5 KB plus a 23.5 MB shared framework; Python needs the interpreter.
Permissive licences throughout (Rust crates MIT; .NET/Python already present).

### Not measured (with reproducible commands for later)

- **Real terminal rendering / flicker / colour fidelity**: this agent's stdout
  is redirected and no interactive console is attached (exp_101 measured the
  `CONOUT$` mode, not pixels). To measure later, run inside Windows Terminal
  and plain conhost:
  `bench.exe 600 30 1 > out/frames.bin` and capture with a high-frame-rate
  screen recording; compare against `ESC[?2026h/l` disabled.
- **GPU impact**: no GPU rendering is planned; the 1660 Super was unused.
- **Multi-process swarm overhead**: not run as a full harness; the relevant
  measured proxy is process launch (~4.9 ms each) plus the RSS numbers above.
  Reproduce with `launch.exe 100 -- <agent-binary>` and `probe_idle.ps1`.

## 3. Language evaluation (fair comparison on Windows)

| Language | Startup/RSS (measured) | Windows TUI story | Verdict |
| --- | --- | --- | --- |
| **Rust** | 4.9 ms / 4.3 MB | crossterm supports console host + Windows Terminal, mouse, resize, alt screen; MIT; tiny binary; Codex CLI itself uses ratatui | **Chosen** |
| C# / .NET 10 | 53.9 ms / 23.5 MB | Spectre.Console (MIT) is excellent for rich output but line-oriented; Terminal.Gui is a full widget framework; framework dependency | Strong second; rejected for always-resident footprint and slower start |
| Python 3.10 | 63.7 ms / 12.1 MB | rich/textual beautiful but interpreter + GC; `curses` needs extra package on Windows | Rejected for shipping; fine for probe scripts |
| C++ | not measured | ftxui (MIT) viable, but `g++`/`clang++` are **not installed** on this machine (exp_101) | Rejected: no toolchain |
| Go | not measured | `go` is **not installed**; tcell/tview viable elsewhere | Rejected: no toolchain |

## 4. TUI approach

- **Use crossterm for I/O only**, plus Win32 calls (`GetConsoleMode`,
  `SetConsoleMode`, `GetConsoleScreenBufferInfo`, `CONOUT$`) for capability
  detection and code-page handling. exp_101 showed VT already enabled in
  Windows Terminal but **ambient code page IBM437**, so either call
  `SetConsoleOutputCP(65001)` or use `WriteConsoleW`.
- **Own cell grid**: `Cell { glyph, fg, bg, flags }` (≈16–24 bytes); 120×50 ≈
  96–144 KB per buffer. Widgets write to the back buffer; diff against front;
  emit CUP + grouped SGR only for changed runs; wrap frames in
  `ESC[?2026h … ESC[?2026l` when supported.
- **Glyphs**: Unicode box-drawing, Braille (2×4 dots/cell) and half-blocks for
  glow/waveform; DEC line-drawing (`ESC(0`) or ASCII fallback when Unicode is
  unavailable. Colour tiers: TrueColor → 256 → 16 → none (`NO_COLOR`).
- **Fallbacks** are a typed `TerminalCaps` struct and a rendering policy,
  not scattered conditionals: no colour, no Unicode, reduced motion, narrow
  terminal, no VT/redirected output (line log). Always restore alt screen,
  cursor and console mode on exit.
- **Frame pacing**: 30 fps ambient / 60 fps during active animation; no frames
  when idle. Animation phases derive from `Instant`, so dropped frames are
  seek-safe. CPU target ≤2 % of one core while animating, ≤1 ms diff+encode.

Rationale for *not* adopting ratatui: its layout/widget engine is useful, but
a glow-heavy HUD needs direct control of per-cell style and Braille glyphs;
starting with a widget framework adds retained state we would fight. Revisit
ratatui only if standard panels become the bulk of the UI.

## 5. Concurrency model

- **Design A (chosen):** orchestrator + workers as tokio tasks in one process.
  Bounded mpsc (64), `CancellationToken` per request → per task, semaphore of
  **3 workers** and **2 model requests** on this 6C/12T machine, typed
  task/message schema from `swarm-design.md`. Shell commands/tools are child
  processes (the isolation boundary). Shared graph/artifacts via a single
  actor; private conversation per worker; UI reads an event projection only.
- **Design B (rejected for baseline):** one OS process per agent. Stronger
  crash isolation, but high complexity (wire protocol, supervisor, orphan
  cleanup, cross-process cancel), ~60–160 MB inferred RSS for 4 agents, and
  each process would fight over the single Windows console/mouse. Real
  harnesses (opencode child sessions/jobs, Codex threads) keep agents in one
  process and isolate tools instead.
- **Cancellation:** hierarchical tokens, `select!` at every await,
  `AbortOnDropHandle` for runaway tasks; exp_103 measured stream close at
  **0.0001 s**, so ESC can abort immediately and emit an explicit event.

## 6. Model-API layer and LM Studio finding

### Exact model ids (measured, exp_101)

GET `http://localhost:1234/v1/models` returned exactly:

1. `qwen3.5-2b`
2. `ternary-bonsai-2-27b`
3. `inkling-2-mini-uncensored`
4. `thinkingmachines/inkling-2-mini`
5. `inkling-2-mini@q8_0`
6. `inkling-2-mini@iq3_xxs`
7. `qwen3.5-35b-a3b`
8. `unsloth/qwen3.8-27b-gguf/mtp-qwen3.8-27b-q4_0.gguf`
9. `unsloth/qwen3.8-27b-gguf/qwen3.8-27b-q4_0.gguf`
10. `text-embedding-nomic-embed-text-v1.5`

**The intended `qwen3.5-0.6b` is not served.** The closest small Qwen is
`qwen3.5-2b`, which we used for all model probes. Differences: ~3.3× the
parameter count (2B vs 0.6B) — measurably slower (first streamed delta
**2.124 s**, full tool-call turn 2.8–3.8 s) and likely more capable, but still
a small local model suitable for exercising failure paths. Exact architecture,
quantisation and context-window size of these GGUFs were **not measured** and
are therefore inferred/unknown; do not assume 0.6B-class speed or limits.
Other served Qwen models (35B-A3B, 27B Qwen3.8) are far too large for the dev
loop. **No models were downloaded.**

### Capabilities (measured)

- **Streaming: YES** — `stream: true` returns `text/event-stream` with
  `chat.completion.chunk` SSE frames.
- **Tool calling: YES** — returned
  `finish_reason: tool_calls` with a JSON-string `arguments` value and a call
  id. Legion must parse/validate that string and echo results with the same
  id.
- **Caveats (exp_103):** `tool_choice` with an object-shaped forced function
  produced HTTP 400; `"required"` was accepted but ignored. An under-specified
  prompt produced `finish_reason: length`, empty content and **no tool call**;
  a numeric argument was coerced to `"42"`. Treat these as first-class retry
  states and validate both schema and domain types.

### API layer recommendation

A `ModelProvider` trait with one OpenAI-compatible implementation: typed
request/response structs; incremental SSE parser producing
`Text | ToolCallDelta{id,name,args_fragment} | Usage | Done`; cancellation by
dropping the stream + token; retries (2–3, jitter) on 429/5xx/timeouts;
tool-argument validation with error reflected to the model; token accounting
shared with compaction and HUD. Add Anthropic provider later by mapping its
`tool_use`/`tool_result` blocks to the same internal types; do not leak either
wire format into the harness.

## 7. Suggested build order (for the lead engineer, not locked)

1. M1: Rust workspace with typed config, tokio orchestrator skeleton, JSONL
   session log, `ModelProvider` + LM Studio provider; plain line-mode output.
2. M2: crossterm capability detection + cell grid/diff renderer; alt screen,
   fallbacks, 30/60 fps pacing; wire the HUD to the event projection.
3. M3: tool registry + permission policy + approval chip; retries and
   malformed-call reflection (exp_103 cases as fixtures).
4. M4: task graph, 3-worker semaphore, sub-agent depth cap 2, UI DAG view.
5. M5: compaction/tail-keeping, token budget readout, resume/replay.

## 8. Risks and open questions

- **Windows process-tree kill** for child tools: validate `JobObject` usage
  before relying on it (not measured here).
- **LM Studio concurrency:** two simultaneous requests may serialise inside
  the server; keep the model semaphore configurable (default 2).
- **Model mismatch:** dev model is 2B, not 0.6B; re-run exp_103 if/when 0.6B
  is loaded. Do not download it as part of this study.
- **Real terminal verification:** conhost vs Windows Terminal mouse/resize,
  colour fidelity and synchronized-output support were documented, not
  measured. Manual smoke test required before M2 (see §2 commands).
- **Console code page:** IBM437 measured in this host; must set UTF-8 or use
  `WriteConsoleW`, or Unicode glyphs will be corrupted in plain `cmd.exe`.
- **C# full-repaint anomaly:** `StringBuilder` beat Rust's formatter; if full
  repaints ever dominate, replace `write!` with a small integer encoder.

## 9. Files written in this study

- `docs/study/harness-fundamentals.md`
- `docs/study/swarm-design.md`
- `docs/study/terminal-ui-windows.md`
- `docs/study/verdict.md` (this file)
- `experiments/exp_101/{HYPOTHESIS,FINDINGS}.md`, `probe_host.ps1`, `probe_model.py`
- `experiments/exp_102/{HYPOTHESIS,FINDINGS}.md`, `probe_idle.ps1`,
  `rust/`, `rust_crossterm/`, `python/bench.py`, `csharp/`
- `experiments/exp_103/{HYPOTHESIS,FINDINGS}.md`, `probe_reliability.py`
- `.gitignore` (added `experiments/**/bin/`, `experiments/**/obj/`)

`ARCHITECTURE_LOG.md` was intentionally left untouched.
