# Phonetic-medoid gismu creation

## Scope and motivation

This document specifies an algorithm for coining new Lojban root words (gismu)
from weighted source words in several languages. It is a successor to the
classical algorithm of CLL §4.14 and is specified at the algorithm level, in
implementation-neutral terms, so that it can be discussed and evaluated on its
own. It is intended for coining *new* roots; re-deriving the official gismu is
out of scope (though instructive as an experiment).

The classical algorithm works in two steps:

1. **Coarsen**: each source word is transliterated into Lojban's small sound
   inventory (17 consonants, 5 vowels) — every phonemic distinction that Lojban
   does not make is discarded per word, up front.
2. **Average**: candidate gismu are scored against the coarsened strings by a
   letter-count rule (length of the longest common subsequence if ≥ 3, or 2 for
   an adjacency-constrained two-letter match, else 0), divided by the source
   length, weighted by language, and summed.

The coarsening step destroys information *before* the averaging step can use
it. A source /pʰ/ and a source /b/ both become indistinguishable from Lojban
`p`/`b` at best crudely; a source /ø/ must commit to `e` or `i` before any
candidate is ever considered; a candidate letter that is merely *near* a source
sound (rather than equal to its coarsened image) scores zero.

The algorithm specified here inverts the order: sources are kept as
full-precision phonemic IPA, and the "averaging" is performed directly in IPA
space, with the coarsening happening implicitly — and optimally — through the
constraint that the result must be a phonotactically valid gismu.

## The key reframing: no "average object" is needed

A naive reading of "average first, coarsen later" calls for constructing an
averaged IPA string (multiple-sequence alignment, per-column feature centroids)
and then snapping it into a valid gismu. Every step of that pipeline leaks:
multiple alignment is heuristic and order-dependent; feature centroids of
distant sounds land on phonemes that no source contains; and the final
snap-to-valid-shape step discards whatever optimality the average had.

Instead, observe that the output is *constrained* to a small finite set — the
phonotactically valid gismu forms. "The average of the sources, expressed as a
gismu" can therefore be defined directly as:

> **the valid gismu whose own pronunciation is maximally similar, on a
> weighted average, to the source pronunciations.**

Formally, given sources as concrete IPA segment strings s₁ … s_k with weights
w₁ … w_k, and G the set of valid candidate gismu with Lojban pronunciation
target sequences targets(g):

```
gismu = argmax over g ∈ G of  Σᵢ wᵢ · sim(targets(g), sᵢ)
```

In the string-averaging literature this object is the **generalized median
(Steiner/consensus string) constrained to a feasible set**. Computing a
generalized median over free-form strings is NP-hard even for plain edit
distance (de la Higuera & Casacuberta 2000), but the gismu shape constraint
makes the feasible set finite and small (on the order of 10⁵ forms), so the
optimum is computed *exactly* by enumeration. This is a genuinely pleasant
property of Lojban's rigid root-word phonotactics: the intractable part of the
problem is dissolved by the constraint rather than approximated around it.

Note that the classical algorithm already has this argmax shape — it enumerates
candidates and maximizes a weighted sum of per-source scores. The change is
entirely in the scoring function `sim`: from letter-counting on coarsened
strings to phonetic alignment on precise IPA.

## Components

### Candidate set G

All phonotactically valid gismu word-forms: shape CCVCV with a permissible
initial pair, or CVCCV with a permissible medial pair, over the full Lojban
inventory (consonants `b c d f g j k l m n p r s t v x z`, vowels `a e i o u`).

The full inventory matters. The classical algorithm restricts candidate letters
to those occurring in the coarsened sources — harmless there, because a letter
absent from every source can never score under equality matching. Under graded
phonetic similarity that restriction is unsound: a candidate `b` can earn real
credit against a source /p/ or /β/ that coarsens to something else, so the
enumeration must range over the whole alphabet.

