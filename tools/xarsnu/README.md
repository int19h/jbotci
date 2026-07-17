# xarsnu

`xarsnu` runs a bounded, tool-gated Lojban discussion and records every event
as flushed JSONL:

```console
xarsnu path/to/run.toml
xarsnu report path/to/run.xarsnu.1750000000000000000.1234.0.jsonl
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

[[participants]]
name = "alice"
model = "anthropic/example-model"
prompt-caching = "auto"
temperature = 0.4
system-prompt = "Speak only Lojban in the visible discussion."

[[participants]]
name = "bob"
model = "example/other-model"
prompt-caching = "off"
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

`prompt-caching` is per participant. `auto` (the default) emits explicit cache
breakpoints only for models that require them; `off` leaves the request alone.

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
