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

- Status: locked
- Decision: the repository is named `legion` and is **private**. The binary,
  repo and docs all use the name Legion (it is a name, not a codename).
- Alternatives considered: a codename during development; a public repo.
- Why: the user specified the name and private-by-default. Making it public
  requires an explicit decision from the user and has not been requested.
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

- Status: **provisional** — Rust is the default until the study sub-agent
  reports; this decision is locked only after the user reviews the verdict.
- Decision: pending.
- Alternatives considered: Rust, C++, C#, Go, Python.
- Why: the study phase exists to choose based on evidence on the target machine.
- Evidence: `docs/study/` (pending), experiment findings (pending).
