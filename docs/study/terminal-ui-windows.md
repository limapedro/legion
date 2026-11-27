# Terminal UI on Windows — research and rendering plan

Status: study report for review, not a locked decision.

Evidence labels: **[measured]** on this machine (see `experiments/exp_101/`),
**[source]** Microsoft docs / library docs read 2026-11-27, **[inferred]**
reasoned from those, not run here.

## 1. What we actually measured on this machine

From `experiments/exp_101/probe_host.ps1` and its `FINDINGS.md`:

```text
HOST_NAME            = ConsoleHost            (PowerShell 5.1.26100.10068)
WT_SESSION           = 5bd7b61f-5a63-45df-9aa6-fb59a2e7193d
CONOUT_MODE          = 0x0007                 (PROCESSED_OUTPUT | WRAP_AT_EOL | VIRTUAL_TERMINAL_PROCESSING)
WINDOW_WxH           = 120x50
BUFFER_WxH           = 120x3000
CODEPAGE_OUTPUT      = IBM437
CODEPAGE_INPUT       = IBM437
```

- **[measured]** This agent runs inside Windows Terminal; `CONOUT$` already has
  VT output processing enabled. `GetStdHandle(-11/-10)` failed because the
  harness's stdio is redirected — capability detection must open `CONOUT$`
  `CreateFileW` or use the console APIs on a real attached console.
- **[measured]** The ambient console code page is OEM 437. UTF-8 box-drawing
  and Braille bytes will be mis-decoded unless the app calls
  `SetConsoleOutputCP(65001)` or writes UTF-16 through `WriteConsoleW`.
- **[measured]** The shell environment contains `WT_SESSION` and `ComSpec=
  C:\Windows\system32\cmd.exe`.
- **[not verified]** Actual pixels, colour fidelity, mouse event delivery,
  resize events, legacy conhost behaviour, and escape-key ambiguity: no
  interactive console was available (stdout redirected). These sections are
  marked as inferred/documented, and exp_104-style manual checks are proposed
  in `verdict.md` as open questions.

## 2. VT / ANSI essentials

Authoritative references:

- <https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences>
- <https://learn.microsoft.com/en-us/windows/console/setconsolemode>
- Synchronized output spec: <https://github.com/contour-terminal/vt-extensions/blob/master/synchronized-output.md>

**[source]** The documented way to enable VT on a Windows console:

```c
HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
DWORD mode = 0;
GetConsoleMode(hOut, &mode);
mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;   // 0x0004 (also keep PROCESSED_OUTPUT 0x0001)
SetConsoleMode(hOut, mode);                  // try DISABLE_NEWLINE_AUTO_RETURN 0x0008, then fall back
```