Each candidate is converted directly from parsed Lojban phonemes to a sequence
of pronunciation targets. Most targets have one concrete realization (`c` =
[ʃ], `j` = [ʒ], `x` = [x], and `i`/`u` glides = [j]/[w]). Lojban `r` is the
one free-variation target in this model and admits any of
`[r ɾ ɹ ʀ ɻ ʁ ɽ]` at zero cost, as required by CLL §3.2. Rhotic vowels
`[ɚ ɝ]` are not realizations of bare `r`; they remain concrete vocalic nuclei
that can participate in the ordinary one-to-two alignment operation.

The canonical display IPA remains deterministic (and continues to render
Lojban `r` as [r]), but it is not round-tripped to construct scoring targets.
Stress is positional (penultimate) and, in this version of the algorithm,
ignored by the scorer (see Limitations).

### Source representation

Each source is a **broad phonemic IPA** transcription of the source word plus a
positive weight. Transcription discipline matters and is shared with the
classical pipeline: transcribe at the phonemic level (apply the language's own
neutralizations; no allophonic detail, no morphophonemic abstraction), ignore
tone, drop grammatical endings. The transcription-quality problem is per
source language and out of scope here; this specification takes the IPA strings
as given. Stress marks may be present but are ignored by this version of the
scorer.

Sources are tokenized into **concrete IPA segments**: base symbols plus a
defined set of multi-character segments (affricates with or without tie bars,
long vowels), with suprasegmentals and unmodeled diacritics skipped. Their
actual ALINE features are preserved: for example, concrete [ʁ] is not globally
rewritten to [r]. Candidates are target sequences whose realization sets
contain concrete IPA segments. The concrete inventory must cover the union of
the source languages' phonologies; an unrecognized segment is an input error,
not something to skip silently.

### Similarity model: ALINE features

