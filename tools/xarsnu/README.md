# xarsnu

`xarsnu` runs a bounded, tool-gated Lojban discussion and records every event
as flushed JSONL:

```console
xarsnu path/to/run.toml
xarsnu report path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
xarsnu report --dialog path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
```

## Run configuration

```toml
scenario = "schedule-negotiation-1.toml"
tersmu-format = "tree+proj"

[caps]
max-parse-attempts-per-turn = 3
max-intent-revisions-per-turn = 2
max-turns = 8
max-cost-usd = 1.25
max-reference-calls-per-phase = 16
reference-dedupe = true
reference-nudge-after = 6

[[participants]]
name = "alice"
model = "anthropic/example-model"
prompt-caching = "auto"
tool-choice = "metadata"
temperature = 0.4
system-prompt = "Speak only Lojban in the visible discussion."

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

`tool-choice` is also per participant and defaults to `metadata`. The vendored
OpenRouter capability snapshot selects `required` for models known to support
required tool calls and otherwise selects `auto`. Explicit `required` and
`auto` values override metadata. Automatic mode records every prose rejection
as a typed event and uses the existing bounded corrective machinery: models
that support assistant prefill receive `Actually, I must use one of the
following tools: ...`; other models receive the existing user correction.
Exhaustion forfeits a speaker turn. Listener exhaustion instead records a
listener-scoped `listener-flow-abandoned` event, leaves that listener's blind
interpretation and acknowledgment unrecorded, and continues with the other
listeners.

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

The reference-loop controls are run-wide caps applied independently to every
protocol phase. `max-reference-calls-per-phase` defaults to 16 and withdraws
all reference tools after that many calls in one phase. `reference-dedupe`
defaults to `true`; repeated calls with the exact same tool name and argument
bytes reuse the first local result while still consuming the reference-call
budget. `reference-nudge-after` defaults to 6 and sends one phase-progress
reminder after that many consecutive reference calls. Both numeric values must
be positive, and the nudge threshold must be lower than the call cap.

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
