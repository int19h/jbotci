# Phonetic-medoid gismu creation

## Overview

This document describes the **phonetic** candidate scorer for gimfi'i — the
alternative to the **classic** CLL §4.14 letter-count scorer, which remains the
default. It is selected with `jbotci gimfihi --scorer phonetic` (or the
`scorer` parameter of the gimfi'i MCP tool). The similarity model and IPA
machinery live in the `jbotci-phonetic` crate; the gimfi'i driver
(candidate enumeration, weighting, ranking, collisions, rafsi) lives in
`jbotci-gimfihi`.

The scorer coins a new Lojban root from weighted source-word pronunciations by
keeping each source as a full-precision phonemic IPA segment string and
choosing

> **the valid gismu whose own pronunciation is maximally similar, on a
> weighted average, to the source pronunciations.**

Formally, given sources as concrete IPA segment strings s₁ … s_k with weights
w₁ … w_k, and G the set of valid candidate gismu with pronunciation target
sequences targets(g):

```
gismu = argmax over g ∈ G of  Σᵢ wᵢ · sim(targets(g), sᵢ)
```

In the string-averaging literature this object is the **generalized median
(Steiner/consensus string) constrained to a feasible set**. Computing a
generalized median over free-form strings is NP-hard even for plain edit
distance (de la Higuera & Casacuberta 2000), but Lojban's rigid root-word
phonotactics make the feasible set finite and small (96,475 forms over both
default shapes), so the optimum is computed *exactly* by enumeration: every
candidate is scored, and pruning heuristics are deliberately absent.

The classic scorer has the same argmax shape — it also enumerates candidates
and maximizes a weighted sum of per-source scores. The two differ entirely in
the scoring function `sim`: letter-counting on Lojban-letter strings versus
phonetic alignment on precise IPA. Unlike the classic pipeline, no phonemic
distinction is coarsened away before scoring; the reduction to Lojban's small
sound inventory happens implicitly — and optimally — through the constraint
that the result must be a phonotactically valid gismu.

## Candidate set G

All phonotactically valid gismu word-forms: shape CCVCV with a permissible
initial pair, or CVCCV with a permissible medial pair, over the full Lojban
inventory (consonants `b c d f g j k l m n p r s t v x z`, vowels `a e i o u`).
Each enumerated form is additionally validated by the real morphology parser —
it must segment as a single word of kind gismu and render back to itself — so
the candidate set is exactly the set of well-formed gismu, not an
approximation of it.

Under the phonetic scorer the enumeration always ranges over the full letter
inventory (the classic scorer's default restriction to letters occurring in the
sources is overridden). The restriction is harmless under equality matching,
where an absent letter can never score, but unsound under graded phonetic
similarity: a candidate `b` can earn real credit against a source /p/ or /β/.

### Candidate pronunciation targets

Each candidate letter maps directly to a **pronunciation target**:

- `c` → [ʃ], `j` → [ʒ]; every other consonant and vowel maps to the IPA
  segment written with the same symbol (`x` → [x], `a` → [a], …). These are
  singleton targets with one concrete realization.
- `r` is the one free-variation target: it admits any of the consonantal
  rhotics `[r ɾ ɹ ʀ ɻ ʁ ɽ]` at zero cost, as required by CLL §3.2. Rhotic
  vowels `[ɚ ɝ]` are not realizations of `r`; they remain concrete vocalic
  nuclei that can participate in the ordinary one-to-two alignment operation.

Gismu shapes place `i`/`u` only in vowel positions, so glide targets never
arise in candidate scoring. The canonical display IPA remains deterministic
(and continues to render Lojban `r` as [r]), but it is not round-tripped to
construct scoring targets. Gismu stress is positional (penultimate) and is
ignored by the scorer (see Limitations).

## Sources

Each source is given as `LANG[:WEIGHT]:WORD`, where `WORD` is either
Lojban-letter text or a **broad phonemic IPA** transcription in `[ ... ]`
brackets. Weights are integers 1–999, supplied explicitly or via a preset
(`1985`, `1987`, `1994`, `1995`, `1999`, `evenly`, `ilmen6`, `ilmen8`,
`ilmen12`); internally they are scaled by 1/1000. Any positive rescaling of
the weights leaves the argmax unchanged, so weights need not be normalized.

Every source — regardless of scorer — is first resolved to a Lojban-letter
form: bracketed IPA goes through the stage-1 transliterator
(`docs/source-word-transliteration.md`), which also enforces the transcription
discipline that a bare schwa must be resolved to a full vowel. The phonetic
scorer then works from the pronunciation, not the transliteration:

- An IPA source is tokenized into **concrete IPA segments** (next section) and
  scored with its actual ALINE features — a concrete [ʁ] is not rewritten to
  [r], a source [ø] is scored as a front rounded mid vowel.
- A plain Lojban-letter source maps each letter to its canonical Lojban
  segment (the same letter→segment table used for candidates; a source-side
  `r` is the concrete trill [r]). Free variation exists only on the candidate
  side, where it expresses what the coined word *permits*; a source is a
  concrete observation.

Transcription discipline is shared with the classic pipeline: transcribe at
the phonemic level (apply the language's own neutralizations; no allophonic
detail, no morphophonemic abstraction), ignore tone, drop grammatical endings.
The transcription-quality problem is per source language and out of scope
here; the scorer takes the IPA strings as given.

### IPA tokenization

`tokenize_ipa_text` converts an IPA string into segment ids over a fixed
concrete inventory of ~150 segments: base consonants and vowels, affricates
(with or without tie bars), long vowels, contrastively aspirated plosives and
affricates (`pʰ`, `tsʰ`, `ʈʂʰ`, …), alveolo-palatals (`ɕ ʑ tɕ dʑ`), retroflex
affricates, `pf`, rounded glides (`ɥ ʍ`), laterals (`ɫ ɭ ʎ`), lax vowels
(`ɪ ʊ ʏ`), and r-coloured vowels (`ɚ ɝ`). Tokenization:

- normalizes to NFD, maps the IPA script ɡ (U+0261) to `g`, and strips
  affricate tie bars, so tied and untied spellings are the same segment;
- treats whitespace, syllable and phrase breaks, stress marks (ˈ ˌ), and tone
  letters as boundaries;
- skips combining diacritics and modifier letters it does not model (including
  vowel nasalization); aspiration survives only where the inventory has a
  dedicated aspirated segment, matched longest-first ahead of modifier
  skipping;
- rejects any segment not in the inventory as an input error
  (`UnsupportedSegment`) — unrecognized sounds are never skipped silently.

The concrete inventory must cover the union of the source languages'
phonologies; tests pin acceptance of every stage-1 transliterator base symbol
and spot-check transcriptions for all twelve preset languages.

## Similarity model: ALINE features

Per-segment similarity follows Kondrak's ALINE (Kondrak 2000; Kondrak 2002,
ch. 4), a feature-based model designed for cross-language cognate alignment —
a close cousin of the question asked here ("would a speaker of the source
language recognize their word in this root?").

Each segment carries twelve features, derived programmatically from its symbol.
Multivalued features take values in [0, 1] along phonetic scales:

- **place**: bilabial 1.0, labiodental 0.95, dental 0.9, alveolar 0.85,
  retroflex 0.8, palato-alveolar 0.75, alveolo-palatal 0.725 (midpoint of
  ALINE's adjacent positions), palatal 0.7, velar 0.6, uvular 0.5, pharyngeal
  0.3, glottal 0.1. Vowels take place −1.0, which makes any consonant–vowel
  substitution expensive.
- **manner**: stop 1.0, affricate 0.9, fricative 0.8, trill 0.7, tap 0.65,
  approximant 0.6, high vowel 0.4, mid vowel 0.2, low vowel 0.0.
- **high** (vowel height): high 1.0, mid 0.5, low 0.0; **back** (vowel
  backness): front 1.0, central 0.5, back 0.0.
- **syllabic**, **voice**, **nasal**, **lateral**, **retroflex**,
  **aspirated**, **round**, **long** are 0/1 flags.

Deliberate approximations, inherited from ALINE's feature inventory: dark ɫ is
featurally identical to alveolar l (no velarization feature); lax ɪ/ʊ/ʏ share
their tense counterparts' vowel features (no tenseness feature); the retroflex
flag stands in for r-colouring on ɚ/ɝ (no vowel-rhoticity feature).

The featural difference between segments a, b is a salience-weighted Manhattan
distance:

```
δ(a, b) = Σ_f  σ_f · |f(a) − f(b)|
```

summed over the consonant feature set — syllabic, place, manner, voice, nasal,
retroflex, lateral, aspirated — if either segment is a consonant, else the
vowel feature set — syllabic, nasal, retroflex, high, back, round, long.
Default saliences σ (Kondrak's, hand-tuned on cognate data): manner 50, place
40, voice 10, nasal 10, lateral 10, retroflex 10, syllabic 5, high 5, back 5,
round 5, aspirated 5, long 1.

From δ, the elementary alignment operation scores are:

```
sub(a, b)        = C_sub − δ(a, b) − V(a) − V(b)            (substitution / match)
exp(a | b, c)    = C_exp − δ(a, b) − δ(a, c) − V(a)
                   − max(V(b), V(c))                        (one segment ↔ two segments)
skip             = C_skip                                   (segment left unmatched)
```

where V(x) = C_vwl if x is a vowel, else 0 (vowels are less reliable evidence
than consonants). Defaults, again Kondrak's: C_sub = 35, C_exp = 45,
C_skip = −10, C_vwl = 10.

When an operand is a pronunciation target, each complete operation is
maximized jointly over its concrete realizations. Thus substitution maximizes
`C_sub − δ − V − V`; a one-to-two operation maximizes its entire `C_exp − δ −
δ − V − max(V,V)` expression. A single target aligned with two segments chooses
one realization for both distance terms, while two distinct target occurrences
choose independently. This operation-level rule also applies target-to-target,
including target self-similarity.

## Alignment regime: semi-global

This is the one place where the scorer deliberately departs from ALINE's
default. ALINE, following its cognate-matching purpose, uses *local* alignment
(Smith–Waterman): the score is that of the best-matching pair of substrings,
and segments outside the chosen window — in **either** string — cost nothing.
That is right for "do these two words share a root?" (and remains what vlacku
sound search uses), and wrong for averaging: a candidate with two well-matched
segments and three junk segments would tie with one that matches throughout,
long sources would stop separating candidates, and many candidates would
plateau on the same best window. Full global (Needleman–Wunsch) alignment
overcorrects the other way: a five-segment candidate against a nine-segment
source pays a fixed skip tax that systematically depresses long-worded
languages' scores.

Here the two strings play asymmetric roles, and the alignment regime reflects
that:

- **Candidate side — global.** The gismu is the artifact being coined; every
  one of its segments should be justified by the source. An unmatched candidate
  segment is noise a listener must swallow, so it is penalized (C_skip)
  wherever it occurs.
- **Source side — free flanks, penalized interior.** Truncated borrowing is
  normal: no five-segment root covers a nine-segment source word, and speakers
  recognize a word from a contiguous chunk of it. Skipping a source *prefix or
  suffix* is therefore free (or cheap — the C_flank parameter), while skipping
  source segments *interior* to the matched region still costs C_skip, so
  scattered cherry-picking does not score like a contiguous match.

Precisely: for candidate segments g₁ … g_m and source segments s₁ … s_n,
compute the table

```
S(0, 0) = 0
S(i, 0) = i · C_skip                          (unmatched candidate prefix)
S(0, j) = j · C_flank                         (skipped source prefix, flank rate)

S(i, j) = max of:
    S(i−1, j−1) + sub(gᵢ, sⱼ)
    S(i−1, j)   + C_skip                      (candidate segment unmatched)
    S(i,   j−1) + C_skip                      (source segment skipped, interior)
    S(i−1, j−2) + exp(gᵢ | sⱼ₋₁, sⱼ)          (j ≥ 2)
    S(i−2, j−1) + exp(sⱼ | gᵢ₋₁, gᵢ)          (i ≥ 2)

raw(g, s) = max over 0 ≤ j ≤ n of  [ S(m, j) + (n − j) · C_flank ]
```

There is no zero floor and no max-over-all-cells (both hallmarks of local
alignment); the candidate must be consumed in full.

The flank rate C_flank spans the whole regime space:

- `C_flank = 0` — **semi-global**, the default;
- `C_flank = C_skip` — full global (Needleman–Wunsch) alignment;
- values in between trade off how forgivable truncation is.

As an illustration, candidate `gunma` against English *government*
[ɡʌvərnmənt]:

```
candidate   ɡ   u   ·   ·   ·   n   m   a   ·  ·
source      ɡ   ʌ   v   ə   r   n   m   ə   n  t
            sub sub  interior   sub sub sub  flank
                     skips                   (free)
```

All five candidate segments are justified; the source pays for the interior
[vər] gap but not for its tail.

## Normalization and the total score

Per-source scores must be commensurable — each in [0, 1] with 1 meaning
"perfect" — or the language weights do not mean what they say. Define
self-similarity `self(x) = raw(x, x)` using the same concrete- or target-aware
dynamic program as `x` (which, with the defaults, is the sum of
identity substitution scores: C_sub − 2·C_vwl per vowel, C_sub per consonant —
all positive, so the identity diagonal is optimal and flanks are unused).

The default normalizer is **source-side** (*coverage semantics*):

```
score(g, s) = clamp( raw(g, s) / self(s),  0, 1 )
```

i.e. "what fraction of the source word survives, phonetically, in the
candidate". This is the direct analogue of the classic scorer's division of
the raw letter count by the source length, so the two scorers remain
comparable, and it keeps the incentive to represent *more* of a long source
rather than a token chunk of it.

Alternatives, selected by the `normalizer` parameter:

- *candidate-side* `raw / self(g)` — "how justified is the candidate"; note
  that it structurally favors vowel-heavy candidates (the vowel penalty
  deflates their self-similarity denominator), which is an artifact;
- *symmetric* `2·raw / (self(g) + self(s))` — ALINE's own normalization,
  appropriate when neither string is privileged.

The total score and the result are then:

```
total(g) = Σᵢ (wᵢ / 1000) · score(g, sᵢ)
gismu    = argmax over g ∈ G of total(g),  ties broken lexicographically
```

The reported per-source `raw-score` under the phonetic scorer is the
normalized similarity in [0, 1]; `weighted-score` multiplies it by wᵢ/1000.
The winner is the highest-ranked candidate without a collision; ranking,
collision filtering, `--show-collisions`, highlighting, and the top-N result
list (default 20, capped at 512, maintained with a bounded heap) work exactly
as with the classic scorer.

## Precomputed scoring tables

Scoring ~10⁵ candidates against every source is kept cheap by resolving all
per-operation work before the candidate loop:

- A **prepared target inventory** over the 22 candidate targets (17 consonants
  + 5 vowels) tabulates every target-to-target substitution and one-to-two
  operation, with realization-set maxima (the `r` target's seven rhotics)
  resolved at table-build time.
- Per source, a **prepared source** tabulates every target-to-segment
  substitution, target-to-source-pair expansion, and segment-to-target-pair
  expansion.

Every dynamic-programming transition in the candidate loop is then a
constant-time table lookup, over a three-row rolling scratch. Under the
default source-side normalizer the candidate's own self-similarity is not
needed and is skipped in the hot loop. An eager brute-force oracle
(re-deriving every score from first principles, including explicit enumeration
of `r` realizations) is kept in the test suite and must match the optimized
pipeline bit-for-bit for every normalizer.

## Parameters

Everything below is a tunable, in the same spirit as the language weights;
defaults are the starting point for empirical tuning, not conclusions. CLI
flags: `--c-sub`, `--c-exp`, `--c-skip`, `--c-vwl`, `--c-flank`,
`--normalizer`, and repeatable `--salience FEATURE=VALUE`.

| Parameter | Default | Meaning |
|---|---|---|
| wᵢ | preset per language | source weights (1–999) |
| σ_f | Kondrak's saliences (see above) | feature importance in δ |
| C_sub | 35 | substitution score ceiling |
| C_exp | 45 | expansion/compression score ceiling |
| C_skip | −10 | unmatched-segment penalty (candidate anywhere; source interior) |
| C_vwl | 10 | vowel evidence discount |
| C_flank | 0 | source prefix/suffix skip rate (0 = semi-global … C_skip = global) |
| normalizer | source-side | source-side / candidate-side / symmetric |

Validation enforces C_sub > 2·C_vwl (so identity self-similarity is positive),
C_skip ≤ 0, C_vwl ≥ 0, C_flank between C_skip and 0, and finite saliences ≥ 0.

The defaults C_sub/C_exp/C_skip/C_vwl and σ are Kondrak's published values,
hand-tuned for cognate identification. They are the canonical known-good set
for this family of models; later refinements in the literature are
learned-from-data (PMI-weighted alignment, sound-class models à la LexStat)
rather than better hand constants. The single most suspect default for *this*
purpose is C_vwl: with several-way averaging, vowels are often where sources
disagree most, and how much they count materially changes which candidate wins.

## Properties

- **Exactness.** The optimum over G is exact; no alignment or averaging
  heuristic intervenes between the inputs and the result. The only heuristic
  content in the whole algorithm is the parameter values.
- **Identity.** If some source is itself a valid gismu pronunciation, it scores
  1.0 against itself and (with C_flank = 0) any candidate containing a source
  contiguously scores that window's self-similarity fraction.
- **Graded ranking.** Scores are continuous; the top-N list is a genuine
  ranking rather than tie-break artifacts of a discrete count, which makes the
  runner-up list meaningful for human review.
- **No coarsening loss.** A source distinction Lojban cannot express still
  influences *which* Lojban form is nearest (e.g. /ø/ pulls toward candidates
  whose vowels are front, without ever being forced to pick `e` vs `i` before
  scoring).
- **Regime nesting.** For fixed inputs, raw scores satisfy local ≥ semi-global
  ≥ global, since each regime's feasible alignments contain the next's — a
  useful sanity check, exercised by tests.

## Limitations, gotchas, and open questions

- **Similarity is a proxy for recognizability.** Human word recognition is not
  position-uniform: onsets and stressed syllables carry disproportionate cue
  value. The scorer models neither — the tokenizer treats stress marks as
  plain boundaries. The natural extension — up-weighting segments in the
  source's stressed region and/or the word onset (gismu stress is
  deterministically penultimate) — is deliberately deferred; the phone-level
  model should be validated first.
- **Parameter sensitivity.** The "average" is only as principled as δ. Any
  empirical claim ("the medoid differs from the classic result in X% of
  cases") is really a claim about the parameter set and should be reported as
  such. Sensitivity analysis over C_vwl, C_flank, and the manner/place
  saliences should accompany any tuning effort.
- **Transcription-level sensitivity.** Scores inherit the quality of the
  phonemic transcriptions. A narrow transcription of one source and a broad one
  of another silently re-weights the languages. The same stage-1 discipline the
  classic pipeline demands (phonemic level, language-own neutralizations, no
  tone) applies unchanged.
- **Schwa must be resolved upstream.** The ALINE tokenizer can score a bare
  [ə] featurally, but every gimfi'i source also gets a Lojban-letter form via
  the stage-1 transliterator, which rejects it — so a source transcription
  must commit to the full vowel actually pronounced. (Sound search, which has
  no Lojban-letter stage, does accept [ə].)
- **Length asymmetry persists.** Even semi-global + coverage normalization caps
  a five-segment candidate's score against a much longer source (it can cover
  only so much). This is inherent to coining short roots from long words — the
  classic scorer has exactly the same trait (max raw 5 against an 8-letter
  source) — but it means effective language influence still interacts with
  typical word length; the normalizer parameter is the lever if this proves
  objectionable.
- **Clamping.** Negative raw scores clamp to 0, erasing gradient among very bad
  candidates. Irrelevant to the argmax, but "0.0" in reported per-source scores
  means "no phonetic case at all", not a fine-grained judgment.
- **Segment inventory coverage.** The tokenizer's segment inventory bounds
  which distinctions can matter at all. A feature present in the model but not
  in the inventory is silently vacuous for the sounds it would distinguish:
  for example, the nasal feature scores nasal consonants, but vowel
  nasalization is stripped at tokenization (there are no nasalized-vowel
  segments), so [ɑ̃] scores identically to [ɑ] — unlike the stage-1
  transliterator, which decomposes it into vowel + nasal consonant. Inventory
  and feature model must be audited together against the source-language
  union.
- **Expansion ops are 1↔2 only.** Diphthongs, affricates, and other contour
  segments are handled by the segment inventory plus the expansion operation;
  correspondences wider than two segments are not modeled.
- **Cross-concept scores are not comparable.** total(g) depends on the sources'
  lengths and mutual (dis)agreement; it ranks candidates *within* one coining,
  and is not a quality scale across different concepts.
- **Everything downstream is unchanged.** Collision checking against existing
  words, rafsi considerations, and shape preferences are orthogonal to the
  scorer and compose with it exactly as with the classic one.

## References

- G. Kondrak, *A New Algorithm for the Alignment of Phonetic Sequences*,
  NAACL 2000 — the ALINE model, its feature system, and the default parameters.
- G. Kondrak, *Algorithms for Language Reconstruction*, PhD thesis, University
  of Toronto, 2002, ch. 4 — extended treatment and parameter discussion.
- CLL (*The Complete Lojban Language*) §4.14 — the classical gismu creation
  algorithm.
- C. de la Higuera, F. Casacuberta, *Topology of strings: Median string is
  NP-complete*, Theoretical Computer Science 230 (2000) — hardness of the
  unconstrained generalized median, sidestepped here by the finite candidate
  set.
- `docs/source-word-transliteration.md` — the stage-1 IPA → Lojban-letter
  transliteration every source passes through.
