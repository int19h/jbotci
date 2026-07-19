# xarsnu

`xarsnu` runs a bounded, tool-gated Lojban discussion and records every event
as flushed JSONL:

```console
xarsnu path/to/run.toml
xarsnu report path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
xarsnu report --dialog path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
xarsnu report --community path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
```

`--community` produces one shareable Markdown document with the public scenario
and participant roster, a turn-numbered chat-room log, and concise chronological
views of each participant's internal loop. It omits raw protocol payloads and
private scenario briefs. Rejected Lojban candidates retain their complete rich
diagnostics in fenced blocks so the export shows exactly what the model received.

## Run configuration

```toml
scenario = "schedule-negotiation-1.toml"
tersmu-format = "tree+proj"
listener-mode = "informed"
allow-degraded-search = false

[caps]
max-parse-attempts-per-turn = 3
max-intent-revisions-per-turn = 2
max-turns = 8
max-cost-usd = 1.25
max-reference-calls-per-phase = 30
reference-dedupe = true
reference-nudge-after = 10

[[participants]]
name = "alice"
model = "anthropic/example-model"
prompt-caching = "auto"
tool-choice = "metadata"
temperature = 0.4
system-prompt = "Speak only Lojban in the visible discussion."

[participants.provider]
only = ["xiaomi/fp8"]

[[participants]]
name = "bob"
model = "example/other-model"
prompt-caching = "off"
tool-choice = "required"
reasoning = "low"
temperature = 0.6
system-prompt = "Speak only Lojban in the visible discussion."
```

Participant names must match the selected scenario instance exactly. Personas
belong in `system-prompt`. Public setup and participant-scoped private briefs
come only from the scenario instance; the removed `private-brief` participant
field is rejected as an unknown field.

`provider` is an optional per-participant OpenRouter routing table. Xarsnu does
not model or reinterpret its keys: the TOML table is serialized directly as
the request's `provider` object, so OpenRouter options such as `only`, `order`,
and `ignore` remain available. For example, the `xiaomi/fp8` pin above routes
MiMo directly through Xiaomi. OpenRouter's serving `provider` response field is
recorded on every usage event, and the report summarizes the observed provider
mix per participant so routing drift is visible.

The `scenario` value is a path, including its extension. An absolute path is
used directly. A relative path is first resolved against the run config's
directory, then against this crate's `tools/xarsnu/scenarios/` directory.

Scenario instances may set `answers-close-dialog = true|false`. It defaults to
`true` for referential games and `false` for negotiation and deduction. When
enabled, completing the first answer-eligible round closes the visible dialog:
no more speaker turns or posts occur, a typed closure event is recorded, and
every required participant receives the same answer-only instruction before
any answer is requested. Those private answer phases offer only
`submit_answer`, so one participant's submission cannot affect another's.

`prompt-caching` is per participant. `auto` (the default) emits explicit cache
breakpoints only for models that require them; `off` leaves the request alone.

`listener-mode` defaults to `informed`. Every listener receives the posted
Lojban, its tersmu rendering, and the embedded definitions together from the
start, then records one acknowledgment. `blind-then-reveal` retains the
two-step blind interpretation and later parser reveal for explicit measurement
arms. The selected mode is recorded in the run header, listener-flow events,
and reports.

`tool-choice` is also per participant and defaults to `metadata`. The vendored
OpenRouter capability snapshot selects `required` for models known to support
required tool calls and otherwise selects `auto`. Explicit `required` and
`auto` values override metadata. Automatic mode records every prose rejection
as a typed event and uses the existing bounded corrective machinery: models
that support assistant prefill receive `Actually, I must use one of the
following tools: ...`; other models receive the existing user correction.
Exhaustion forfeits a speaker turn. Listener exhaustion instead records a
listener-scoped `listener-flow-abandoned` event, leaves that listener's current
interpretation flow incomplete, and continues with the other listeners.

`reasoning` is an optional per-participant policy: `off`, `default`, `low`,
`medium`, or `high`. When it is absent, reasoning is disabled only if metadata
says the model supports that setting and its effective tool choice is
`required`; otherwise the provider default is requested. `off` sends effort
`none`, `default` sends `enabled = true`, and the explicit effort values map
directly to OpenRouter. Every shape keeps `exclude = false` so returned traces
remain available for review and requests OpenRouter's maximum reasoning-summary
verbosity with `summary = "detailed"`.

Returned `reasoning` and `reasoning_details` are private observability. They
are recorded per provider call and rendered under `### Thinking` in the full
report, but never enter the visible dialog or the canonical message history.
Within one continuing tool loop, observed `reasoning_details` are attached
verbatim to their originating assistant messages in subsequent requests, then
discarded at the next loop boundary.

Existing run configs and schema-v1 transcript headers using
`disable-reasoning = true|false` remain readable and map to `off|default`, but
newly serialized configuration uses only the unified `reasoning` field.

The audited snapshot lives in `openrouter-model-capabilities.json` and records
its exact Bickr source commit. Refresh Bickr's generated map first, then import
the five policy fields deterministically:

```console
(cd ~/git/bickr && node scripts/probe-openrouter-model-capabilities.mjs)
python3 tools/xarsnu/scripts/import-bickr-openrouter-capabilities.py --bickr ~/git/bickr
```

The meaning-first doctrine tells participants to search `vlacku` by meaning
before choosing uncertain content words. When the problem is how to express
something grammatically rather than which word to use, it tells them to query
`cukta` with the concept. The standing rules distinguish freely testing drafts
with reference `tersmu` from the limited commitment made by `submit_lojban`.
They also require participants to verify the precise meaning and type of every
selbri place and reject type-mismatched argument assignments at confirmation.

The reference-loop controls are run-wide caps applied independently to every
protocol phase. `max-reference-calls-per-phase` defaults to 30 and withdraws
all reference tools after that many calls in one phase. `reference-dedupe`
defaults to `true`; repeated calls with the exact same tool name and argument
bytes reuse the first local result while still consuming the reference-call
budget. `reference-nudge-after` defaults to 10 and sends one phase-progress
reminder after that many consecutive reference calls. Both numeric values must
be positive, and the nudge threshold must be lower than the call cap.

Before initializing the model client or starting a debate, the conductor runs
a real semantic `vlacku` query through the production embedding-search path.
Missing or unusable embedding assets fail startup with setup guidance by
default. `allow-degraded-search = true` is reserved for intentional degraded
measurement arms; it continues with a loud CLI/report warning and a typed
`embedding-search-degraded` transcript event.

## Transcript and exit behavior

The transcript is created beside the run config as
`<config-stem>.xarsnu.<unix-nanoseconds>.<pid>.<sequence>.jsonl`. Creation never
replaces an existing file, and every event is flushed before the run continues.
The CLI prints the transcript path and a one-line outcome.

A completed run exits successfully even when the scenario checker reports task
failure, partial success, a turn cap, forfeits, or a cost-budget abort. Runtime,
configuration, I/O, and protocol failures exit unsuccessfully. An orderly
runtime failure is still recorded as the terminal `run-failed` event and can be
rendered by `xarsnu report`; a transcript with no terminal event indicates a
process crash or corruption and is rejected as truncated.