Per-segment similarity follows Kondrak's ALINE (Kondrak 2000; Kondrak 2002,
ch. 4), a feature-based model designed for cross-language cognate alignment —
a close cousin of the question asked here ("would a speaker of the source
language recognize their word in this root?").

Each segment decomposes into articulatory features. Multivalued features take
values in [0, 1] along phonetic scales, e.g. place: bilabial 1.0, labiodental
0.95, dental 0.9, alveolar 0.85, retroflex 0.8, palato-alveolar 0.75, palatal
0.7, velar 0.6, uvular 0.5, pharyngeal 0.3, glottal 0.1; manner: stop 1.0,
affricate 0.9, fricative 0.8, trill 0.7, tap 0.65, approximant 0.6, high vowel
0.4, mid vowel 0.2, low vowel 0.0. Binary features (voice, nasal, lateral,
retroflex, aspirated, round, long) are 0/1. Vowels are compared on height,
backness, roundness, and length instead of place and manner.

The featural difference between segments a, b is a salience-weighted Manhattan
distance:

```
δ(a, b) = Σ_f  σ_f · |f(a) − f(b)|
```

summed over the consonant feature set if either segment is a consonant, else
the vowel feature set. Default saliences σ (Kondrak's, hand-tuned on cognate
data): manner 50, place 40, voice 10, nasal 10, lateral 10, retroflex 10,
syllabic 5, high 5, back 5, round 5, aspirated 5, long 1.

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
including target self-similarity. The maxima are resolved when request tables
are prepared, so enumerating candidates still performs one constant-time table
lookup per dynamic-programming transition.

### Alignment regime: semi-global

This is the one place where this algorithm deliberately departs from ALINE's
default. ALINE, following its cognate-matching purpose, uses *local* alignment
(Smith–Waterman): the score is that of the best-matching pair of substrings,
and segments outside the chosen window — in **either** string — cost nothing.
That is right for "do these two words share a root?" and for sound search, and
wrong for averaging, for reasons given below.

Here the two strings play asymmetric roles, and the alignment regime should
reflect that:

- **Candidate side — global.** The gismu is the artifact being coined; every
  one of its segments should be justified by the source. An unmatched candidate
  segment is noise a listener must swallow, so it is penalized (C_skip)
  wherever it occurs.
- **Source side — free flanks, penalized interior.** Truncated borrowing is
  normal: no five-segment root covers a nine-segment source word, and speakers
  recognize a word from a contiguous chunk of it. Skipping a source *prefix or
  suffix* is therefore free (or cheap — parameterized below), while skipping
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

The flank rate C_flank is a parameter that spans the whole regime space:

- `C_flank = 0` — **semi-global**, the default specified here;
- `C_flank = C_skip` — full global (Needleman–Wunsch) alignment;
- values in between trade off how forgivable truncation is.

Local alignment is thus not needed as a separate mode for this algorithm at
all (it remains the right choice for sound *search*).

### Why not local, why not global

**Local** fails the averaging use in three ways:

1. *Window matching.* Against a long source (say English *government*,
   [ɡʌvərnmənt]), a candidate matching [ɡʌv] and a candidate matching [mən]
   both "found a good window"; local alignment is indifferent to which part and
   how much of either word is represented. Worse, unmatched **candidate**
   segments are also free: a gismu with two well-matched segments and three
   junk segments ties with one that matches throughout.
2. *Weight distortion.* Under the usual symmetric normalization, a long
   source's large self-similarity caps every candidate's score against it —
   uniformly. The source stops separating candidates and its language's
   *effective* weight in the sum Σ wᵢ·simᵢ silently shrinks below its nominal
   wᵢ.
3. *Plateaus.* Many candidates achieve the same best window, reintroducing the
   tie-plateau problem that graded scoring is meant to fix.

**Global** fixes all three but overcorrects: a five-segment candidate against a
nine-segment source forces at least four skips — a fixed tax levied identically
on *every* candidate. Per source that is a harmless uniform shift; across
sources it again means long-worded languages contribute systematically
depressed scores, the same weight distortion arrived at from the other side.

**Semi-global** charges the candidate for its own noise and the source only for
interior gaps, which is the recognizability structure of borrowing. As an
illustration, candidate `gunma` [ɡun.ma] against [ɡʌvərnmənt]:

```
candidate   ɡ   u   ·   ·   ·   n   m   a   ·  ·
source      ɡ   ʌ   v   ə   r   n   m   ə   n  t
            sub sub  interior   sub sub sub  flank
                     skips                   (free)
```

All five candidate segments are justified; the source pays for the interior
[vər] gap but not for its tail.

### Normalization and the total score

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
candidate". This is the direct analogue of the classical algorithm's division
of the raw letter count by the source length, so the two scorers remain
comparable, and it keeps the incentive to represent *more* of a long source
rather than a token chunk of it.

Alternatives, available as a parameter:

- *candidate-side* `raw / self(g)` — "how justified is the candidate"; note
  that it structurally favors vowel-heavy candidates (the vowel penalty
  deflates their self-similarity denominator), which is an artifact;
- *symmetric* `2·raw / (self(g) + self(s))` — ALINE's own normalization,
  appropriate when neither string is privileged.

The total score and the result are then:

```
total(g) = Σᵢ wᵢ · score(g, sᵢ)
gismu    = argmax over g ∈ G of total(g),  ties broken lexicographically
```

Any positive rescaling of the weights leaves the argmax unchanged, so weights
need not be normalized. All candidates are scored — the space is small enough
that exactness is cheap, and pruning heuristics are explicitly rejected.

## Parameters

Everything below is a tunable, in the same spirit as the language weights;
defaults are the starting point for empirical tuning, not conclusions.

| Parameter | Default | Meaning |
|---|---|---|
| wᵢ | preset per language | source weights |
| σ_f | Kondrak's saliences (see above) | feature importance in δ |
| C_sub | 35 | substitution score ceiling |
| C_exp | 45 | expansion/compression score ceiling |
| C_skip | −10 | unmatched-segment penalty (candidate anywhere; source interior) |
| C_vwl | 10 | vowel evidence discount |
| C_flank | 0 | source prefix/suffix skip rate (0 = semi-global … C_skip = global) |
| normalizer | source-side | source-side / candidate-side / symmetric |

The defaults C_sub/C_exp/C_skip/C_vwl and σ are Kondrak's published values,
hand-tuned for cognate identification. They are the canonical known-good set
for this family of models; later refinements in the literature are
learned-from-data (PMI-weighted alignment, sound-class models à la LexStat)
rather than better hand constants. The single most suspect default for *this*
purpose is C_vwl: with several-way averaging, vowels are often where sources
disagree most, and how much they count materially changes which candidate wins.

## Ferment regression

The issue #587 reference run uses the Ilmen12 weights and default gismu shapes,
requests 160 results, and supplies these concrete observations: Mandarin
`[fat͡ɕjɑʊ]`, English `[fɚmɛnt]`, Spanish `[feɾment]`, Hindi `[kɪɳʋan]`,
Arabic `[taxamːur]`, Bengali `[ɡãd͡ʒɔn]`, Russian `[fʲermʲent]`, Portuguese
`[feʁmẽt]`, Malay `[pɘnapaian]`, Japanese `[hakːoː]`, German `[ɡɛːʁ]`, and
French `[fɛʁmɑ̃t]`. With collision filtering enabled, the run examines 96,475
valid candidates and retains 82,567 after filtering. Its top three are `ferme`,
`farme`, and `ferma`. The winner `ferme` is also the first r-bearing result, at
rank 1; this is the intended meaningful placement that accepting every
consonantal Lojban-r realization restores.

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
  useful sanity check for implementations.

## Limitations, gotchas, and open questions

- **Similarity is a proxy for recognizability.** Human word recognition is not
  position-uniform: onsets and stressed syllables carry disproportionate cue
  value. This version models neither. The natural extension — up-weighting
  segments in the source's stressed region and/or the word onset (gismu stress
  is deterministically penultimate) — is deliberately deferred; the phone-level
  model should be validated first.
