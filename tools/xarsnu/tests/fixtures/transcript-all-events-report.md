# xarsnu run report

- Transcript schema: 1
- Scenario reference: `schedule-negotiation-1.toml`
- Gate format: `tree+proj`
- Models:
  - `alice`: `example/alice` (temperature 0.25)
  - `bob`: `example/bob` (temperature 0.25)

## Dialog

**alice:** mi klama

*(alice forfeited turn 1)*

*(alice submitted an answer)*

*(checker: partial)*

*(run aborted: cost budget exceeded)*

<details><summary>Scenario instance snapshot</summary>

```toml
id = "schedule-negotiation-1"
title = "Recurring Tuesday planning"
public-setup = "Agree on one recurring weekly meeting. Submit the weekday, minutes after midnight at which it starts, and its duration. Every participant's private availability must be satisfied."
minimum-rounds = 1
maximum-rounds = 3
maximum-turns = 5
scenario-type = "schedule-negotiation"
meeting-duration-minutes = 60
slot-granularity-minutes = 30

[answer-schema]
additionalProperties = false
required = [
    "day",
    "start_minute",
    "duration_minutes",
]
type = "object"

[answer-schema.properties.day]
enum = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
]
type = "string"

[answer-schema.properties.duration_minutes]
maximum = 1440
minimum = 1
type = "integer"

[answer-schema.properties.start_minute]
maximum = 1439
minimum = 0
type = "integer"

[[participants]]
name = "alice"

[[participants.availability]]
day = "tuesday"
start-minute = 540
end-minute = 720

[[participants]]
name = "bob"

[[participants.availability]]
day = "tuesday"
start-minute = 660
end-minute = 750
```

</details>

## Turn 1 — `alice`

### Intent revision

Participant: `alice`

> I am going to the market.

Revision number: 1

### Parse attempt 1

Participant: `alice`

> not lojban

**Gate result:** rejected (morphology)

Diagnostics (verbatim):

> error: invalid word at byte 0

### Parse attempt 2

Participant: `alice`

> mi klama

**Gate result:** accepted

tersmu rendering (verbatim):

> (klama mi)

### Sender confirmation

Verdict: **mismatch**

Paraphrase:

> I go somewhere.

Discrepancies:

> The destination was not expressed.

### Posted message

Lojban:

> mi klama

tersmu rendering:

> (klama mi)

### Blind interpretation — `bob`

> Alice goes somewhere.

### tersmu revealed to `bob`

> (klama mi)

### Acknowledgment — `bob`

Final understanding:

> Alice goes somewhere.

Recorded discrepancies:

> I initially inferred a destination.

### Reference tool `vlacku` — `alice`

Status: **success**

Arguments:

> {"word":"klama"}

Result:

> klama: to go or come

### Repeated reference lookup — `alice` / `vlacku`

Exact-query occurrence: **2**; reference calls remaining in phase: **14**.

### Reference-call budget exhausted — `alice`

Phase maximum: **16**; reference tools withdrawn.

### Reference-research nudge — `alice`

Consecutive reference calls: **6**

Correction:

> Reference research has not advanced the protocol. Current phase protocol tools: `register_intent`, `submit_lojban`. Current phase intent: compose or revise Lojban for the registered intent and submit it. Use a protocol tool now unless another reference lookup is essential.

### Protocol error — `bob` / `submit_lojban`

> tool is not legal in the listener phase

### Turn forfeited — `alice`

Reason: parse-attempt cap (2)

### Scenario answer — `alice`

```json
{
  "scenario-kind": "schedule",
  "answer": {
    "answer": {
      "day": "tuesday",
      "start_minute": 660,
      "duration_minutes": 60
    }
  }
}
```

### Scenario checker

Aggregate status: **partial**

### API usage — `alice`

120 prompt + 30 completion = 150 tokens; $0.012000; 80 cached, 40 cache-write tokens

Reasoning field present: true; reasoning tokens: 20

### Run aborted

Cost budget $0.010000; actual cost $0.012000.

## Run finished

Outcome: **budget aborted** after 1 turn(s).

## Summary

### Task outcomes

Aggregate: **partial**
- `alice`: correct
- `bob`: incorrect

### Parse attempts

- Turn 1: 2
- morphology failures: 1
- syntax failures: 0
- other failures: 0

### Revisions and mismatches

- Intent revisions: 1
- Confirmation mismatches: 1

### Reference-loop mitigations

- Memoized repeats: 1
- Phase budgets exhausted: 1
- Idle-research nudges: 1

### Divergence flags

- Turn 1: sender intent and blind interpretation are both recorded; review them side by side.
- Turn 1: `bob` acknowledged with discrepancies: I initially inferred a destination.

### Protocol stops

- Protocol errors: 1
- Forfeits: 1
- Budget aborts: 1
- Runtime failures: 0

### Usage

- `alice`: 120 prompt + 30 completion = 150 tokens; $0.012000
  - Cache totals: 80 cached tokens; 40 cache-write tokens
  - Cache efficiency: 66.67% (80 / 120 prompt tokens)
  - Call hit rate: 100.00% (1 / 1 provider calls)
  - Reasoning totals: 20 tokens across 1 provider calls
- `bob`: 0 prompt + 0 completion = 0 tokens; $0.000000
  - Cache totals: 0 cached tokens; 0 cache-write tokens
  - Cache efficiency: n/a (0 / 0 prompt tokens)
  - Call hit rate: n/a (0 / 0 provider calls)
- Run total: 120 prompt + 30 completion = 150 tokens; $0.012000
  - Cache totals: 80 cached tokens; 40 cache-write tokens
  - Cache efficiency: 66.67% (80 / 120 prompt tokens)
  - Call hit rate: 100.00% (1 / 1 provider calls)
  - Reasoning totals: 20 tokens across 1 provider calls
