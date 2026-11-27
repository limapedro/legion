# exp_101 — Environment, terminal and local-model capability probe

Status: hypothesis (written before the experiment)

## Questions

1. What host and console capabilities does this target machine expose (Windows
   11 build, shell/host under which this agent runs, VT processing, colour
   depth, Unicode/box-drawing, mouse/resize support detection)?
2. Which models are actually served by LM Studio at `http://localhost:1234/v1`,
   and does the closest small Qwen support streaming and tool calling?
3. Which toolchains are installed, for fair benchmark selection?

## Hypotheses

- H1: The machine is Windows 11 Pro (build 26300) with an i5-12400F (6C/12T),
  32 GB RAM and a GTX 1660 Super, matching the brief.
- H2: Rust, Python, Node and .NET toolchains are installed; no C/C++ compiler
  is on PATH, so C++ is a weak candidate for prototyping here.
- H3: LM Studio does not serve `qwen3.5-0.6b`; the closest small Qwen is
  `qwen3.5-2b`, roughly 3.3x the parameters and therefore slower and more
  capable, but still small enough to exercise malformed tool calls.
- H4: The agent shell is PowerShell inside a Windows Terminal-like host that
  supports VT sequences; plain `cmd.exe` behaviour must be inferred from
  documented Windows APIs rather than measured interactively here.
- H5: `qwen3.5-2b` accepts OpenAI-compatible `stream: true` SSE responses and
  an array of `tools` definitions (function calling), because LM Studio 1.x
  exposes both for Qwen3.5 GGUF models. This must be verified, not assumed.

## Method

- Read-only WMI/CIM queries and .NET console API calls from PowerShell.
- `Invoke-RestMethod` / raw HTTP for `/v1/models`, `/v1/chat/completions`.
- Keep all requests tiny (max a few hundred tokens); no downloads.

## Pass criteria

Exact model ids recorded; a clear yes/no on streaming and tool calls for the
chosen model; host capabilities recorded with the exact commands used.
