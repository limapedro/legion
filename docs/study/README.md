# Study phase reports

The study sub-agent's deliverables. These are **reports for review**, not
locked decisions; locking happens in `ARCHITECTURE_LOG.md` after the lead
engineer and user review the verdict.

1. [Harness fundamentals](harness-fundamentals.md) — agent loop, tool-call
   formats, streaming, context management, permissions, sessions, error
   recovery, sub-agents and parallelism in open-source harnesses (opencode,
   aider, Codex CLI).
2. [Swarm design](swarm-design.md) — two compared architectures (async tasks
   in one process vs one process per agent), typed task graph/message schema,
   context sharing, cancellation, back-pressure, UI representation.
3. [Terminal UI on Windows](terminal-ui-windows.md) — VT/ANSI, alternate
   screen, colour models, Unicode/Braille, input/resize, host differences,
   TUI libraries vs a custom renderer, double buffering and fallbacks.
4. [Verdict](verdict.md) — recommended language, TUI approach, concurrency
   model, model-API layer, measured benchmarks and the LM Studio model finding.

Experiments are numbered from `exp_101` so they do not collide with the main
line's `exp_001` onward:

- `experiments/exp_101/` — machine, console and LM Studio capability probe.
- `experiments/exp_102/` — Rust / Python / .NET startup, RSS, idle CPU and
  60 fps redraw benchmarks.
- `experiments/exp_103/` — weak-model tool-call reliability, stream latency
  and cancellation.

Every experiment folder has `HYPOTHESIS.md` written before the work and
`FINDINGS.md` written afterwards, including negative results.
