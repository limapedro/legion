# Harness fundamentals — how real agent harnesses work

Status: study report for the lead engineer's review. Nothing here is a locked
decision; see `ARCHITECTURE_LOG.md` for decisions.

Evidence labels: **[source]** = read from a shallow clone or official docs on
2026-11-27 (paths below); **[measured]** = ran it on the target machine
(experiments); **[inferred]** = reasoned from the source/docs, not executed.

Clones were shallow (`git clone --depth 1`) into
`C:\Users\multi\AppData\Local\Temp\opencode\clones\` and removed afterwards
because C: was full. Revisions read: opencode `5d1f788`, aider `5dc9490`, codex
`045a11f`.

- opencode: <https://github.com/anomalyco/opencode> — `packages/core/src/…`,
  `packages/ai/src/…`
- aider: <https://github.com/Aider-AI/aider> — `aider/…`
- Codex CLI: <https://github.com/openai/codex> — `codex-rs/…`
- Anthropic tool use: <https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview>
- OpenAI function calling: <https://developers.openai.com/api/docs/guides/function-calling>
- opencode agents/subagents: <https://opencode.ai/docs/agents/>

## 1. The agent loop

Every harness implements the same shape, with different bookkeeping:

```
user prompt -> build messages (system + history + context) -> model stream
   -> text deltas to UI; tool_call events -> execute tools
   -> append tool results to history -> loop until no tool calls / stop reason
