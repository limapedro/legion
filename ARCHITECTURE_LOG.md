# Architecture log

Every **locked** decision is recorded here with its alternatives and why. A
decision is locked only after the stated evidence exists; provisional choices
are marked as such and must be replaced with a locked entry before the
milestone that depends on them.

Format:

```
## DNNN — Short decision title  (date, milestone)
- Status: locked | provisional | superseded by DNNN
- Decision:
- Alternatives considered:
- Why:
- Evidence: link to experiment / study report / finding
```

---

## D001 — Repository name and visibility (2026-11-27, M0)

- Status: superseded by D005
- Decision: the repository is named `legion` and was initially **private**. The
  binary, repo and docs all use the name Legion (it is a name, not a codename).
- Alternatives considered: a codename during development; a public repo.
- Why: the user specified the name and private-by-default for the bootstrap.
- Evidence: user instructions, section 0 rule 2 and section 1.

## D002 — License (2026-11-27, M0)

- Status: locked
- Decision: MIT License, copyright 2026 Pedro Lima Silva.
- Alternatives considered: Apache-2.0, GPL-3.0, AGPL-3.0, proprietary.
- Why: user chose MIT when asked; permissive and simple for a CLI tool.
- Evidence: user answer during bootstrap.

## D003 — Methodology and branch flow (2026-11-27, M0)

- Status: locked
- Decision:
  - `HYPOTHESIS.md` before each experiment, in `experiments/exp_NNN/`;
    `FINDINGS.md` after, including negative results.
  - All decisions recorded in this file; docs kept current under `docs/`.
  - Never push to `main`; branches (`feat/`, `exp/`, `docs/`, `chore/`) plus a PR
    for every meaningful unit of work.
  - Typed config structs/files instead of sprawling CLI flags.
- Alternatives considered: ad-hoc scripts without notes; direct commits to
  `main`; long-lived unmerged branches; flag-heavy CLI configuration.
- Why: experiments must be reproducible and reviewable; `main` stays buildable.
- Evidence: user instructions, section 0.

## D004 — Language / TUI stack (M0)

- Status: **provisional** — recommendation adopted for implementation, still
  awaiting the user's explicit review of the verdict before being locked.
- Decision: build in Rust 1.99+ with a thin custom renderer (crossterm for console
  I/O only), tokio async tasks in one process for orchestrator/workers, and a
  typed `ModelProvider` trait with an OpenAI-compatible streaming client.
- Alternatives considered: C#/.NET 10 (53.9 ms startup, 23.47 MB RSS), Python
  (63.7 ms, 12.11 MB), C++ and Go (no toolchain installed), ratatui/widget
  frameworks (too much retained state for a glow-heavy HUD), process-per-agent
  (heavier, cross-process cancellation and console ownership problems).
- Why: measured on this machine Rust starts in ~4.9 ms with 4.31 MB idle RSS and
  a 3-cell diff+encode of 0.0062 ms; crossterm adds <0.2 ms startup and keeps
  a 112.5 KB binary. One process gives clean hierarchical cancellation and a
  single owner of the Windows console; tools remain child processes.
- Evidence: `docs/study/verdict.md`, `experiments/exp_102/FINDINGS.md`,
  `experiments/exp_103/FINDINGS.md`.

## D005 — Repository visibility: public (2026-11-27, M0)

- Status: locked. Supersedes D001.
- Decision: `limapedro/legion` is a **public** repository.
- Alternatives considered: keeping it private (the bootstrap default).
- Why: the user explicitly asked to make it public after reviewing the skeleton.
- Evidence: user instruction following PR #1; `gh repo view` reports `PUBLIC`.

## D006 — Documentation tone (2026-11-27, M0)

- Status: locked
- Decision: describe the interface by the idea behind it — an ambient
  intelligence, a calm head-up display in the terminal, a voice-first future —
  without direct references to Iron Man or its fictional assistant. Use neutral
  terms such as "radial progress rings" instead of franchise-specific names.
- Alternatives considered: keeping the original "JARVIS-style HUD" and
  "arc-reactor" wording in docs and README.
- Why: the user asked for a nice README that talks about the concept rather
  than the reference.
- Evidence: user instruction after PR #1; README and `docs/UI_NOTES.md` edits.

## D007 — Dev/test model: `qwen3.5-2b` (2026-11-27, M0/M1, provisional)

- Status: provisional — confirmed by experiment, but the provider/stack choice
  is locked only after the study verdict.
- Decision: use `qwen3.5-2b` (2B, 1.94 GB) from LM Studio as the development and
  test model. The requested Qwen3.5 0.6B is not downloaded or served; the 2B
  model is the closest small Qwen available.
- Alternatives considered: `qwen3.5-0.6b` (absent), `qwen3.5-35b-a3b` (56 s
  TTFT — far too slow), `inkling-2-mini@q8_0` (not Qwen), downloading a 0.6B
  model (out of scope; no downloads).
- Why: ~2 s TTFT and valid tool calls; small enough for parallel workers.
- Evidence: `experiments/exp_001/FINDINGS.md` (H1, H4), `/v1/models`, `lms ls`.

## D008 — Mock model with injected failures (2026-11-27, M1/M2, provisional)

- Status: provisional
- Decision: keep a fake/mock model implementation and inject malformed tool
  calls, wrong argument types and timeouts in tests; the real 2B model does not
  naturally emit malformed JSON under the tested prompts.
- Alternatives considered: relying on the real model to fail usefully; using
  only the 35B-A3B model as a failure fixture.
- Why: deterministic failure modes are needed for retry/graceful-failure tests.
- Evidence: `experiments/exp_001/FINDINGS.md` (H3).

## D009 — Reasoning/answer delta separation in the model interface (2026-11-27, M1, provisional)

- Status: provisional
- Decision: the model interface distinguishes reasoning deltas from answer
  deltas and supports `reasoning_effort: "none"` (or an adequate token budget)
  so the HUD can show a "thinking" state and still guarantee visible answers.
- Alternatives considered: treating all deltas as answer text (produces an
  empty UI); relying on `chat_template_kwargs.enable_thinking` (no effect here).
- Why: Qwen3.5 streams `delta.reasoning_content` and can consume the whole
  `max_tokens` budget before emitting `delta.content`.
- Evidence: `experiments/exp_001/FINDINGS.md` (new finding), probe results.

## D010 — Product scope: ambient assistant with tools and connectors (2026-11-27, M0)

- Status: locked
- Decision: Legion's goal is a local, voice-first virtual assistant, not only a
  coding-agent harness. It needs a broad typed tool catalogue (files, shell,
  web, notes, tasks, system info, clipboard, notifications), a `Connector`
  trait for external systems (speech, smart home, calendar, webhooks), and
  mandatory tool-calling tests with both mock-injected failures and the real
  LM Studio model.
- Alternatives considered: coding-agent-only scope; tool calling postponed
  until after the HUD; trusting the model to validate its own arguments.
- Why: the user explicitly clarified the goal; a useful assistant is defined
  by what it can do, and tool calls are where weak models fail.
- Evidence: user instruction; `docs/ASSISTANT.md`; updated `docs/ROADMAP.md`.
