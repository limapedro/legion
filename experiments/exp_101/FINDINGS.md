# exp_101 — Findings

Date: 2026-11-27. All commands were run from the worktree
`E:\__PROJECTS__\Legion-study` on the target machine.

## Environment (measured)

Command:

```powershell
Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors
Get-CimInstance Win32_ComputerSystem | Select-Object TotalPhysicalMemory
Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM,DriverVersion
Get-ComputerInfo -Property OsName,OsVersion,OsBuildNumber | Format-List
```

- CPU: `12th Gen Intel(R) Core(TM) i5-12400F`, 6 cores / 12 logical processors.
- RAM: `34359738368` bytes = 32.00 GiB (`Win32_PhysicalMemory` sum).
- GPU: `NVIDIA GeForce GTX 1660 SUPER` (WMI `AdapterRAM` was blank in this
  session; the 6 GB figure comes from the brief, not from a measurement here).
- OS: `Microsoft Windows 11 Pro`, build `10.0.26300`.

Toolchains:

```powershell
foreach ($c in 'rustc','cargo','python','py','node','dotnet','g++','clang++','go','git','gh','curl') { Get-Command $c -ErrorAction SilentlyContinue }
```

- `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0`
- `Python 3.10.11`, `node v24.13.1`, `.NET 10.0.401`, `git`, `gh`
- `g++`, `clang++`, `go`: MISSING on PATH (C++/Go prototypes would need a
  toolchain install first; treated as a negative for rapid experimentation).

## Console / host (measured, with caveats)

Script: `experiments/exp_101/probe_host.ps1` (read-only P/Invoke of
`GetConsoleMode`, `GetConsoleScreenBufferInfo`, `CreateFile("CONOUT$")`).

```text
HOST_NAME            = ConsoleHost
PS_VERSION           = 5.1.26100.10068
WT_SESSION           = 5bd7b61f-5a63-45df-9aa6-fb59a2e7193d
TERM_PROGRAM         =
CONEMU               =
ANSICON              =
SESSIONNAME          = Console
ComSpec              = C:\Windows\system32\cmd.exe
OUTPUT_MODE(std)     = FAILED
INPUT_MODE(std)      = FAILED
CONOUT_HANDLE        = 2560
CONOUT_MODE          = 0x0007
BUFFER_WxH           = 120x3000
WINDOW_WxH           = 120x50
CODEPAGE_OUTPUT      = IBM437
CODEPAGE_INPUT       = IBM437
RAM_BYTES            = 34359738368
```

Interpretation (measured vs inferred):

- **Measured:** a `WT_SESSION` GUID is present, so this agent runs inside
  Windows Terminal. `CONOUT$` reports mode `0x0007` =
  `ENABLE_PROCESSED_OUTPUT | ENABLE_WRAP_AT_EOL_OUTPUT |
  ENABLE_VIRTUAL_TERMINAL_PROCESSING` — VT processing is already enabled in
  this host. Window size measured as 120 columns x 50 rows; buffer height 3000.
- **Measured:** the agent's stdout/stderr handles are *redirected*
  (`GetConsoleMode` on `GetStdHandle(-11/-10)` fails), hence the `CONOUT$`
  workaround. This means rendered frames from this session cannot be visually
  verified; capability detection is by API/env, not by eyeballing pixels.
- **Measured:** console code pages are still IBM437 (OEM), not UTF-8. A native
  TUI must either emit UTF-8 bytes and call `SetConsoleOutputCP(65001)`, or use
  the UTF-16 `WriteConsoleW` API; box-drawing/Braille output cannot rely on the
  ambient code page. This is a concrete Windows-specific gotcha.
- **Not measured here:** legacy conhost behaviour, mouse event delivery,
  resize event delivery, and rendering inside plain `cmd.exe`. These are
  documented from Microsoft APIs in `terminal-ui-windows.md` and marked as
  inferred. No interactive keyboard/escape-sequence ambiguity test was
  possible from a non-console stdout.

## LM Studio model inventory (measured)

Command: `python experiments/exp_101/probe_model.py qwen3.5-2b`
(which GETs `http://localhost:1234/v1/models`). Exact ids returned, in order:

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

**The intended `qwen3.5-0.6b` is absent.** Closest small Qwen is
`qwen3.5-2b`: about 3.3x the parameters of 0.6B, so noticeably slower and more
capable, but still a small local model and therefore still useful for
exercising malformed tool calls. The other Qwen models served (35B-A3B, 27B
Qwen3.8) are far too large/slow for a calm HUD development loop. **Nothing was
downloaded.**

## Model capabilities (measured on `qwen3.5-2b`)

- **Streaming: YES.** Request with `"stream": true` returned HTTP
  `Content-Type: text/event-stream` and consecutive
  `data: {"object":"chat.completion.chunk", ...}` lines.
- **Tool calling: YES.** Request with a `tools` array containing
  `get_weather(city)` and `"tool_choice": "auto"` returned
  `finish_reason: tool_calls` and
  `[{"type":"function","id":"Qp5e9lXVBOA09GtQoD9tjVhI987LxqjH",
  "function":{"name":"get_weather","arguments":"{\"city\":\"Porto\"}"}}]`
  with empty `content`. Arguments arrived as a JSON *string*, as in the
  OpenAI protocol — the harness must parse and validate it.

## Outcome

H1, H2, H3, H4 and H5 all supported. Caveat: H4's plain-`cmd.exe` half is
documented, not measured. All probes were read-only; no changes to the machine
were made. Next: exp_103 will stress malformed calls/retries on this model.
