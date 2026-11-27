```
██╗     ███████╗███████╗███╗   ██╗██╗ ██████╗
██║     ██╔════╝██╔════╝████╗  ██║██║██╔═══██╗
██║     █████╗  █████╗  ██╔██╗ ██║██║██║   ██║
██║     ██╔══╝  ██╔══╝  ██║╚██╗██║██║██║   ██║
███████╗███████╗███████╗██║ ╚████║██║╚██████╔╝
╚══════╝╚══════╝╚══════╝╚═╝  ╚═══╝╚═╝ ╚═════╝
```

**An ambient intelligence for your command line.** Legion is a native Windows
terminal harness for LLM agents — a calm, glanceable head-up display where a
team of agents works on your behalf, and you stay in control.

```
┌─ LEGION ────────────────────────────────── ╭─────────╮ ─┐
│ ◉ ONLINE    ctx 12.4k    model qwen3.5    │  WORKERS │  │
│                                           │  3 / 8   │  │
│  › analyse the repo and report findings   │ ╰─────────╯  │
│    ├─ worker-01  reading files       ▸▸▸                │
│    ├─ worker-02  running tests      ▸▸                  │
│    └─ worker-03  thinking           ▒▒▒▒                │
│  ──────────────────────────────────────────────────────  │
│  › ask the swarm anything…                         ↵    │
└─────────────────────────────────────────────────────────┘
```

## The idea

Most AI tools ask you to leave your workspace and talk to a chat window.
Legion flips that around: the assistant lives where you already work. It sits
in the terminal, quietly aware of your project, and responds with a compact
display you can read at a glance — status, progress, and what each agent is
doing, without a browser or a heavyweight GUI in the way.

When a task is bigger than one conversation, Legion doesn't pretend one agent
can do it all. It forms a **swarm**: a small orchestrator breaks the work into a
task graph, workers pick up pieces, and the display shows who is doing what in
real time. The interface is designed to stay out of your way — low CPU, low
memory, no fan noise, and readable whether you are in a 80-column console or a
wide window.

The long-term vision is voice-first: the same display becomes an ambient
surface you can speak to. The waveform widget is already planned as part of the
UI research, with mocked input until voice is real.

## What it is

- A Windows command-line harness: runs in classic `cmd.exe` and Windows Terminal.
- Native rendering with terminal escape sequences. No Electron, no browser, no
  GUI toolkit.
- Model-agnostic behind an interface. Development uses a small local model via
  LM Studio's OpenAI-compatible server.
- A research project: the UI and swarm behaviour are treated as experiments,
  with written hypotheses, findings and decisions.

## What it is not (yet)

- A chat wrapper with a pretty prompt.
- A cloud service. Everything locally testable stays local.
- A finished product — see the status and roadmap below.

> **Status: M0 — study phase.** The repository and docs are in place; the stack
> is not locked yet. Nothing is runnable until M1. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Design constraints

- Target machine: Windows, i5-12400F, 32 GB RAM, GTX 1660 Super (6 GB, no Tensor
  Cores). CPU, memory and GPU use must stay low; the UI must never make the
  machine feel busy.
- Must work on plain `cmd.exe`. Features that only work in Windows Terminal are
  detected and degrade gracefully.
- Development/test model: a small Qwen served by LM Studio's OpenAI-compatible
  local API (`http://localhost:1234/v1`). The model layer is behind an interface
  so Legion is not tied to LM Studio; other OpenAI-compatible endpoints follow.
- A deliberately weak small model is used to exercise malformed tool calls,
  retries and graceful failure.

## Repository layout

```
README.md              this file
ARCHITECTURE_LOG.md    every locked decision, its alternatives and rationale
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
- [Architecture](docs/ARCHITECTURE.md) — written after the study verdict
- [UI notes](docs/UI_NOTES.md) — terminal/HUD research notes and experiment index
- [Study reports](docs/study/) — harness fundamentals, swarm designs, Windows TUI, verdict

## License

MIT — see [LICENSE](LICENSE).
