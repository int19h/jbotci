---
name: gismu-making
description: >-
  Coin a brand-new Lojban gismu (a five-letter root word) for a concept using
  jbotci's `gimfihi` tool: choose weighted source words from several languages,
  transcribe each as broad phonetic IPA, run `gimfihi`, and read its ranked
  candidates and collisions. Use this whenever someone wants to invent, coin,
  propose, or create a new Lojban root word / gismu, add a brand-new concept to
  Lojban's core vocabulary, or asks what a good gismu for some meaning would be —
  even if they never say the words "gismu" or "gimfihi". Reach for it before
  guessing a root by hand: getting the source-word IPA and the collision checks
  right is exactly what makes a coined gismu valid and usable.
---

# Making a Lojban gismu

A **gismu** is a five-letter Lojban root word (shape `CVCCV` or `CCVCV`, e.g.
*klama*, *gerku*). Coining one is not free invention. The standard method
(CLL §4.14) blends the everyday words for the concept across several major
languages, weighted by how many people speak each, so the new root echoes
something familiar to as many people as possible. jbotci's **`gimfihi`** tool
does the blending and scoring; your job is to feed it good input and read its
output.

How `gimfihi` scores: it generates every legal five-letter shape and scores each
by how well its letters recall the weighted source words (letters shared in
order), sums the weighted scores, and ranks them — then sets aside any that
collide with an existing gismu. The quality of the result is therefore decided
almost entirely by **how accurately you transcribe the source words**.

## When to use this

Use this any time the goal is to *create a new root word*: "invent a gismu for
X", "what would a good Lojban root for 'gravity' be", "coin a brivla root for
this concept", "add a word for X to Lojban". It does **not** look up existing
words — for that, use the dictionary tools instead.

## The procedure

1. **Frame the concept and pick the source set** — a preset (default `ilmen12`),
   or custom languages with explicit weights.
2. **Transcribe each source word as broad phonetic IPA** — the hard part; see
   [references/ipa-transcription.md](references/ipa-transcription.md).
3. **Call `gimfihi`** with those IPA source words.
4. **Read the result** — the winner, the ranked candidates, and any collisions.
5. **Iterate** if nothing good comes out.

## Step 1 — Sources and weights

Default to the **`ilmen12`** preset unless the user asks otherwise. It draws on
twelve languages (`cmn eng spa hin ara ben rus por msa jpn deu fra`) weighted by
speaker population — the broadest, most modern basis. You must supply one source
word for **exactly** the languages the chosen preset names, or `gimfihi` rejects
the call. See [references/presets.md](references/presets.md) for every preset and
its language set.

To weight languages yourself instead, omit the preset and give each source an
explicit `weight` (1–999), typically scaled to speaker population. Use presets
unless the user specifically wants custom weights.

## Step 2 — Transcribe each source word as IPA

This is where a gismu lives or dies: a single mis-transcribed sound shifts every
candidate's score. For each language in your source set:

- Find the **ordinary, native** word for the concept — not a rare, archaic, or
  obviously borrowed one.
- Write a **broad phonetic IPA transcription** of how it is actually pronounced
  — its *sounds*, not its spelling and not an existing Lojban word.
- Apply the language's real phonology: vowel reduction (e.g. Russian *akanye*),
  final devoicing, assimilation, and positional allophones, as actually spoken.
- **Drop grammatical endings** (Spanish noun *-o*/*-a*, etc.).
- **Never write the schwa `ə`** — use the nearest full vowel it is actually
  pronounced as. Ignore tone and stress.
- When unsure, look the word up in **Wiktionary** and skim the language's
  Wikipedia phonology article; don't guess.

The full transcription guide — the supported IPA inventory, the rules above in
detail, and per-language gotchas — is in
[references/ipa-transcription.md](references/ipa-transcription.md). Read it before
transcribing if you are at all unsure; sloppy IPA is the most common way a coined
gismu comes out wrong.

## Step 3 — Call `gimfihi`

Pass one source per language as `{language, word}` (plus `weight` if you are not
using a preset), where `word` is the IPA:

```json
{
  "preset": "ilmen12",
  "sources": [
    { "language": "eng", "word": "kæt" },
    { "language": "spa", "word": "gat" }
    // ... one per preset language
  ]
}
```

Useful options: `count` (how many candidates to show, 1–512); `show-collisions`
(off by default — turn it on to also see the candidates that were set aside, each
flagged with what it collides with); and `require-free-short-rafsi` (keep only
candidates that have at least one unclaimed short rafsi).

## Step 4 — Read the result

- **winner** — the top-scoring *usable* candidate (collision-free). This is the
  gismu to recommend.
- **candidates** — ranked by score (higher = recalls the weighted sources better).
- **collisions** — by default, candidates that clash with an existing gismu are
  hidden. With `show-collisions: true` they appear, each flagged:
  - `[= existing <type>]` — the candidate already exists as a word of that type
    (a gismu, or an experimental gismu).
  - `[~ word: similar consonant]` — too close to *word* (differs only by a
    confusable consonant), so it is not allowed.
  - `[~ word: final vowel]` — differs from *word* only in the last vowel.
  These are why a high-scoring blend may not be the winner.
- **rafsi** — each candidate's short combining forms and whether each is `free`,
  `official-taken`, or `experimental-taken` (a taken form names the word that
  claimed it). A gismu with no free short rafsi still works but is less convenient
  to build compounds from.

## Step 5 — Iterate

If nothing is appealing: re-check your IPA first (the usual culprit), try a more
everyday word in a language, add or reweight languages, or accept a slightly
lower-scoring but cleaner-sounding winner. You can also raise `count` to see
further down the ranking.

## Worked example — a gismu for "cat"

This uses the **classic six** via the `evenly` preset for a compact, easy-to-
follow demonstration; for a real coinage prefer `ilmen12`. Transcribe the
everyday word in each language, reasoning about each pronunciation:

| Lang | Word | IPA | note |
|------|------|-----|------|
| `cmn` | 猫 *māo* | `mau` | tone dropped |
| `hin` | बिल्ली *billī* | `bɪli` | |
| `eng` | cat | `kæt` | |
| `spa` | gato | `gat` | drop the `-o` noun ending |
| `rus` | кошка | `koʂkɐ` | unstressed final *-а* reduces to `[ɐ]` (akanye); `ш` is retroflex `ʂ` |
| `ara` | قطة *qiṭṭa* | `qitˤ` | drop the feminine `-a`; `ق` = uvular `q`, emphatic `ṭ` = `tˤ` |

Call:

```json
{
  "preset": "evenly",
  "sources": [
    { "language": "cmn", "word": "mau" },
    { "language": "hin", "word": "bɪli" },
    { "language": "eng", "word": "kæt" },
    { "language": "spa", "word": "gat" },
    { "language": "rus", "word": "koʂkɐ" },
    { "language": "ara", "word": "qitˤ" }
  ]
}
```

Result: the winner is **`katli`** (the best collision-free blend). With
`show-collisions: true`, the higher-scoring **`katma`** also appears, but flagged
`[~ katna: similar consonant]` — it differs from the existing gismu *katna*
("cut") only by `m`/`n`, so it is disqualified and `katli` wins instead. That is
the collision machinery doing its job: it keeps you from coining a root that
would be confused with one that already exists.

(The IPA above is illustrative — verify each word's real pronunciation as the
transcription guide advises.)