Do not blindly overwrite the mode: read-modify-write, and if `SetConsoleMode`
fails with `ERROR_INVALID_PARAMETER`, retry with only
`ENABLE_VIRTUAL_TERMINAL_PROCESSING`, then fall back to `WriteConsoleOutputW`
or plain text (**[source]** Microsoft's own "Enabling Virtual Terminal
Processing" sample).

Useful sequences (all **[source]** from the MS page):

| Purpose | Sequence |
| --- | --- |
| Cursor position (1-based row;col) | `ESC [ <row> ; <col> H` (CUP) |
| Cursor up/down/forward/back | `ESC [ n A/B/C/D` |
| Hide / show cursor | `ESC [ ?25l` / `ESC [ ?25h` |
| Erase display / line | `ESC [ 2J` / `ESC [ 2K` |
| SGR colour | `ESC [ 38;2;r;g;b m`, `ESC [ 48;2;r;g;b m` (RGB); `ESC [ 38;5;n m` (256) |
| Alternate screen on/off | `ESC [ ?1049h` / `ESC [ ?1049l` |
| Scroll region | `ESC [ top ; bottom r` (per-buffer; safe with alt screen) |
| Begin/end synchronized output | `ESC [ ?2026h` / `ESC [ ?2026l` (BSU/ESU) |
| DEC line drawing | `ESC ( 0` / `ESC ( B` |
| Device queries | `ESC [ 6n` (CPR), `ESC [ 0c` (DA) |

**Truecolor vs 256 vs 16:** **[source]** the MS docs state that classic
Windows Console maps extended colours (RGB/256) to the nearest entry in its
16-colour table — i.e. no real truecolor in legacy conhost. Windows Terminal
renders RGB natively. **[inferred]** Legion should treat colour as a
capability: `TrueColor > Indexed256 > Basic16 > NoColor`, selected once at
startup (env `NO_COLOR`/`COLORTERM` + OS build + terminal identity), and
convert a small internal palette down at the output boundary rather than
scattering conditionals through widgets.

**Unicode / box-drawing:** **[source]** DEC line drawing gives ASCII-safe box
characters (`q`→`─`, `x`→`│`, `l`→`┌`, …) when UTF-8 cannot be assumed.
**[inferred]** Unicode box-drawing, Braille (`U+2800`–`U+28FF`, 2×4 dots per
cell) and half-blocks (`▀▄█`) give sub-cell resolution for glow/spinner/waveform
widgets; use them only in `Unicode` mode, with ASCII/DEC fallbacks. Braille is
especially cheap: an 8-dot bitmap per cell means a 120×50 screen can hold a
240×200 "pixel" canvas with only 6 000 cells, and the character can be built
by bit arithmetic (`0x2800 | dots`) — no font/GPU work.

**Cursor control:** **[source]** CUP is bounded to the viewport; wrap at EOL
is governed by `ENABLE_WRAP_AT_EOL_OUTPUT` and `DISABLE_NEWLINE_AUTO_RETURN`.
For a full-screen HUD, hide the cursor, disable wrapping on the final column,
redraw with CUP, and restore the cursor/mode on exit.

**Mouse input:** **[source]** Two paths:

1. `ENABLE_MOUSE_INPUT` (0x0010) + `ReadConsoleInput` → `MOUSE_EVENT_RECORD`.
   **[source]** `ReadFile`/`ReadConsole` *discard* mouse events even when the
   flag is set — must use `ReadConsoleInput`.
2. VT input (`ENABLE_VIRTUAL_TERMINAL_INPUT` 0x0200) → SGR 1006 mouse reports
   `ESC [ <btn>;<x>;<y> M/m` on stdin (modern terminal convention;
   **[inferred]** supported by Windows Terminal; conhost support is limited).

Prefer VT SGR 1006 when available; fall back to `ReadConsoleInput` on classic
conhost. Quick-edit mode (`ENABLE_QUICK_EDIT_MODE | ENABLE_EXTENDED_FLAGS`)
must be disabled while the HUD owns the screen, or mouse selection fights the
UI.

**Resize events:** **[source]** `ENABLE_WINDOW_INPUT` (0x0008) →
`WINDOW_BUFFER_SIZE_RECORD` via `ReadConsoleInput`. VT-only apps get no
resize notification in conhost; **[inferred]** Windows Terminal may deliver
`SIGWINCH`-like behaviour to some runtimes, but the robust approach on Windows
is to poll `GetConsoleScreenBufferInfo` (cheap, once per frame or on a timer)
and compare `(srWindow.Right-Left+1, Bottom-Top+1)` — which we already did in
exp_101.

**Synchronized output:** **[source]** `ESC [?2026h` / `ESC [?2026l` wraps a
frame so the terminal renders once. **[inferred]** Emit it only if the host is
known to support it (Windows Terminal does; older conhost may print the raw
sequence). Gate behind capability detection, not by default.

**Keyboard / escape ambiguity:** **[source]** With VT input, arrows are
`ESC [ A/B/C/D` (or `ESC O A` in application cursor mode), F1–F12 and Home/End
are `ESC [ n ~`/`ESC O P`, Alt prefixes a character with `ESC`, and Ctrl maps
to control bytes (Ctrl+`[` is itself `ESC` = 0x1b). Therefore a bare ESC byte
is ambiguous: it may be the Escape key or the first byte of a sequence.
**[inferred]** use a small state machine with a ~50 ms timeout after a lone
ESC (`ESC` alone → Escape key; more bytes → parse CSI/SS3; timeout → Escape).
For text entry (the `› ask the swarm anything…` box) raw mode should be off
until the prompt is focused, so line editing and IME keep working.

## 3. Host differences and graceful degradation

| Host | VT out | RGB | Unicode | Mouse | Resize | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Classic conhost (legacy `cmd.exe`) | Yes since Win10 Anniversary, if enabled | **[source]** rounded to 16-colour palette | Depends on font/code page | Win32 `MOUSE_EVENT_RECORD`; SGR 1006 limited | `WINDOW_BUFFER_SIZE_RECORD` | Scrollback buffer exists; alt screen hides it |
| `cmd.exe` inside Windows Terminal | Yes (WT_SESSION) | **[source]** native 24-bit | Yes (Cascadia/font dependent) | VT SGR 1006 works well | VT/poll works | Best target |
| PowerShell 5.1 console host | Same console APIs as conhost; ANSI enabled by default only in newer builds | as host | as host | as host | as host | **[measured]** our host is PowerShell 5.1 inside Windows Terminal |
| PowerShell 7 / pwsh | VT on modern Windows; `$PSStyle` may already emit ANSI | native in WT | yes | as host | as host | **[inferred]** |
| VS Code integrated terminal | `WT_SESSION` can be inherited → false positive | yes | yes | VT | VT | **[source]** known false positive: <https://github.com/Textualize/rich/issues/140>, <https://mikefrobbins.com/2024/05/16/detecting-windows-terminal-with-powershell/> |

**Detection plan (typed capability struct, computed at startup):**

1. `GetConsoleMode(CONOUT$)` → VT flag. If unavailable (redirected output),
   treat as non-interactive: print plain lines, no control sequences.
2. If VT available, query with `ESC [0c` / `ESC [6n` with a short timeout and
   parse the reply from stdin; **[inferred]** conhost answers `ESC[?1;0c`
   ("VT101 with no options") so don't over-trust it. Use OS build ≥ 10.0.15063
   + `WT_SESSION` + `COLORTERM` as hints, not proof.
3. Capabilities stored as `TerminalCaps { color: ColorMode, glyphs: GlyphMode,
   mouse: MouseMode, sync_output: bool, alternate_screen: bool,
   reduced_motion: bool }`. `reduced_motion` from `NO_COLOR`/OS accessibility
   setting/env `LEGION_REDUCED_MOTION`; typed config, not CLI flags.
4. Degrade by *rendering policy*, not by code forks: one cell grid, different
   glyph/style mappers.

**What could not be verified:** no interactive console was attached to this
agent, so conhost vs Windows Terminal differences, real mouse/resize delivery,
and colour rendering are documented from Microsoft sources, not measured.
A manual smoke test (open `cmd.exe` in conhost via `conhost.exe cmd.exe`, run
the probe binary, then repeat in Windows Terminal) is recommended before M2.

## 4. Library survey

| Library | Language / licence | Weight & Windows specifics | Verdict for Legion |
| --- | --- | --- | --- |
| [ratatui](https://ratatui.rs/) + [crossterm](https://github.com/crossterm-rs/crossterm) | Rust, MIT + MIT | Pure Rust, few deps; crossterm can drop `mio`/`signal-hook` with `filedescriptor`; supports Win 7+, console host + Windows Terminal, mouse, resize, alt screen, RGB | **Use crossterm for I/O.** Ratatui's widget/layout engine is useful, but its immediate-mode buffer + diff is the part we want |
| [cursive](https://github.com/gyscos/Cursive) | Rust, MIT | Retained-mode widget tree, heavier deps (ncurses backend on Windows) | Not needed; fights custom glow rendering |
| [termion](https://github.com/redox-os/termion) | Rust, MIT | Unix-only | Reject (Windows target) |
| [Spectre.Console](https://github.com/spectreconsole/spectre.console) | C#, MIT | Rich markup/tables, auto colour downgrade; .NET runtime required; line-oriented, not a cell-buffer renderer | Good for reports, not a 60 fps HUD |
| [Terminal.Gui](https://github.com/gui.cs/Terminal.Gui) | C#, MIT | Full widget toolkit, large API/state; .NET runtime | Too heavy for a calm HUD |
| [rich](https://github.com/Textualize/rich) / [textual](https://github.com/Textualize/textual) | Python, MIT | Beautiful output, but interpreter startup + GC; Textual is an app framework with its own event loop | Prototyping only; not the shipping renderer |
| `curses` | Python stdlib (PSF licence) | Windows needs `windows-curses` package; 16-colour model | Reject |
| [ftxui](https://github.com/ArthurSonzogni/FTXUI) | C++17, MIT | Nice DOM API; needs a C++ toolchain (g++/clang++ **missing** on this target per exp_101) and `fmt` | Reject for now (no compiler) |

**[measured]** toolchain availability: Rust 1.99, Python 3.10, Node 24, .NET
10 SDK; no g++/clang++/go. **[inferred]** Ratatui/Crossterm wins on licence
(permissive), dependency weight, native Windows console support and the fact
that Codex CLI itself is built with ratatui (per <https://ratatui.rs/> showcase).

**Recommendation:** a **thin custom renderer using crossterm only** for
terminal I/O (raw mode, events, alt screen), with our own cell buffer and
diff. Ratatui can be adopted later for standard panels if desired, but the
glow/animation-heavy HUD needs direct control over per-cell style and
Braille/half-block glyphs. Do not take on a widget framework's retained state.

## 5. Rendering strategy and CPU budget

**Double-buffered cell grid.** Two `Vec<Cell>` buffers of `width × height`;
`Cell { glyph: char, fg: Color, bg: Color, modifiers: u8 }` (~16–24 bytes).
120×50 = 6 000 cells ≈ 96–144 KB per buffer — negligible. Widgets write into
the back buffer only. At frame time:

1. Compare back vs front cell-by-cell (cheap integer/char compares).
2. Emit one `CUP` (cursor move) per run of changed cells, group consecutive
   cells on a row into one write, and only emit SGR when style changes.
3. Wrap the whole frame in `ESC[?2026h … ESC[?2026l` when supported.
4. Swap buffers; `flush()` once.

No full-screen clears except on resize/first frame. This is "diff-only
redraw": a spinner changing 3 cells emits ~3 CUPs + a handful of SGRs, not
6 000 cells.

**Frame pacing.** A single UI thread/task drives a `tokio::time::interval`
(or a dedicated thread with `Sleep`). Default target 30 fps for ambient
animation, with 60 fps cap only while actively animating a waveform/spinner;
idle (no changes, no animation) → **no frames at all** (park the timer).
Animation phases are computed from a monotonic `Instant`, not accumulated
per-frame state, so dropping frames never changes timing (seek-safe, like a
paused media timeline).

**Concrete budget on this machine (i5-12400F, measured 6C/12T, 32 GB):**

- Idle: **0 redraws/s**; UI process CPU < 0.2% of one core (target, to be
  measured in exp_102/104); RSS < 15 MB for the renderer process.
- Animating at 60 fps: **≤ 2% of one core** average; diff+encode+write ≤
  1.0 ms per frame for a 120×50 grid; output ≤ ~20 KB/s of escape sequences
  for typical small diffs; no allocation in the steady-state frame path
  (pre-allocated string buffer reused).
- Full resize/repaint: oneoff ≤ 5 ms and ≤ 100 KB output.
- If a frame exceeds 4 ms twice in a row, drop animation quality (Braille →
  half-block → simple glyphs) before dropping frame pacing.

These are **targets derived from design**, not measured numbers; exp_102
measures the actual prototype.

**Fallback plan (concrete):**

| Condition | Behaviour |
| --- | --- |
| `NO_COLOR` set or colour=Basic | 16-colour palette only; no gradients/glow, use bold/inverse for emphasis |
| glyphs=ASCII | Box drawing via `ESC(0`/`ESC(B` or `+ - |`; Braille → `*` spinner; half-blocks → `#`; waveform → ASCII sparkline `▁▂▃▄▅▆▇█`→`:-=+#@` |
| glyphs=Unicode but no RGB | Unicode shapes with 16-colour palette |
| reduced motion | Static frames; spinners hold a glyph; progress shown numerically; no pulse |
| no VT / redirected output | Line-mode log: timestamped `status` lines only, same event source, no escape sequences |
| narrow terminal (< 60 cols) | Single-column stack; hide worker detail and waveform |
| no console handle (piped) | Auto-disable alt screen and cursor moves; plain text |

On shutdown (including panic via a guard), always emit `ESC[?1049l`,
`ESC[?25h`, `ESC[0m` and restore the original console mode; otherwise the
user's prompt is left inverted/cursored.
