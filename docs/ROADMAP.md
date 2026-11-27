# Legion roadmap

Milestones are sequential. A milestone is complete only when its docs are
updated, its decisions are logged, and its PR is merged.

## M0 — Repo, docs, study verdict, stack locked

- [x] GitHub repo `legion` created (private), cloned, `main` bootstrapped
- [x] Initial structure: `README.md`, `docs/`, `ARCHITECTURE_LOG.md`,
      `experiments/`, `src/`, `.gitignore`, `LICENSE`
- [x] First PR: `chore: repo skeleton and docs`
- [x] Roadmap written
- [ ] Study sub-agent launched in a worktree/branch, reporting into `docs/study/`
- [ ] Study verdict reviewed by the user; language, TUI approach, concurrency
      model and model-API layer locked in `ARCHITECTURE_LOG.md`

## M1 — Minimal harness

- [ ] Agent loop (prompt → model call → tool call → result → loop)
- [ ] Model interface with LM Studio OpenAI-compatible implementation
- [ ] `/v1/models` discovery, Qwen3.5 0.6B preferred with documented fallback
- [ ] Streaming output
- [ ] Plain UI in `cmd.exe` and Windows Terminal
- [ ] Fake/mock model for deterministic tests
- [ ] Tests for the agent loop

## M2 — Tools

- [ ] read file
- [ ] write file
- [ ] run command
- [ ] web search
- [ ] Approval/permission flow
- [ ] Malformed tool call handling, retries and graceful failure with the weak model
- [ ] Tests for the tool layer

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
- [ ] Single-exe release or installer
- [ ] Benchmarks published in README
- [ ] Final docs pass
