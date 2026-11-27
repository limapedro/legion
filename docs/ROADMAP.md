# Legion roadmap

Milestones are sequential. A milestone is complete only when its docs are
updated, its decisions are logged, and its PR is merged.

## M0 — Repo, docs, study verdict, stack locked

- [x] GitHub repo `legion` created (private), cloned, `main` bootstrapped
- [x] Initial structure: `README.md`, `docs/`, `ARCHITECTURE_LOG.md`,
      `experiments/`, `src/`, `.gitignore`, `LICENSE`
- [x] First PR: `chore: repo skeleton and docs`
- [x] Roadmap written
- [x] Study sub-agent launched in a worktree/branch, reporting into `docs/study/`
- [x] Study verdict delivered (`docs/study/verdict.md`, PR #3) with measured
      benchmarks and the LM Studio model finding
- [ ] Language, TUI approach, concurrency model and model-API layer locked in
      `ARCHITECTURE_LOG.md` after explicit user review. Rust + crossterm-I/O +
      tokio + typed OpenAI-compatible provider is the measured recommendation
      and is being implemented provisionally on `feat/m1-harness-core`.

## M1 — Minimal harness

- [x] Agent loop (request → streamed deltas → text/tool-call outcome). Tool
      execution arrives in M2; the loop is provider-agnostic.
- [x] Model interface with LM Studio OpenAI-compatible implementation
- [x] `/v1/models` checked in exp_001/exp_101; Qwen3.5 0.6B absent, closest
      small Qwen `qwen3.5-2b` selected and documented. Runtime discovery not
      implemented yet — default model lives in typed config.
- [x] Streaming output (real SSE against LM Studio, verified end-to-end)
- [x] Plain UI in `cmd.exe` and Windows Terminal (line mode; HUD arrives in M3)
- [x] Fake/mock model for deterministic tests
- [x] Tests for the agent loop (8 passing, including cancellation and split SSE)

## M2 — Tools and connectors (assistant capabilities)

Vision and catalogue: [ASSISTANT.md](ASSISTANT.md).

- [ ] Tool registry with typed schemas and result types
- [ ] Filesystem tools: read_file, write_file, list_dir, search_files
- [ ] run_command as a supervised child process
- [ ] web_search and fetch_url
- [ ] Assistant tools: time, system_status, notes, shared tasks, clipboard, notify
- [ ] `Connector` trait and `connector_call` dispatch; optional connectors
- [ ] Approval/permission flow (auto-allow, prompt, deny, session allow, cancel)
- [ ] Malformed tool call handling, schema + domain validation, retries
- [ ] Mock-provider failure fixtures and real-model smoke test with `qwen3.5-2b`
- [ ] Tests for the tool layer and tool-calling loop

## M3 — HUD UI v1

- [ ] Chosen layout (evidence from layout experiments)
- [ ] Status bar, task progress, streaming/typing animation, spinners
- [ ] Animations: progress rings, palette, boot sequence, subtle idle motion
- [ ] Fallbacks: no-color, no-Unicode, reduced-motion
- [ ] Renderer snapshot/golden test if feasible

## M4 — Sub-agents

- [ ] Spawn, track and cancel a worker from the main agent
- [ ] Worker state visible in the UI

## M5 — Swarm

- [ ] Parallel workers, orchestrator, shared task list
- [ ] Swarm panel readable at 8+ agents
- [ ] Throughput measured against LM Studio with multiple workers

## M6 — Polish

- [ ] Session persistence
- [ ] Themes and typed configuration
- [ ] Voice connectors: speech-to-text and text-to-speech behind traits
- [ ] Home Assistant / calendar / webhook connectors
- [ ] Single-exe release or installer
- [ ] Benchmarks published in README
- [ ] Final docs pass