- **Parameter sensitivity.** The "average" is only as principled as δ. Any
  empirical claim ("the medoid differs from the classical result in X% of
  cases") is really a claim about the parameter set and should be reported as
  such. Sensitivity analysis over C_vwl, C_flank, and the manner/place
  saliences should accompany any tuning effort.
- **Transcription-level sensitivity.** Scores inherit the quality of the
  phonemic transcriptions. A narrow transcription of one source and a broad one
  of another silently re-weights the languages. The same stage-1 discipline the
  classical pipeline demands (phonemic level, language-own neutralizations, no
  tone) applies unchanged.
- **Length asymmetry persists.** Even semi-global + coverage normalization caps
  a five-segment candidate's score against a much longer source (it can cover
  only so much). This is inherent to coining short roots from long words — the
  classical algorithm has exactly the same trait (max raw 5 against an 8-letter
  source) — but it means effective language influence still interacts with
  typical word length; the normalizer parameter is the lever if this proves
  objectionable.
- **Clamping.** Negative raw scores clamp to 0, erasing gradient among very bad
  candidates. Irrelevant to the argmax, but "0.0" in reported per-source scores
  means "no phonetic case at all", not a fine-grained judgment.
- **Segment inventory coverage.** The tokenizer's segment inventory bounds
  which distinctions can matter at all. Features present in the model but not
  in the inventory (e.g. aspiration, if ʰ is stripped at tokenization) are
  silently vacuous — inventory and feature model must be audited together
  against the source-language union.
- **Expansion ops are 1↔2 only.** Diphthongs, affricates, and other contour
  segments are handled by the segment inventory plus the expansion operation;
  correspondences wider than two segments are not modeled.
- **Cross-concept scores are not comparable.** total(g) depends on the sources'
  lengths and mutual (dis)agreement; it ranks candidates *within* one coining,
  and is not a quality scale across different concepts.
- **Everything downstream is unchanged.** Collision checking against existing
  words, rafsi considerations, and shape preferences are orthogonal to the
  scorer and compose with it exactly as with the classical one.

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