```

**[source] opencode** — `packages/core/src/session/runner/llm.ts` `drain()`
(lines 71–170) is the turn loop. It pulls the next item from a persistent
`SessionInbox`, settles stale compactions and tool calls on entry (lines 86–87),
runs step after step (`step` counter, line 75), honours `Delivery` modes
`steer` vs `queue` (line 83), and handles `move`/`compaction` control items
between steps. `packages/core/src/session/runner/step.ts` `attempt()` (lines
79–180) consumes `llm.stream(...)` event by event: text/progress deltas are
published immediately (lines 138–147); each `tool-call` event forks a tool fiber
(`Effect.forkScoped`, line 164) so multiple tool calls in one response run
concurrently; `toolOutput.truncate` bounds the result before it is published
(line 155). Step outcomes are a tagged enum — `Completed`, `Retry`,
`RecoverFull`, `Compacted`, `OutputLimit` (`step.ts` lines 30–41) — which is a
clean model for a harness state machine.

**[source] aider** — `aider/coders/base_coder.py` `run()` (line 876) is the
outer REPL; `run_one()` (line 924) calls `send_message()` in a loop while a
`reflected_message` (lint/test errors, missing file mentions) is set, with
`max_reflections` as the guard (lines 932–944). `send_message()` (line 1419)
formats chat chunks, checks tokens, warms the cache, streams the reply, then
`reply_completed()` (line 1625) applies edits and may set another reflection.
This is a single-agent, strictly sequential loop by design.

**[source] Codex CLI** — `codex-rs/core/src/tools/parallel.rs` implements the
dispatcher for parallel tool calls with `tokio::task::JoinHandle`,
`CancellationToken` and `AbortOnDropHandle` (imports lines 4–8). The turn loop
lives in `codex-rs/core/src/agent/` / `thread_manager.rs`; tool specs are
registered in `codex-rs/core/src/tools/spec_plan.rs`. Cancellation is a
`CancellationToken` threaded through `ToolCtx` (`orchestrator.rs` lines
79–85), so a user stop propagates to running tools and network approvals.

**Lesson for Legion:** the loop is cheap; the expensive parts are history
management, cancellation, and making tool results safe to feed back. Keep the
loop in one place with explicit outcomes, as opencode does.

## 2. Tool-call formats

Three formats are in play across these projects:

| Format | Where | Shape |
| --- | --- | --- |
| Native function calling (JSON Schema) | LM Studio / OpenAI-compatible, Anthropic Messages API | `tools: [{type:"function", name, description, parameters}]`; reply carries `tool_calls[].function.arguments` as a **JSON string** with a `call_id`; result sent back keyed by that id |
| Typed content blocks | Anthropic Messages API | `stop_reason: "tool_use"`, `tool_use` blocks with `input_schema`, then a user `tool_result` block keyed by `tool_use.id` ([source] Anthropic docs above) |
| Free-form edit protocols | aider | `editblock`, `editblock_fenced`, `udiff`, `wholefile` coders under `aider/coders/` |

**[measured]** exp_101: LM Studio `qwen3.5-2b` returned
`{"type":"function","id":"Qp5…","function":{"name":"get_weather",
"arguments":"{\"city\":\"Porto\"}"}}` and `finish_reason: tool_calls`. So
Legion must parse a JSON *string*, validate it against the tool's typed schema,
and re-send results with the same `id` (OpenAI Chat Completions convention).

**[source]** Anthropic's current docs use JSON `input_schema`, `strict: true`,
`tool_choice: {"type":"auto","disable_parallel_tool_use":true}`; there is no
need for Legion to implement a bespoke XML tool language. **[inferred]** XML-
tagged tool arguments are a legacy prompt-level convention from early Claude
Code-era tooling; supporting them would add parser surface for no benefit on a
Qwen/LM Studio dev loop.

**[source]** OpenAI docs stress: assume *several* calls per turn; preserve the
model's tool-call items in history; `strict` requires `additionalProperties:false`
and all properties `required`; streaming can expose partial argument JSON.
This matches what a robust harness must handle.

**[source]** aider's `aider/coders/editblock_*_coder.py` and
`wholefile_func_coder.py` show why a harness may want a text fallback: weaker
models emit fenced patches more reliably than structured calls. For Legion,
the dev model is deliberately weak, so keep a **schema-validated function call
as the only execution path**, and treat malformed calls as errors to reflect
back to the model (see exp_103).

## 3. Streaming, deltas and cancellation

**[source]** opencode `step.ts` lines 134–168: the provider is a `Stream`;
each event is published to a bus; tool calls are forked; `stopTools()`
(lines 112–120) first asks cooperative cancellation (`cancel.trigger()`),
then interrupts fibers with a timeout, then force-interrupts. Final step
events are emitted under `Effect.uninterruptibleMask` (line 172) so the UI
never loses the end of a turn. `packages/ai/src/llm.ts` exposes
`generate`, `stream`, `loop` and `compact` primitives.

**[source]** aider `base_coder.py` lines 1440–1447: a `WaitingSpinner` runs
until the first delta; `mdstream` renders Markdown incrementally;
`show_send_output_stream()` (line 1806) consumes the stream. `KeyboardInterrupt`
sets `interrupted` and appends `^C KeyboardInterrupt` to history (lines
1489–1491, 1575–1578) — a nice pattern: cancellation becomes an explicit
conversation event, not a silent kill.

**[source]** Codex `tools/parallel.rs` imports `CancellationToken` and
`AbortOnDropHandle`; `orchestrator.rs` passes it into every tool attempt.

**[measured]** exp_101: LM Studio streaming returns `text/event-stream` with
`chat.completion.chunk` SSE frames. **[inferred]** A Rust client should parse
SSE incrementally (e.g. `reqwest` bytes stream + a small line-state machine),
coalesce `[DONE]`, and expose a `Stream<Item = Delta>` where a `Delta` is
`Text(String) | ToolCallDelta{id,name?,arguments_fragment} | Usage | Done`.
Cancellation = dropping the stream/aborting the HTTP request plus a
`CancellationToken` shared with tool tasks.

Cancellation edge cases seen in source: provider dies mid-arguments
(opencode's `RESULT_MISSING` at `step.ts` line 70); user stop while tools run
(lines 112–120); output-limit mid-turn (`aider` lines 1492–1505, opencode
`CONTINUE_AFTER_OUTPUT_LIMIT` in `llm.ts` lines 43–44).

## 4. Context management: budgeting, truncation, compaction, caching

**[source] opencode** `session/compaction.ts` is the most explicit policy:

- `Settings` (lines 46–54): `auto`, `buffer` (default 15% of window, minimum
  16k tokens free — `RESERVE_SHARE` line 137, `RESERVE_MIN` line 131),
  `keep` recent tokens verbatim, `background` precompute.
- On provider "too long" rejection, retry with shrinking fractions
  `SHRINK_STEPS = [0.85, 0.7, 0.5, 0.35]` (line 129).
- Transcript reduction levels (lines 143–152): full → clip tool outputs →
  drop tool outputs and clip inputs → user text only → omitted; clipped items
  keep 2 000 chars from each end (`TRANSCRIPT_CLIP` line 152).
- Background summaries start ~60k tokens before the ceiling
  (`PRECOMPUTE_LEAD` line 159) because writing one takes 30–60 s; a failed
  summary is retried on later turns. A summary references the messages it
  keeps verbatim via `metadata.keepID` (lines 89–97).
- Token estimates: 1 500 for images, 2 000 for PDFs (lines 153–154).

**[source] aider** `aider/repomap.py` budgets ~1 024 tokens of repository map
by default (line 49), estimates tokens by sampling and scaling (lines 89–101),
and caches parsed tags on disk with `diskcache` (`TAGS_CACHE_DIR` line 43,
`cache_threshold` 0.95 line 68). `base_coder.py` `check_tokens()` warns at
estimated overflow (lines 1401–1417) and asks the user before proceeding.

**[source] aider prompt caching** `base_coder.py` `format_messages()` adds
cache-control headers when supported (lines 1333–1337); `warm_cache()` (lines
1340–1389) sends a `max_tokens: 1` completion every ~5 minutes using only the
cacheable prefix to keep the provider cache warm. This only matters for cloud
providers; **[inferred]** LM Studio 1.x does expose prompt caching controls,
but a local 2B model makes it a non-priority for Legion's dev loop.

**[source]** Codex compaction lives in `codex-rs/core/src/compact.rs` with a
token-budget module (`compact_token_budget.rs`) and retained-tool-output
handling (`compact_retained_tool_outputs.rs`). Rollout files can be compacted
and resumed (`rollout/src/compression.rs`, `compact_resume_metadata`).

**Lesson for Legion:** a small local model has a small context (often 8–32k).
Legion needs: (1) token estimate before each request, (2) hard budget for tool
results (truncate at capture time), (3) summarise-and-keep-tail compaction,
(4) a visible `ctx 12.4k` HUD readout driven by the same counter the loop
uses. Do not implement caching until a cloud provider is on the roadmap.

## 5. Permissions and approvals

**[source] opencode** `packages/core/src/permission/policy.ts`, `saved.ts` and
the documented agent config at <https://opencode.ai/docs/agents/>: rules are
ordered `{action, resource, effect}` with `effect: allow | ask | deny`, last
matching rule wins; actions include `shell`, `edit`, `read`, `subagent`,
`webfetch`, `websearch`, `skill`. Built-in agents show the intended split:
`build` (full access, env-file reads ask), `plan` (no edits), `general`
(subagent, broad tools, cannot spawn more), `explore` (read-only). Parent
permissions control which subagents may be launched; the child uses its own.

**[source] Codex** `codex-rs/core/src/tools/orchestrator.rs` (lines 1–3, 51–97)
coordinates approval, sandbox selection and retries after sandbox denial;
`codex-rs/core/src/tools/approvals.rs` and
`execpolicy.rs` hold the allow/ask/deny policy; `AskForApproval` and
`ReviewDecision` are protocol enums. A denied sandboxed attempt is re-reviewed
before escalation.

**[source] aider** `aider/io.py` has `yes` (auto-confirm) and
`confirm_ask()`; file edits and shell commands go through confirmation unless
`--yes`. aider's permissions are coarse compared with opencode/Codex.

**Lesson for Legion:** keep a typed `PermissionPolicy` in config (not CLI
flags), with three effects and resource globs, evaluated at tool-call time;
the HUD should show an approval chip when the swarm blocks on a user decision.
Shell/network are `ask` by default; reads inside the workspace are `allow`;
writes outside workspace and subagent spawning are policy-controlled.

## 6. Session persistence, replay and resume

**[source] opencode** persists sessions in SQLite (`packages/core/src/database/`,
`session/sql.ts`, `session/store.ts`); messages, tool calls, jobs and
permissions are rows. `SessionInbox` (session/inbox.ts) is a durable queue:
input can be `steer` or `queue` and survives restarts; `subagent-job.ts` lines
32–37 explicitly keep a child resumable when the server shuts down (only an
explicit caller cancel releases the child's claim). Event bus events
(`session/event.ts`) give a replayable activity stream for a UI.

**[source] Codex** uses JSONL rollouts: `codex-rs/rollout/src/lib.rs` defines
`RolloutLine`, `RolloutItem`, `CompactedItem`, `ResumedHistory`;
`decode_rollout_line()` (lines 44–70) handles a serde JSON quirk at the
persistence boundary. Files include timestamp + optional ordinal, so replay
and resume are deterministic. There is also a state DB bridge
(`core/src/state_db_bridge.rs`) and a message history crate.

**[source] aider** keeps chat history in `.aider.chat.history.md` via
`aider/history.py::ChatSummary` and input history in `aider/io.py`; repo state
is git commits (`aider/repo.py`) — resume means "open the files and the chat
history again", not a full event log.

**Lesson for Legion:** append-only JSONL is the smallest format that supports
both replay and a live HUD feed: every line is
`{seq, ts, session_id, agent_id, event}`. Store deltas sparingly (aggregate
text per message), but always store tool calls/results and status transitions.
Resume = read the file, rebuild typed state, continue from the last sequence
number. This also gives the UI a natural event source.

## 7. Error recovery

Common patterns from the sources:

- **Retries with backoff:** opencode `runner/retry.ts` lines 32–55 obtains a
  decision from a policy (with plugin hook override), emits `RetryScheduled`
  (lines 60–76) and sleeps the remaining delay; aider `send_message()` starts
  at 0.125 s and doubles up to `RETRY_TIMEOUT` (lines 1449–1487).
- **Provider "too long":** opencode compacts and re-aims at 85–35% of the
  rejected request; aider treats `ContextWindowExceededError` as terminal for
  the request and asks the user to drop files or clear history.
- **Malformed / missing tool results:** opencode defines
  `RESULT_MISSING` = "Provider did not return a tool result" (`step.ts` line
  70); tool errors are caught per call and published as failed tool events
  (lines 160–162) so the turn continues.
- **Output limit:** opencode injects a continuation prompt
  (`CONTINUE_AFTER_OUTPUT_LIMIT`, `llm.ts` lines 43–44); aider either reflects
  the partial content as an assistant prefix (`supports_assistant_prefill`)
  or stops (`base_coder.py` lines 1492–1505).
- **User interrupt:** aider records `^C KeyboardInterrupt` in history;
  opencode's two-stage cooperative-then-forced tool stop.
- **Timeouts:** Codex LM Studio client uses a 5 s connect timeout and emits a
  specific 'LM Studio is not responding' message
  (`lmstudio/src/client.rs` lines 15–16).

**[inferred] for Legion:** every external call gets a timeout, bounded
retries (2–3 with jitter), and a typed error that the UI can display without
leaking stack traces. Malformed tool JSON should be returned to the model as
a tool result containing the validation error and a short correction hint —
this is exactly the behaviour exp_103 probes with the weak local model.

## 8. Sub-agents and parallelism

**[source] opencode** agents are Markdown/JSON profiles with
`mode: primary | subagent | all`, model preference, system prompt, permissions
and step limits (<https://opencode.ai/docs/agents/>). Subagents always run in
the background in their **own child session with fresh context**; the parent
gets a report when the child goes idle and after each later round; a parent
permission list controls which agents it may spawn. Internally
`packages/core/src/session/subagent-job.ts` wraps `execution.resume(id)` in a
background `Job`; `compaction-subagents.ts`, `subagent-completion.ts` and
`subagent-depth.ts` handle reporting and depth limits. Jobs outlive the plugin
that launched them (`subagent-job.ts` lines 43–52).

**[source] Codex** has first-class multi-agent tools in
`codex-rs/core/src/tools/handlers/multi_agents_v2/`:
`spawn_agent`, `send_message`, `followup_task`, `wait_agent`,
`interrupt_agent`, `list_agents`, `close_agent`, `resume_agent`
(see `spec_plan.rs` lines 35–51). Arguments are typed structs with
`#[serde(deny_unknown_fields)]` — e.g. `SendMessageArgs { target, message }`
in `message_tool.rs` lines 16–30. `followup_task` differs from `send_message`
only in whether it wakes the target immediately. `agent-graph-store/src/types.rs`
models spawn edges as `Open | Closed`; `agent_communication.rs` tracks pending
mail with a gauge and owns delivery until history commits it. `thread_spawn.rs`
and `spawn.rs` implement process-internal thread spawning with depth limits
(`exceeds_thread_spawn_depth_limit`, `next_thread_spawn_depth` in
`spec_plan.rs` lines 1–2).

