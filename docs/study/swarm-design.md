# Legion swarm design — study proposal

Status: study report for review. Nothing is locked in `ARCHITECTURE_LOG.md`.

Evidence: **[source]** from `harness-fundamentals.md` (opencode, aider, Codex
CLI source/docs), **[measured]** from exp_101 (target machine), **[inferred]**
design reasoning. Target machine: i5-12400F 6C/12T, 32 GiB, Windows 11 Pro,
GTX 1660 Super unused — **[measured]**.

## 0. Goals and constraints

- Small orchestrator decomposes work; workers execute; the HUD shows who is
  doing what, continuously, without the machine feeling busy.
- Dev model is a small local Qwen via LM Studio (`qwen3.5-2b`, **[measured]**),
  so prompts must be small and malformed tool calls expected.
- Low CPU/memory; no GPU; calm visuals. UI must never block the swarm.
- Everything resumable from an append-only session log; typed config, no flag
  sprawl.

## 1. Two candidate architectures

### Design A — async tasks in one process (Rust + tokio)

All agents are `tokio::task`s in a single binary. Each agent owns a private
conversation state and a `CancellationToken`; they communicate over bounded
`tokio::sync::mpsc` channels. Shared state (task graph, artifact store, event
log) lives behind a `RwLock`/actor task. External shell commands are spawned
as child processes (as real harnesses do) so a bad command cannot kill the
harness.

- **[source]** closest precedent: opencode subagents = child sessions +
  background jobs in one server process, with cooperative-then-forced cancel
  (`session/subagent-job.ts`, `session/runner/step.ts`); Codex multi-agent v2
  = threads with `CancellationToken`, typed `send_message`/`wait`/`interrupt`
  tools (`codex-rs/core/src/tools/handlers/multi_agents_v2/`).
- **Complexity:** low–medium. One binary, one address space, direct typed
  messages, no serialisation for internal messages. Must be disciplined about
  `Send/Sync` and lock scope.
