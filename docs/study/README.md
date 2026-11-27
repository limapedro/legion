# Study phase reports

Reserved for the study sub-agent launched before real code is written. It will
report back here with:

1. `harness-fundamentals.md` — agent loop, tool-call formats, streaming,
   context management, permissions, sessions, error recovery, sub-agents and
   parallelism in open-source harnesses (opencode, aider, Claude-Code-style CLIs).
2. `swarm-design.md` — at least two compared architectures (async tasks in one
   process vs one process per agent), task graph, message passing, context
   sharing, cancellation, back-pressure, UI representation.
3. `terminal-ui-windows.md` — VT/ANSI, alternate screen, color models, Unicode,
   braille/half-blocks, input/resize, host differences, TUI libraries vs custom
   renderer, double buffering and diff-only redraw strategy.
4. `verdict.md` — recommended language, TUI approach, concurrency model,
   model-API layer, and benchmarks (startup, idle CPU, memory, 60 fps redraw).

The verdict is reviewed by the user before the stack is locked in
`ARCHITECTURE_LOG.md`.