**[source] aider** has no subagents; parallelism is limited to background
cache warming (`warm_cache_worker` thread) and asynchronous command waiting.
Its `architect_coder.py` coordinates two roles by *mode switching*, not by
parallel agents.

**[source] Codex tool parallelism** `tools/parallel.rs` runs multiple tool
calls in one turn concurrently and aborts on drop. **[source] opencode**
`step.ts` forks tool calls as fibers with a shared cancel handle.

**Lesson for Legion:** model the swarm as typed threads/agents with explicit
spawn → message → wait → interrupt → close lifecycle, fresh private contexts,
and a shared, append-only result bus. Cap depth (Codex and opencode both do)
and keep a single orchestrator that is the only agent allowed to spawn.

## Summary of transferable design rules

1. One explicit loop with tagged outcomes; never hide state in callbacks.
2. Function calling with JSON-string arguments; validate and echo errors back.
3. Stream to the UI as events; cancellation is cooperative then forced.
4. Budget context before the request; truncate tool output at the boundary;
   compact with a kept tail, not a blind truncation.
5. Typed permission rules in config; approval is an event, not a modal dialog.
6. Append-only JSONL session log = resume + HUD feed from one source.
7. Bounded retries, explicit output-limit continuation, recorded interrupts.
8. Swarm = private contexts + typed message passing + shared status bus, with
   depth and concurrency limits.
