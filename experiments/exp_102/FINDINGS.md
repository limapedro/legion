# exp_102 — Findings: prototype benchmarks

Date: 2026-11-27. Machine: Windows 11 Pro 10.0.26300, i5-12400F (6C/12T),
32 GiB, GTX 1660 Super (unused). Source: `experiments/exp_102/`.
`CARGO_TARGET_DIR` was set to the worktree's `experiments/exp_102/rust/target`
so C: (initially full) was not used for build artifacts.

## Commands (exact)

```powershell
$env:CARGO_TARGET_DIR='E:\__PROJECTS__\Legion-study\experiments\exp_102\rust\target'
cargo build --release --manifest-path 'E:\__PROJECTS__\Legion-study\experiments\exp_102\rust\Cargo.toml' --offline

$r='E:\__PROJECTS__\Legion-study\experiments\exp_102\rust\target\release'
& "$r\launch.exe" 60 -- "$r\hello.exe"
& "$r\launch.exe" 60 -- python -c "pass"
& "$r\launch.exe" 30 -- cmd /c exit 0

& "$r\bench.exe" 600 3 0
& "$r\bench.exe" 600 30 0
& "$r\bench.exe" 600 6000 0
& "$r\bench.exe" 600 3 1 > 'E:\__PROJECTS__\Legion-study\experiments\exp_102\out\rust-write.bin'
python experiments\exp_102\python\bench.py bench 300 3 0
python experiments\exp_102\python\bench.py bench 300 30 0
python experiments\exp_102\python\bench.py bench 300 6000 0

& 'E:\__PROJECTS__\Legion-study\experiments\exp_102\probe_idle.ps1' -FilePath "$r\idle.exe" -Arguments '12' -Seconds 5
& 'E:\__PROJECTS__\Legion-study\experiments\exp_102\probe_idle.ps1' -FilePath 'python.exe' -Arguments '-c "import time; time.sleep(12)"' -Seconds 5

dotnet new console --no-restore --output experiments\exp_102\csharp --force --name LegionBench
dotnet build experiments\exp_102\csharp\LegionBench.csproj -c Release --nologo
$exe='...\csharp\bin\Release\net10.0\LegionBench.exe'
& "$r\launch.exe" 40 -- $exe
& $exe bench 300 3 0
& $exe bench 300 30 0
& $exe bench 300 6000 0
& '...\probe_idle.ps1' -FilePath $exe -Arguments 'idle 12' -Seconds 5

# crossterm dependency weight
cargo build --release --manifest-path experiments\exp_102\rust_crossterm\Cargo.toml
& "$r\launch.exe" 40 -- '...\rust_crossterm\target\release\legion-crossterm-probe.exe'
```

The prototype renders a 120×50 cell grid (6 000 cells), double-buffered, with
a per-frame cell diff, CUP cursor moves, grouped SGR style changes and a
`ESC[?2026h … ESC[?2026l` synchronized-output wrapper. `changes` is the number
of cells perturbed per frame; 6 000 means a full repaint.

## Startup (median of N launches, lower is better)

| Program | Runs | Median | Min | p90 |
| --- | --- | --- | --- | --- |
| Rust `hello.exe` (std, LTO, stripped) | 60 | **4.875 ms** | 4.191 | 5.342 |
| Rust + crossterm probe | 40 | **4.770 ms** | 4.410 | 5.218 |
| `cmd /c exit 0` (control for Windows process-creation floor) | 30 | 10.004 ms | 8.814 | 10.986 |
| Python `python -c "pass"` | 60 | **63.668 ms** | 60.148 | 70.727 |
| .NET 10 console apphost (`LegionBench.exe`, framework-dependent) | 40 | **53.887 ms** | 48.664 | 61.060 |

The ~4.2 ms floor for a trivial Rust process is mostly `CreateProcessW`, not
Rust initialisation: crossterm linking changed it by <0.2 ms. Python is ~13×
and .NET ~11× slower to start. (A previous run using a Python `subprocess`
harness gave the same ordering: Rust 4.75 ms, Python 62.6 ms, cmd 9.3 ms.)