- **Crash isolation:** weak for in-process code (a panic in a worker task is
  contained by tokio's `JoinHandle` but a process-level fault kills all);
  strong for commands/tools because those remain subprocesses.
- **Footprint:** one process. **[inferred]** ~10–20 MB RSS for a Rust binary
  plus ~1–2 MB stack per task; 4 workers ≈ +8 MB. Far below the 32 GB budget.
- **IPC cost:** near zero — channel send of an owned enum, no kernel round trip.
- **Cancellation:** simple and hierarchical: a parent token cancels children;
  `select!` on token in every await; `AbortOnDropHandle` for runaway tasks
  (**[source]** Codex `tools/parallel.rs` uses exactly this).

### Design B — one OS process per agent

Each agent is a separate executable instance, communicating over named pipes /
TCP loopback / a local broker, with a supervisor process.

- **[source]** precedent: CLI harnesses that run tools in subprocesses and
  sandbox them (Codex `codex-rs/windows-sandbox*`, opencode persistent PTY
  sessions), but *not* one process per reasoning agent — both keep agents in
  one process.
- **Complexity:** high. Need a wire protocol, process lifecycle management,
  log collection, stdout/stderr framing, orphan cleanup, crash restart, and
  cross-process cancellation tokens. Windows process creation is heavier than
  on Unix.
- **Crash isolation:** strong — a worker crash cannot take down the
  orchestrator or other workers.
- **Footprint:** **[inferred]** N process images + N HTTP clients + N model
  connection pools. With 4 workers, ~4 × 15–40 MB = 60–160 MB RSS; still fine
  on 32 GB, but noticeably busier on a 6-core CPU and much more visible in
  Task Manager — contrary to "never feel busy".
- **IPC cost:** kernel pipe/socket round trip per message; negligible in
  absolute terms (<ms) but requires serialisation and back-pressure handling.
- **Cancellation:** awkward — a token cannot be shared across processes; need
  explicit `Cancel` messages plus a kill fallback (and a way to distinguish a
  deliberate cancel from a crash).

### Comparison summary (on this machine)

| Criterion | A: async tasks | B: process per agent |
| --- | --- | --- |
| Complexity | Low | High (protocol + supervisor) |
| Crash isolation | Weak in-process, strong for tools | Strong |
| Memory (4 workers) | ~10–20 MB + ~4–8 MB stacks **[inferred]** | ~60–160 MB **[inferred]** |
| CPU idle | ~0 (parked tasks) | N idle processes (small but nonzero) |
| IPC cost | channel send | pipe/socket + serde |
| Cancellation | native, hierarchical | message + kill fallback |
| Windows specifics | one console attach, one VT setup | each process may fight for console/mouse |
| Resume | trivial (shared state) | need to reconstruct N processes |

**Recommendation: Design A**, with **tool/shell execution in child processes**
as the isolation boundary. Rationale: the real harnesses we inspected choose
in-process agents + out-of-process tools; this is the simplest design that
gives clean cancellation and a single console owner (critical for the HUD),
and its footprint is comfortably within budget on a 6C/12T machine. Use B only
if a worker needs a fundamentally different trust level or runtime (e.g. a
plugin in another language); that is a future extension, not the baseline.

## 2. Task graph representation

The orchestrator owns a DAG. Nodes are typed, not free-form strings.

```rust
type AgentId = u32;
type TaskId  = u32;

struct Task {
    id: TaskId,
    title: String,                // one line for the HUD
    kind: TaskKind,               // Research | Edit | Test | Review | Synthesize
    deps: Vec<TaskId>,            // hard prerequisites (DAG)
    state: TaskState,             // Ready | Assigned(run_id) | Running | Done | Failed | Cancelled
    owner: Option<AgentId>,
    prompt: String,               // private worker prompt, built by orchestrator
    allowed_tools: Vec<ToolId>,   // capability filter, checked at dispatch
    budget: Budget,
    attempt: u32,
    result: Option<ArtifactId>,
}

struct Budget {
    max_model_steps: u32,         // e.g. 8 (opencode agents support `steps`)
    max_output_tokens: u32,
    max_tool_calls: u32,
    deadline: Option<Duration>,
}

struct Edge { from: TaskId, to: TaskId }  // from must finish before to starts
```

The graph is stored in a small actor (`GraphActor`) and persisted as part of
the JSONL session log (`TaskAdded`, `TaskStateChanged`). Workers never mutate
the graph directly; they send `TaskEvent`s. This mirrors opencode's durable
inbox/job model and avoids races.

**Scheduling:** topological ready-set; a worker claims a task with an atomic
`Claim(task_id, agent_id)`; on completion the orchestrator unblocks
dependents. Failed tasks retry up to `budget.max_attempts` with a shortened
prompt containing the validation error (see exp_103 for why). No cycles:
`GraphActor::add_edge` rejects them.

## 3. Message passing and shared vs private context

```rust
struct Envelope {
    seq: u64,                     // per-session monotonic, also UI ordering
    from: AgentId,
    to: MessageTarget,            // Orchestrator | Agent(AgentId) | Broadcast
    task: Option<TaskId>,
    payload: Msg,
}

enum Msg {
    Assign(Task),
    Progress { phase: Phase, note: String },   // Phase::Thinking|Reading|Tool|Writing
    ToolResult { call_id: String, ok: bool, summary: String },
    Result { artifact: ArtifactId, summary: String, tokens_used: u32 },
    NeedApproval { tool: ToolId, resource: String },
    Approval { granted: bool },
    Ask { question: String },                 // worker -> orchestrator/user
    Cancel { reason: String },
    Shutdown,
}

struct Artifact { id: ArtifactId, kind: ArtifactKind, bytes: Vec<u8>, version: u32 }
```

- **Transport:** bounded `mpsc::channel(64)` per agent; a single
  `broadcast`/`watch` channel publishes a *projection* of envelopes to the UI.
  UI never reads agent channels directly.
- **Private context:** each worker has its own message vector, system prompt,
  model choice and token budget. **[source]** opencode subagents explicitly
  run in fresh child sessions; Codex passes structured `AgentInput::Message`
  into a child thread. Workers do not see each other's conversations.
- **Shared context (blackboard):** an append-only artifact store
  (`ArtifactId → content`), plus a small read-only workspace snapshot (paths,
  git status) and the task graph. Workers receive *references and summaries*,
  not full transcripts — this keeps prompts small for the 2B model.
- **Handoff:** a result is `(ArtifactId, summary ≤ 400 tokens, tokens_used)`;
  the orchestrator decides what to inject into the next worker's prompt. This
  is the "parent receives a report when a child goes idle" pattern from
  opencode, made explicit.
- **Approval:** `NeedApproval` is routed to the UI as a chip; the HUD sends
  `Approval` back through the orchestrator. No blocking modal in a worker.

## 4. Cancellation, back-pressure, error recovery

- One root `CancellationToken` per user request; each task gets a child token;
  spawning a child task inherits it. Cancelling the root cancels all workers,
  all HTTP streams and all subprocesses (kill process tree on Windows via
  `JobObject` **[inferred]** — verify before implementing).
- Every `await` in a worker is wrapped in `tokio::select!` with the token;
  exhausted budgets also produce `Cancel { reason: "budget" }`.
- **Back-pressure:** semaphore of size `max_workers` (default 3 on this
  6C/12T machine, configurable in typed settings) and a second semaphore of
  size 2 for concurrent model requests (LM Studio serves one model at a time
  efficiently; avoid queueing large prompts). Channels are bounded; if the UI
  projection lags, drop intermediate `Progress` events but never `Result` or
  `NeedApproval`. Tool output is truncated at capture time (opencode's
  `toolOutput.truncate`) to keep context and channels small.
- **Error recovery:** retries 2–3 with jitter on transient HTTP errors;
  malformed tool JSON is returned as a tool result with the validation error;
  worker panic → `TaskState::Failed`, orchestrator retries or surfaces a HUD
  error chip; no silent losses.
- **Resume:** replay the JSONL log; tasks in `Running` are reset to `Ready`;
  artifacts are reloaded by id; a user can re-issue the root request without
  re-downloading anything.

## 5. Swarm representation in the HUD

The UI is a *projection* of `(GraphActor, envelopes)`, never a second source of
truth. Concrete layout (uses the README's visual language):

```
┌─ LEGION ────────────────────────────────── ╭─────────╮ ─┐
│ ◉ ONLINE    ctx 12.4k    model qwen3.5-2b │  WORKERS │  │
│                                           │  3 / 3   │  │
│  › analyse the repo and report findings   │ ╰─────────╯  │
│    ├─ T12 reading files       ▸▸▸  w1                │
│    ├─ T13 running tests      ▒▒▒▒  w2   [needs ok]  │
│    └─ T14 synthesize         ─────  queued (T12,T13)│
│  ──────────────────────────────────────────────────────  │
│  › ask the swarm anything…                         ↵    │
└─────────────────────────────────────────────────────────┘
```

- One row per *active* task, not per agent: phase glyph + worker id + budget
  bar; `queued` rows show dependency ids.
- Worker chips: idle / busy / blocked-on-approval; colour from typed palette
  (degrades to ASCII markers in no-colour mode).
- Graph view toggled by a key: compact ASCII DAG using box-drawing/Braille,
  driven directly by `Edge` list.
- Progress events are coalesced to ≤ 30 Hz before reaching the renderer; the
  renderer remains seek-safe (phase derived from `Instant`, not event count).
- An `Artifact` arriving updates the "findings" strip, not the whole screen —
  diff-only redraw handles the rest.

## 6. Risks and open questions for the lead engineer

- **Windows process-tree kill**: confirm the `JobObject` approach on Windows 11
  before relying on it for subprocess cancellation (unverified here).
- **LM Studio concurrency**: exp_101 shows one model id loaded in this server;
  two simultaneous requests may serialise inside LM Studio. The model
  semaphore should be configurable, with default 2.
- **Model quality**: `qwen3.5-2b` is stronger than the intended 0.6B, so
  swarm-level behaviour on a deliberately weak model is only partially
  exercised; exp_103 probes malformed calls.
- **Sub-agent depth**: cap at 2 (orchestrator → worker → no further spawn),
  following both opencode and Codex depth limits.
- **Config**: all of the above (`max_workers`, budgets, timeouts, palette,
  reduced motion) must live in one typed settings file, not CLI flags.
