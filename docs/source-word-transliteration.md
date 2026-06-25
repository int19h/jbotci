# Source-word transliteration to Lojban phonemes

## Purpose

`gimfihi` builds candidate gismu from source words in several languages, following
the CLL §4.14 methodology: each source word is rendered into Lojban's sound
inventory, and candidate gismu are scored by how many letters they share with
those renderings. This document specifies how an arbitrary source word becomes a
string of Lojban phonemes.

The pipeline is split deliberately:

1. **Phonemic transcription (judgement, done by the caller / the model).** Turn
   the source word into a *broad phonemic* IPA transcription of how it is
   actually pronounced — the hard part, because it needs real knowledge of the
   language. Tone and stress are ignored; morphological endings are dropped.
2. **Mapping to Lojban (mechanical, done in product code).** Everything below is
   a deterministic function from IPA to Lojban phonemes: normalize the IPA,
   collapse affricates, and snap each remaining phoneme to the nearest Lojban
   sound. This stage involves *no* per-word judgement.

This document is the spec for stage 2 and the definition of what stage 1 is
allowed to emit. The docstring on the tool's `word` field (see the last section)
only has to tell the model to produce stage-1 IPA; it does not need to restate
the mapping, because the mapping lives here and in the code.

## Scope: the source languages

The mapping must accept anything that occurs in the phonologies of every language
referenced by any built-in weight preset. That is twelve languages:

| Code | Language | Notable contributions to the inventory |
|------|----------|----------------------------------------|
| `cmn` | Mandarin Chinese | retroflex & alveolo-palatal sibilants/affricates, /y/, aspiration-only stops, tones (dropped) |
| `hin` | Hindi (Hindustani) | dental vs. retroflex stops, four-way laryngeal series (aspirated/breathy), nasal vowels, Perso-Arabic /q x ɣ z/ |
| `eng` | English (GA ∪ RP) | /θ ð/, broad vowel space, rhotic vowels, diphthongs |
| `spa` | Spanish (Castilian ∪ Latin-American) | /θ/ (distinción), trill/tap, /ɲ ʎ ʝ/ |
| `rus` | Russian | **phonemic palatalized (soft) consonant series**, /ɨ/, vowel reduction |
| `ara` | Modern Standard Arabic | emphatics /tˤ dˤ sˤ ðˤ/, uvular /q/, pharyngeals /ħ ʕ/, /θ ð/, long vowels |
| `fra` | French | front-rounded /y ø œ/, nasal vowels /ɛ̃ ɑ̃ ɔ̃ œ̃/, uvular /ʁ/, /ɥ/ |
| `ben` | Bengali | dental vs. retroflex, breathy series, seven nasal vowels, /ɔ/ vs /o/, /æ/ |
| `por` | Portuguese (European ∪ Brazilian) | oral & nasal vowels, nasal diphthongs, /ɲ ʎ/, dorsal /ʁ/, /ɨ ɐ/ |
| `msa` | Malay | small core; loan /f v z ʃ x θ ð ɣ q/; final glottal stop |
| `jpn` | Japanese | unrounded back **/ɯ̟/**, moraic nasal /N/, geminates /Q/, long vowels, /ɸ ç/ |
| `deu` | German | front-rounded /y ø œ/, /pf ts/, /ç x/, tense/lax (length) pairs |

The guiding principle throughout, taken from CLL §4.14, is **consistency over
parsimony**: the same uniform map is applied to every language, and where a
language's analysis is contested we accept *more* phonemes rather than fewer,
because accepting a phoneme we did not strictly need is harmless, whereas
rejecting one that does occur is a bug.

## Target inventory: the Lojban phonemes

Everything maps onto this fixed set (IPA value in brackets):

**Consonants**

| Lojban | IPA | Notes |
|--------|-----|-------|
| `p` | /p/ | |
| `b` | /b/ | |
| `t` | /t/ | covers dental–alveolar |
| `d` | /d/ | covers dental–alveolar |
| `k` | /k/ | |
| `g` | /ɡ/ | always hard |
| `f` | /f/ | |
| `v` | /v/ | |
| `s` | /s/ | as in *sell*, never /z/ |
| `z` | /z/ | |
| `c` | /ʃ/ | "sh" |
| `j` | /ʒ/ | "zh" (measure) |
| `x` | /x/ | "kh" (Bach, jota) |
| `m` | /m/ | |
| `n` | /n/ | → [ŋ] before velars, allophonically |
| `l` | /l/ | may be syllabic |
| `r` | /r/ | any rhotic: trill, tap, or approximant; may be syllabic |
| `'` | /h/ | the apostrophe; a voiceless breathy sound |

**Vowels**: `a` /a/, `e` /ɛ~e/, `i` /i/, `o` /o~ɔ/, `u` /u/.

**Glides**: Lojban has no separate glide letters; `i` and `u` *are* the on-/off-glides
[j]/[w] when adjacent to another vowel (e.g. `ia` = [ja], `au` = [aw]).

There is deliberately **no `y`**. Lojban's `y` is /ə/ and is reserved for buffering
and grammar; the gismu algorithm forbids it in candidate roots. Consequently
schwa has no direct target and must be resolved to a full vowel (see the vowel
rule). `m`, `n`, `l`, `r` may stand as syllable nuclei, so syllabic consonants
need no inserted vowel.

The output of this process is a bare phoneme string used only for letter-scoring;
it does **not** have to be a phonotactically legal gismu (the scorer generates the
legal gismu itself), so no buffer vowels, cluster repairs, or terminator vowels
are added.

## Stage 2a — IPA normalization

Before snapping, reduce the broad IPA to bare base phonemes. Each rule below is a
rewrite; apply all of them.

### Suprasegmentals and length — drop entirely

| Input | Action | Reason |
|-------|--------|--------|
| stress `ˈ ˌ` | delete | Lojban stress is positional, not lexical here |
| syllable break `.`, ligature ‿ | delete | not phonemic |
| tone marks: letters `˥˦˧˨˩`, contour diacritics `◌̄ ◌́ ◌̌ ◌̂ ◌̀`, trailing tone digits | delete | tone is not represented |
| length `ː`, half-length `ˑ`, doubled vowels | delete the length (one vowel) | Lojban has no phonemic vowel length |
| consonant length / gemination `ː`, doubled consonants, Japanese moraic `Q` | collapse to a single consonant | no phonemic gemination |

### Secondary articulations — decompose or strip

| Input | Action | Example |
|-------|--------|---------|
| palatalization `ʲ` (incl. Russian soft consonants) | consonant **+ `i`-glide** onto the following vowel | rus. *нет* /nʲet/ → `niet`; *тётя* /tʲotʲa/ → `tiotia` |
| labialization `ʷ` | consonant **+ `u`-glide** | /kʷa/ → `kua` |
| aspiration `ʰ`, `ʱ`/breathy `◌̤`, creaky `◌̰` | strip (snap the base) | hin. /pʰal/ → `pal`, /bʱ/ → `b` |
| pharyngealization / emphasis `ˤ`, `◌̴`, velarization | strip (snap the base) | ara. /sˤ/ → `s`, /tˤ/ → `t`, /ðˤ/ → `z` |
| dental/apical/laminal/advanced/retracted place diacritics `◌̪ ◌̺ ◌̻ ◌̟ ◌̠` | ignore (snap the base) | /t̪/ → `t` |
| rhoticity `◌˞` and r-coloured vowels `ɚ ɝ` | vowel **+ `r`** | eng. *letter* /lɛtɚ/ → `leter` |
| explicit voicing `◌̥ ◌̬` | snap to the voiceless/voiced counterpart | |

### Vowel quality diacritics — ignore, snap the base symbol

Raised `◌̝`, lowered `◌̞`, advanced `◌̟`, retracted `◌̠`, centralized `◌̈`,
mid-centralized `◌̽`, more/less rounded `◌̹ ◌̜`, nasalized-consonant marks, etc. all
collapse to their base vowel before the vowel rule runs. (Nasalization on a
*vowel* is not ignored — see below.)

### Nasalized vowels — decompose to oral vowel + nasal consonant

A nasal vowel becomes its oral counterpart (snapped by the vowel rule) followed by
a nasal consonant whose place assimilates to what follows:

- **`m`** before a following labial (`p b m f v`),
- **`n`** otherwise (and `n` already surfaces as [ŋ] before velars).

Nasal diphthongs decompose the same way, keeping the off-glide:

| Input | Output | Example |
|-------|--------|---------|
| fra. /ɔ̃/ | `on` | *bon* → `bon` |
| fra. /ɛ̃/ | `en` | *vin* → `ven` |
| fra. /ɑ̃/ | `an` | *blanc* → `blan` |
| fra. /œ̃/ | `en` | *brun* → `bren` |
| por. /ɐ̃w̃/ | `aun` | *mão* → `maun` |
| por. /ɐ̃j̃/ | `ain` | *mãe* → `main` |
| hin. /ɑ̃ː/ | `an` | |

### Other segments

| Input | Action | Reason |
|-------|--------|--------|
| glottal stop `ʔ` | delete | no Lojban segment; leaves the vowels in contact |
| ʿayn `ʕ` (voiced pharyngeal) | delete | no approximation; it mainly colours adjacent vowels |
| ejective `ʼ` | strip the ejection (plain stop) | not in the source set, but be safe |
| affricate tie-bar `◌͡◌` | treat as an affricate (next section) | |

## Stage 2b — affricate collapse

Following CLL §4.14, an affricate made of a stop plus its matching fricative is
**simplified to the fricative**; then the fricative snaps as usual. This is the
single rule behind several rows of the consonant table:

| Affricate | → fricative | → Lojban |
|-----------|-------------|----------|
| /t͡ʃ/ | /ʃ/ | `c` |
| /d͡ʒ/ | /ʒ/ | `j` |
| /t͡ɕ/ | /ɕ/ | `c` |
| /d͡ʑ/ | /ʑ/ | `j` |
| /ʈ͡ʂ/ | /ʂ/ | `c` |
| /ɖ͡ʐ/ | /ʐ/ | `j` |
| /t͡s/ | /s/ | `s` |
| /d͡z/ | /z/ | `z` |
| /p͡f/ | /f/ | `f` |

Aspirated or breathy affricates (Mandarin /t͡sʰ t͡ɕʰ ʈ͡ʂʰ/, Hindi/Bengali
/t͡ʃʰ d͡ʒʱ/) first lose the laryngeal feature, then collapse identically.

## Stage 2c — consonant snapping

Every consonant phoneme that survives normalization maps as follows. The table is
grouped by Lojban target; the rationale column gives the principle.

| Lojban | IPA sources | Rationale |
|--------|-------------|-----------|
| `p` | p | identity |
| `b` | b, β | identity; bilabial fricative [β] is an allophone of /b/ |
| `t` | t, t̪, ʈ, tˤ | voiceless coronal stop; dental/alveolar/retroflex/emphatic all neutralize (Lojban has one coronal stop) |
| `d` | d, d̪, ɖ, dˤ | voiced coronal stop; same neutralization |
| `k` | k, q | voiceless dorsal stop; uvular /q/ has no Lojban target and the velar is nearest |
| `g` | ɡ | identity |
| `f` | f, ɸ | voiceless labial fricative; bilabial [ɸ] (Japanese) → labiodental |
| `v` | v, ʋ | voiced labial fricative; Hindi /ʋ/ is the v/w phoneme |
| `s` | s, θ, sˤ | voiceless coronal fricative; **θ → s** (matches Spanish *seseo*, and keeps fricative manner; alt. `t`) |
| `z` | z, ð, ðˤ | voiced coronal fricative; **ð → z** (parallel to θ; alt. `d`). Spanish/Portuguese [ð] as an allophone of /d/ should be transcribed /d/ → `d` |
| `c` | ʃ, ɕ, ʂ | voiceless postalveolar/alveolo-palatal/retroflex sibilant — all "sh-like" → `c` |
| `j` | ʒ, ʑ, ʐ | voiced counterparts of the above → `j` |
| `x` | x, χ, ɣ, ç | dorsal fricatives; Lojban `x` is the only one, so voicing (ɣ) and exact place (uvular χ, palatal ç) neutralize. ç as an allophone of /h/ (Japanese *hi*) is transcribed /h/ → `'` |
| `m` | m, ɱ | bilabial nasal |
| `n` | n, n̪, ŋ, ɳ, ɴ, moraic N | every non-labial nasal → `n` (which is [ŋ] before velars anyway); the palatal /ɲ/ is handled as `n` + `i`-glide |
| `l` | l, ɫ, ɭ | lateral; dark/retroflex variants neutralize; palatal /ʎ/ → `l` + `i`-glide |
| `r` | r, ɾ, ɹ, ɻ, ʀ, ʁ, ɽ | any rhotic — trill, tap, approximant, uvular, retroflex flap. Uvular /ʁ ʀ/ are the **rhotic** of French/German/Portuguese, so → `r`, not `x` |
| `'` | h, ɦ, ħ | glottal/pharyngeal "h"; voicing (ɦ) and pharyngeal constriction (ħ, Arabic *ḥ*) neutralize (alt. for ħ: `x`) |
| `i`-glide | j, ʝ, ɲ→nj, ʎ→lj, ɥ | palatal approximants and the palatal consonants' glide. Spanish /ʝ/ (*yo*) → `i`-glide; Rioplatense [ʒ] → `j` |
| `u`-glide | w | labiovelar approximant |
| *(deleted)* | ʔ, ʕ, Q | glottal stop, ʿayn, gemination marker — no target |

Notes on the palatal consonants: /ɲ/ and /ʎ/ are palatalized /n/ and /l/, so by the
palatalization rule they become `n`/`l` **+ `i`-glide** onto the next vowel —
spa. *español* /espaɲol/ → `espaniol`, por. *filho* /fiʎu/ → `filiu`. With no
following vowel they reduce to bare `n`/`l`.

## Stage 2d — vowel snapping

Lojban's five vowels sit at the periphery of the vowel space: front-unrounded
`i e`, back-rounded `o u`, and open `a`. Snap every (oral, de-diacriticked) vowel
to the nearest of these by **height** and **acoustic frontness**:

- **Height** picks the row: close / near-close → `i`/`u`; mid (close-mid &
  open-mid) → `e`/`o`; open → **`a`** (the only open vowel — all open qualities,
  front to back, rounded or not, land here).
- **Frontness** picks the column for the close & mid rows: **`i`/`e`** for front
  vowels *and* central-unrounded vowels; **`u`/`o`** for back vowels *and*
  central-rounded vowels.

Because Lojban has no front-rounded or back-unrounded vowels, one feature has to
give. We keep **tongue position (≈ F2)** and discard the lip-rounding mismatch:

- **Front-rounded → front-unrounded**: `y → i`, `ø → e`, `œ → e` (and the glide
  `ɥ → i`-glide). These have a high, front F2, so they are acoustically nearest
  `i`/`e`; this is also the usual historical repair (cf. Yiddish *über → iber*,
  *schön → sheyn*; Greek *y → i*).
- **Back-unrounded → back**: `ɯ → u`, `ɤ → o`. Japanese /ɯ̟/ ("u") is back, so it
  must land on `u`, not `i` — this is exactly why frontness (not rounding) is the
  deciding feature.

Worked tabulation of the full union:

| Lojban | IPA sources |
|--------|-------------|
| `i` | i, iː, ɪ, ɨ, y, yː, ʏ |
| `u` | u, uː, ʊ, ɯ, ʉ |
| `e` | e, eː, ɛ, ɛː, ø, øː, œ, ɘ, ɜ, ɜː |
| `o` | o, oː, ɔ, ɔː, ɒ, ɤ, ɵ |
| `a` | a, aː, ä, æ, ɐ, ɑ, ɑː, ʌ |

(English /ʌ/ — *cup* — is open central [ɐ] in modern GA/RP, hence `a`. Open back
rounded /ɒ/ — RP *lot* — keeps its rounding and goes to `o`; the unrounded GA
equivalent /ɑ/ goes to `a`, so *lot* is `lot` for an RP transcription and `lat`
for a GA one. Both are correct for their variety.)

### The schwa /ə/

`ə` is the one vowel we **forbid as input**. It sits dead-center with neutral
rounding, so the feature that decides `i/e` vs `u/o` — rounding bias on a central
vowel — is exactly the information it lacks; any snap would be arbitrary. The
near-schwa central vowels, which *do* carry that bias, already resolve cleanly
under the rule above (`ɘ → e`, `ɵ → o`, `ɨ → i`, `ʉ → u`, `ɐ → a`).

So stage 1 must commit: when a syllable would be transcribed `/ə/`, choose the
near-schwa vowel that matches its actual realization in the source word (almost
every language biases its schwa — open in one, fronter in another). For example
English *sofa* ends in [ɐ] → `a`; the first vowel of *about* is [ɐ] → `a`; a
clearly front reduced vowel → `e`. If the realization is genuinely indeterminate,
default to `a` (the most central, neutral Lojban vowel). The product code may
accept a bare `ə` defensively and apply that `a` default, but the intent is that
stage 1 resolves it.

### Vowel sequences and glides

A high vowel adjacent to another vowel is the glide: `i` before/after a vowel is
[j], `u` is [w]. Falling diphthongs therefore fall out as vowel + glide: `aɪ → ai`,
`aʊ → au`, `ɔɪ → oi`, `eɪ → ei`, `oʊ → ou`; rising ones as glide + vowel: `ja → ia`,
`wa → ua`. A glide identical to the vowel it lands on collapses (`i`-glide + `i`
→ `i`), which is why the rare /ɥi/ (fra. *huit*) reduces cleanly. Sequences of
two non-high vowels simply sit adjacent in the scoring string.

## Per-language sanity checks

A few end-to-end renderings to confirm the rules compose (source → IPA → Lojban):

- cmn. 用心 → /jʊŋɕin/ → `iuncin` (j→`i`-glide, ʊ→`u`, ŋ→`n`, ɕ→`c`, i→`i`, n→`n`)
- cmn. 需 → /ɕy/ → `ci` (ɕ→`c`, y→`i`)
- eng. *cat* → /kæt/ → `kat`
- spa. *gato*, drop the -o ending → /ɡat/ → `gat`
- rus. *спасибо* → /spasʲibə/ → `spasibo` (sʲ→`si`-glide onto i collapses to `si`; final ə→`o` by its [ɐ~o] quality, or `a`)
- ara. *kitāb* → /kitaːb/ → `kitab` (drop length)
- ara. *ḥasan* → /ħasan/ → `'asan`
- fra. *bon* → /bɔ̃/ → `bon`; *tu* → /ty/ → `ti`
- deu. *schön* → /ʃøːn/ → `cen`; *über* → /yːbɐ/ → `iba`
- jpn. *sushi* → /sɯɕi/ → `suci`
- hin. *cāy* (tea) → /t͡ʃaːj/ → `cai` (affricate→`c`, drop length, j→`i`-glide)
- por. *pão* → /pɐ̃w̃/ → `paun`
- ben. *bhālo* → /bʱalo/ → `balo` (drop breathiness)

## What the tool docstring should say

The `word` field only needs to elicit stage 1 and name the target sound space; it
must *not* try to restate the mapping. Something like:

> A word for this concept in the source language. Give a **broad phonemic IPA
> transcription** of how it is pronounced — its sounds, not its spelling and not
> an existing Lojban word. Ignore tone and stress, and drop grammatical endings
> (e.g. Spanish noun -o/-a). Do not use the schwa `ə`; instead pick the nearest
> full vowel to how the reduced vowel is actually pronounced. We map your IPA onto
> Lojban's sound inventory automatically — `a e i o u`; `b d f g k l m n p r s t v
> z`; `c`=ʃ, `j`=ʒ, `x`=x, `'`=h — so you do not need to know Lojban spelling.

The full inventory of accepted IPA symbols and their snaps is this document; the
code implements it. The model is free to use any standard IPA symbol — anything in
the union inventory maps as tabulated above, and the normalization rules absorb
diacritics, length, nasalization, and affricates that are not listed explicitly.