## Idle CPU and memory (sleeping process, sampled after 0.8 s settle)

| Program | RSS (WorkingSet64) | Private bytes | CPU over 5 s wall |
| --- | --- | --- | --- |
| Rust `idle.exe` | **4.31 MB** | 0.65 MB | 0.0000 s → **0 %** of one core |
| Python `time.sleep(12)` | **12.11 MB** | 7.00 MB | 0.0000 s → **0 %** |
| .NET 10 console apphost | **23.47 MB** | 6.62 MB | 0.0000 s → **0 %** |

H3 confirmed: a parked process uses no measurable CPU in all three. Rust uses
~3× less RSS than Python and ~5× less than .NET for a do-nothing process.

## Diff + escape encode cost (mean per frame)

| Diff size | Rust (std) | C# / .NET 10 | Python 3.10 | Output bytes/frame |
| --- | --- | --- | --- | --- |
| 3 cells (typical spinner/status change) | **0.0062 ms** | 0.0622 ms | 0.4229 ms | ~114 / ~74 / ~75 |
| 30 cells (busy swarm) | **0.0119 ms** | 0.0644 ms | 0.3749 ms | ~952 / ~565 / ~566 |
| 6 000 cells (full repaint/resize) | 0.1942 ms | **0.0830 ms** | 1.7237 ms | ~67 538 / ~49 095 / ~49 095 |

Notes:

- Rust small diffs are ~10× faster than C# and ~68× faster than Python.
- **Negative/surprising result:** for a full repaint C#'s `StringBuilder` was
  ~2.3× *faster* than Rust's `write!`-macro implementation (0.083 vs 0.194 ms).
  This is a micro-benchmark artefact of formatting style, not a language limit;
  an optimised integer encoder would close the gap. Recorded honestly.
- Writing the frame to a redirected file added ~0.0006 ms for a 3-cell diff
  (0.0062 → 0.0068 ms) — negligible at small diffs.
- Even Python's full repaint (1.72 ms) fits in a 16.7 ms 60 fps budget, so the
  deciding factors are startup, memory and whole-harness complexity, not raw
  diff speed.
- At 60 fps with small diffs the Rust renderer emits only ~114 bytes/frame
  (~7 KB/s); most frames should therefore be no-ops when idle.

## Binary sizes

- Rust std-only `hello.exe`: 109 KB; `bench.exe`: 150.5 KB; `idle.exe`: 117 KB.
- Rust + crossterm (windows/events features, stripped, LTO): **112.5 KB** —
  crossterm adds negligible binary weight here.
- .NET apphost: 158.5 KB, but requires the shared framework (23.5 MB RSS) —
  a self-contained publish would be tens of MB.
- Python: interpreter already on the machine but 12 MB RSS and 64 ms startup.

## What was NOT measured / negative results

- **Terminal rendering itself** was not measured: stdout is redirected in this
  agent session and no interactive console is attached. All timings are
  diff+encode (+ file write) only. A real-TTY measurement needs a manual run
  in Windows Terminal and conhost; reproducible command:
  `bench.exe 600 <changes> 1` inside an interactive window, with a capture
  card or high-frame-rate recording (out of scope here).
- The first C# build failed with `CS0136` because top-level programs already
  define `args`; fixed by renaming to `argv`. Recorded as a negative result.
- `probe_idle.ps1` rejects an empty `-Arguments` value, so an idle sample of
  the short-lived crossterm probe could not be taken; only its startup and
  binary size are reported.
- LM Studio concurrency/throughput was not benchmarked; only API capability
  was checked in exp_101.

## Verdict from this experiment

Rust is the clear winner for a calm, always-resident HUD: ~4.9 ms startup,
4.3 MB idle RSS, ~6 µs small redraw, tiny binary, crossterm essentially free.
Python is acceptable for scripting/probes but too heavy to sit in the
background next to the swarm. .NET's redraw is respectable but startup/RSS are
10×/5× Rust's.
