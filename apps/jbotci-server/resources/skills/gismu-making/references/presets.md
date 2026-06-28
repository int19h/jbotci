# `gimfihi` presets and weighting

A **preset** fixes both the set of source languages and their relative weights,
so you only have to supply the words. When you use a preset you must provide a
source for **exactly** the languages it lists — no more, no fewer — or the call
is rejected. The order of your sources does not matter (they are matched by
language code, not position). Pick a preset, then transcribe one word per listed
language.

Language codes are ISO 639-3: `cmn` Mandarin, `eng` English, `spa` Spanish,
`hin` Hindi, `ara` Arabic, `rus` Russian, `ben` Bengali, `por` Portuguese,
`fra` French, `msa` Malay, `jpn` Japanese, `deu` German.

| Preset | Languages | Notes |
|--------|-----------|-------|
| `1985`, `1987`, `1994`, `1995`, `1999` | `cmn hin eng spa rus ara` | The **classic six**. The original CLL basis; the year names differ only in the speaker-population weights used that year. |
| `evenly` | `cmn hin eng spa rus ara` | The classic six, weighted **equally** — the simplest choice and good for demonstrations. |
| `ilmen6` | `cmn eng hin spa ara fra` | French in place of Russian. |
| `ilmen8` | `cmn eng spa hin ara ben rus por` | Adds Bengali and Portuguese (eight). |
| `ilmen12` | `cmn eng spa hin ara ben rus por msa jpn deu fra` | Twelve languages — the **recommended default** for a real coinage: the broadest, most current basis. |

Weighting: every preset weights languages by the number of speakers (the `ilmen`
presets factor in first- and second-language speakers and how many top languages
to include). Heavier languages pull the result toward their word's sounds.

**Choosing:** default to `ilmen12` unless the user wants something narrower or
asks for a specific basis. Use `evenly` or a year preset when you want the
historical classic-six result, and `ilmen6`/`ilmen8` for a middle ground.

**Custom weights instead of a preset:** omit `preset` and give each source an
explicit `weight` (1–999). Then you choose both the languages and their weights —
useful when the user wants a specific set of languages or a non-standard
weighting. Every source must carry a weight in that case.
