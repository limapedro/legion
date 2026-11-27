# exp_102 — Prototype benchmarks: startup, memory, idle CPU, redraw cost

Status: hypothesis (written before the experiment)

## Questions

For small prototypes on the target machine (i5-12400F 6C/12T, 32 GiB, Windows
11 Pro), how do the leading candidate language stacks compare on:

1. cold-ish process startup (median of repeated launches),
2. resident memory of a sleeping process,
3. idle CPU while parked,
4. cost of a double-buffered cell-grid diff + escape-sequence encode for a
   120×50 screen at 60 fps, with small and large diffs?

Candidates: Rust (std-only custom renderer; crossterm I/O where relevant),
Python 3.10 (stdlib-only equivalent, plus `rich` if installed), and .NET 10
(plain console baseline if buildable offline).

## Hypotheses

- H1: Rust startup is < 5 ms; Python ~25–60 ms; .NET ~40–100 ms.
- H2: Rust RSS < 15 MB, Python RSS 10–25 MB, .NET RSS 30–80 MB.
- H3: Idle CPU for all three is ~0 once parked; none should spin.
- H4: Rust diff+encode of a 120×50 grid is < 0.3 ms for a 3-cell change and
  < 1.5 ms for a full repaint; Python is 10–30× slower; .NET sits between.
- H5: Writing escape sequences to a redirected file adds measurable but small
  cost; terminal rendering itself cannot be measured from this session
  (stdout redirected, no interactive console), so only encode+write is
  reported and rendering is flagged as unmeasured.

## Method

- Build Rust prototypes in `experiments/exp_102/rust/` with
  `CARGO_TARGET_DIR` on E: (C: was full during setup).
- Use one Python timing harness (`bench.py`) with `subprocess`/`time.perf_counter`
  for startup (report median and min, and note harness overhead floor).
- Use PowerShell `Get-Process` for RSS and CPU deltas of sleeping processes.
- Run each benchmark ≥ 10 times; report median and range, exact commands.

## Negative-result policy

If .NET restore fails offline, record the failure and report Python vs Rust
only. If C: fills again, stop cloning/downloading and use std-only code.
