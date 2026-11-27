# Legion

Legion is an LLM agent harness for the Windows command line. It runs natively in
`cmd.exe` and Windows Terminal — no Electron, no browser, no GUI toolkit — and
supports **agent swarms**: multiple workers cooperating on a shared task graph,
observed through a JARVIS-style heads-up display drawn with terminal escape
sequences.

> **Status: M0 — study phase.** The repository is being set up and the stack is
> not locked yet. Nothing here is runnable until M1. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Design constraints

- Target machine: Windows, i5-12400F, 32 GB RAM, GTX 1660 Super (6 GB, no Tensor
  Cores). CPU, memory and GPU use must stay low; the UI must never make the
  machine feel busy.
- Must work on plain `cmd.exe`. Features that only work in Windows Terminal are
  detected and degrade gracefully.
- Development/test model: a small Qwen served by LM Studio's OpenAI-compatible
  local API (`http://localhost:1234/v1`). The model layer sits behind an
  interface so Legion is not tied to LM Studio.
- The weak small model is a feature: malformed tool calls, retries and graceful
  failure are tested with it.

## Repository layout

```
README.md              this file
ARCHITECTURE_LOG.md    every locked decision, its alternatives and rationale
HYPOTHESIS.md          (inside each experiment) written before experimenting
experiments/           numbered exp_NNN/ folders, each with FINDINGS.md
docs/                  README, architecture, UI notes, roadmap, study/
src/                   implementation (language chosen after the study phase)
```

## Method

Every experiment gets a numbered folder with `HYPOTHESIS.md` written *before*
the work and `FINDINGS.md` written afterwards — negative results included.
Locked decisions go in `ARCHITECTURE_LOG.md`. Configuration lives in typed
structs/files, not a sprawling set of CLI flags. Work happens on branches with
a PR for each meaningful unit; `main` stays buildable.

## Documentation

- [Roadmap](docs/ROADMAP.md) — milestones M0–M6 and current status
- [Docs index](docs/README.md)
- [Architecture](docs/ARCHITECTURE.md) — filled in after the study sub-agent verdict
- [UI notes](docs/UI_NOTES.md) — terminal/JARVIS HUD research notes
- [Study reports](docs/study/) — harness fundamentals, swarm designs, TUI on Windows, verdict

## License

MIT — see [LICENSE](LICENSE).
