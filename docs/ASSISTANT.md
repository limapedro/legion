# Assistant vision: tools and connectors

Legion is not just a coding-agent harness. Its goal is an **ambient,
voice-first assistant** that lives in the terminal: it can hear/understand,
act through tools, remember, and drive external systems through connectors —
the class of product people expect from modern voice assistants, but local,
scriptable and terminal-native.

The interface language stays conceptual: a calm head-up display, a voice
waveform widget, radial progress rings. No direct references to fictional or
branded assistants in user-facing docs.

## Capability layers

1. **Reasoning** — the model loop (`ModelProvider`) with streaming text,
   separate reasoning tokens, tool calls and retries.
2. **Tools** — typed actions the assistant can take locally: filesystem,
   shell, web, notes, tasks, system info, clipboard, time. Every tool has a
   typed input schema, an approval policy and a result type.
3. **Connectors** — typed adapters to external systems (Home Assistant,
   calendar, mail, music, notifications, local speech services). Connectors
   are configuration-driven and optional; missing credentials degrade to a
   clear "not connected" state rather than breaking the harness.
4. **Voice** — waveform/orb widget (mock input first), wake-word and
   push-to-talk later, speech-to-text and text-to-speech behind connector
   traits so no vendor is baked in.
5. **Swarm** — the same task graph that coordinates coding workers also
   coordinates assistant sub-tasks ("research this", "summarise that",
   "prepare a plan") in parallel.

## Tool catalogue (target for M2/M3)

| Tool | Purpose | Risk / approval |
| --- | --- | --- |
| `read_file` | Read a UTF-8 file in the workspace | low, auto-allow in session |
| `write_file` | Create/overwrite a file | medium, prompt on first use |
| `list_dir` | List a directory | low |
| `search_files` | Filename/content search | low |
| `run_command` | Run a child process with a timeout | high, always prompt |
| `web_search` | Search the web via configured provider | low, network disclosure note |
| `fetch_url` | Fetch and extract text from a URL | low |
| `get_time` | Local/UTC time and timezone | low |
| `system_status` | CPU, memory, disk, uptime | low |
| `notes_add` / `notes_search` | Local markdown notes | low |
| `task_add` / `task_update` / `task_list` | Shared task graph (also used by swarm) | low |
| `clipboard_read` / `clipboard_write` | Windows clipboard | medium |
| `notify` | Toast/terminal notification | low |
| `connector_call` | Invoke a configured connector action | per-connector policy |

## Connector design

A connector is a typed struct implementing:

```
trait Connector: Send + Sync {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn actions(&self) -> Vec<ActionSpec>;   // name + JSON schema + risk
    async fn call(&self, action: &str, args: Value) -> Result<Value, ConnectorError>;
}
```

- All actions become tools in the registry with names like
  `homeassistant.light.set`, `calendar.event.create`.
- Credentials come from typed config/env, never CLI flags, and are never
  logged. `.env` / `config.local.*` are gitignored.
- A connector that cannot initialise registers itself as `unavailable`; the
  model sees a short description saying so and stops trying to call it.
- First connectors planned: local speech (whisper/TTS), Home Assistant REST,
  generic JSON/HTTP webhook, and a calendar read-only connector.

## Tool-calling test strategy (required, not optional)

- **Golden fixtures** from `experiments/exp_001` and `exp_103`: valid call,
  no call with `finish_reason: length`, unknown tool, string-for-number
  coercion, object-shaped forced `tool_choice` HTTP 400.
- **Mock provider** injects malformed JSON, missing required fields, wrong
  types, duplicate call ids and timeouts so retries are deterministic.
- **Schema validation** plus **domain validation** before execution (e.g.
  `read_file.path` must stay inside the workspace allow-list).
- **Approval flow** tests: auto-allow, prompt/deny, session-wide allow, and
  cancellation while waiting for approval.
- **Real-model smoke test** against LM Studio `qwen3.5-2b` with `read_file`
  and `get_time`, recorded in `experiments/exp_NNN/FINDINGS.md` including
  negative results.

## Safety defaults

- Tools run as child processes where they touch the OS; the harness keeps a
  single owner of the console.
- Destructive commands and arbitrary network writes require explicit
  approval every time.
- The weak dev model is never trusted to decide policy: the typed layer does.
