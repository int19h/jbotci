# Source-word transliteration to Lojban phonemes

## Purpose

`gimfihi` builds candidate gismu from source words in several languages, following
the CLL §4.14 methodology: each source word is rendered into Lojban's sound
inventory, and candidate gismu are scored by how many letters they share with
those renderings. This document specifies how an arbitrary source word becomes a
string of Lojban phonemes.

The pipeline is split deliberately:

1. Phonetic transcription (the caller or model). Transcribe how the source word sounds, within the supported IPA inventory.
   An allophone is a positional form of a sound.
   Use the narrowest transcription that the inventory permits.
   Select the allophones that occur in each position.
   Apply vowel reduction, final devoicing, and consonant assimilation.

   For example, assimilation can change `nb` to `mb` or `kz` to `gz`.
   Remove grammatical endings, such as Spanish noun `-o` in *gato* → `ɡat`.
   Resolve schwa as described in "Level of representation".
   Tone and stress do not affect the scoring letters.
2. Mapping to Lojban (product code). The code normalizes IPA, collapses affricates, and maps the remaining sounds to Lojban scoring letters.
   This stage applies the same rules to each transcription.

This document is the spec for stage 2 and the definition of what stage 1 is
allowed to emit. The docstring on the tool's `word` field (see the last section)
only has to tell the model to produce stage-1 IPA; it does not need to restate
the mapping, because the mapping lives here and in the code.

## Scope: the source languages

The built-in weight presets use twelve source languages.
The tables describe their supported IPA symbols.
The "Input limits" section lists symbols and notation that the classic mapper rejects.

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

**Vowels**: `a` /a/, `e` /ɛ~e/, `i` /i/, `o` /o~ɔ/, `u` /u/.

**Glides**: Lojban has no separate glide letters; `i` and `u` *are* the on-/off-glides
[j]/[w] when adjacent to another vowel (e.g. `ia` = [ja], `au` = [aw]).

There is deliberately **no `y`**. Lojban's `y` is /ə/ and is reserved for buffering
and grammar; the gismu algorithm forbids it in candidate roots. Consequently
schwa has no direct target and must be resolved to a full vowel (see the vowel
rule). `m`, `n`, `l`, `r` may stand as syllable nuclei, so syllabic consonants
need no inserted vowel.

There is also no `'` (the apostrophe, Lojban /h/) in the emitted set, and for an
instructive reason. A gismu candidate is built only from consonants and vowels, so
it can never contain `'`; an apostrophe in a source word would therefore match no
candidate, yet would still count in the length the raw match is divided by — CLL
§4.14 divides the score by *"the length of the source-language word in its
Lojbanized form,"* and `score_source` divides by `source.word.chars().count()` — so
it could only dilute the score. The `[h]`-type sounds are therefore **not** sent to
`'`; they go to **`x`**, along with every other fricative at or behind the velum
(`/x ɣ χ ħ h ɦ/`, see the consonant table). This is exactly what the official
gismu do: *derxi* "heap" carries its `x` straight from English *heap* `/hip/` →
`xip`, and Arabic renders `ḥ`/`x` the same way. (The input validator **rejects**
`'` outright — it is not a gismu-scoring letter — so a stray one cannot slip in and
dilute a score.)

The output of this process is a bare phoneme string used only for letter-scoring;
it does **not** have to be a phonotactically legal gismu (the scorer generates the
legal gismu itself), so no buffer vowels, cluster repairs, or terminator vowels
are added.

## Stage 2a — IPA normalization

Before snapping, normalize the phonetic IPA to base symbols.
The code applies the normalization rules in this section.

### Suprasegmentals and length — drop entirely

| Input | Action | Reason |
|-------|--------|--------|
| stress `ˈ ˌ` | delete | Lojban stress is positional, not lexical here |
| syllable break `.` | delete | syllable divisions do not affect scoring |
| tone marks: letters `˥˦˧˨˩`, contour diacritics `◌̄ ◌́ ◌̌ ◌̂ ◌̀` | delete | tone is not represented |
| length `ː`, half-length `ˑ` | remove the length mark | Lojban has no phonemic vowel length |
| consonant length / gemination `ː` | remove the length mark | no phonemic gemination |

### Secondary articulations — decompose or strip

| Input | Action | Example |
|-------|--------|---------|
| palatalization `ʲ` (incl. Russian soft consonants) | consonant + `i` | rus. *нет* /nʲet/ → `niet`; *тётя* /tʲotʲa/ → `tiotia` |
| labialization `ʷ` | consonant **+ `u`-glide** | /kʷa/ → `kua` |
| aspiration `ʰ`, `ʱ`/breathy `◌̤`, creaky `◌̰` | strip (snap the base) | hin. /pʰal/ → `pal`, /bʱ/ → `b` |
| pharyngealization / emphasis `ˤ`, `◌̴`, velarization | strip (snap the base) | ara. /sˤ/ → `s`, /tˤ/ → `t`, /ðˤ/ → `z` |
| dental/apical/laminal/advanced/retracted place diacritics `◌̪ ◌̺ ◌̻ ◌̟ ◌̠` | ignore (snap the base) | /t̪/ → `t` |
| rhoticity `◌˞` and r-coloured vowels `ɚ ɝ` | vowel **+ `r`** | eng. *letter* /lɛtɚ/ → `leter` |
| explicit voicing `◌̥ ◌̬` | ignore the mark and snap the base symbol | |

### Vowel quality diacritics — ignore, snap the base symbol

Raised `◌̝`, lowered `◌̞`, advanced `◌̟`, retracted `◌̠`, centralized `◌̈`,
mid-centralized `◌̽`, and more/less rounded `◌̹ ◌̜` marks do not change the base symbol.
The code maps that symbol by the vowel rule.
The combining nasalization mark `◌̃` adds a nasal after the current vocalic sequence.
The code applies this rule even when the mark follows a consonant.

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

Every supported consonant sound that survives normalization maps as follows. The table is
grouped by Lojban target; the rationale column gives the principle.

| Lojban | IPA sources | Rationale |
|--------|-------------|-----------|
| `p` | p | identity |
| `b` | b | identity |
| `t` | t, t̪, ʈ, tˤ | voiceless coronal stop; dental/alveolar/retroflex/emphatic all neutralize (Lojban has one coronal stop) |
| `d` | d, d̪, ɖ, dˤ | voiced coronal stop; same neutralization |
| `k` | k, q | voiceless dorsal stop; uvular /q/ has no Lojban target and the velar is nearest |
| `g` | g, ɡ | identity |
| `f` | f, ɸ | voiceless labial fricative; bilabial [ɸ] (Japanese) → labiodental |
| `v` | v, ʋ | voiced labial fricative; Hindi /ʋ/ is the v/w phoneme |
| `s` | s, θ, sˤ | voiceless coronal fricative; **θ → s** (matches Spanish *seseo*, and keeps fricative manner; alt. `t`) |
| `z` | z, ð, ðˤ | voiced coronal fricative; **ð → z** (parallel to θ; alt. `d`). The phonetic input `ð` maps to `z`, including positional allophones of /d/ |
| `c` | ʃ, ɕ, ʂ | voiceless postalveolar/alveolo-palatal/retroflex sibilant — all "sh-like" → `c` |
| `j` | ʒ, ʑ, ʐ | voiced counterparts of the above → `j` |
| `x` | x, ɣ, χ, ħ, h, ɦ, ç | **every fricative at or behind the velum → `x`** (Lojban's only fricative there), so place (velar/uvular/pharyngeal/glottal) and voicing all neutralize. The official gismu rule: English /h/ → `x` (whence *derxi* ← *heap* /hip/), Arabic `ḥ`/`x` alike. ç realizes /x/ (German *ich*) or /h/ (Japanese *hi*) → `x`. Exception: the uvular **rhotic** /ʁ ʀ/ → `r` |
| `m` | m, ɱ | bilabial nasal |
| `n` | n, n̪, ŋ, ɳ, ɴ | every non-labial nasal → `n` (which is [ŋ] before velars anyway); the palatal /ɲ/ is handled as `n` + `i`-glide |
| `l` | l, ɫ, ɭ | lateral; dark/retroflex variants neutralize; palatal /ʎ/ → `l` + `i`-glide |
| `r` | r, ɾ, ɹ, ɻ, ʀ, ʁ, ɽ | any rhotic — trill, tap, approximant, uvular, retroflex flap. Uvular /ʁ ʀ/ are the **rhotic** of French/German/Portuguese, so → `r`, not `x` |
| `i`-glide | j, ʝ, ɲ→nj, ʎ→lj, ɥ | palatal approximants and the palatal consonants' glide. Spanish /ʝ/ (*yo*) → `i`-glide; Rioplatense [ʒ] → `j` |
| `u`-glide | w, ʍ | labiovelar approximant |
| *(dropped)* | ʔ, ʕ | The code removes the glottal stop and voiced pharyngeal. Neither has a scoring target. |

The code maps `ɲ` to `ni` and `ʎ` to `li`.
These mappings also apply when no vowel follows.
For example, `espaɲol` maps to `espaniol`, and `fiʎu` maps to `filiu`.
The final normalization collapses repeated `i` and `u` letters.
It retains other repeated letters.

## Stage 2d — vowel snapping

Lojban's five vowels sit at the periphery of the vowel space: front-unrounded
`i e`, back-rounded `o u`, and open `a`. Snap every (oral, de-diacriticked) vowel
to the nearest of these by **height** and **acoustic frontness**:

- **Height** picks the row: close / near-close → `i`/`u`; mid (close-mid &
  open-mid) → `e`/`o`; open → **`a`** (the only open vowel — all open qualities,
  front to back land here, except rounded `ɒ`, which maps to `o`).
- **Frontness** picks the column for the close & mid rows: **`i`/`e`** for front
  vowels *and* central-unrounded vowels; **`u`/`o`** for back vowels *and*
  central-rounded vowels.

Because Lojban has no front-rounded or back-unrounded vowels, one feature has to
give. We keep **tongue position (≈ F2)** and discard the lip-rounding mismatch:

- **Front-rounded → front-unrounded**: `y → i`, `ø → e`, `œ → e` (and the glide
  `ɥ → i`-glide). These have a high, front F2, so they are acoustically nearest
  `i`/`e`.
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

### Level of representation, and the schwa /ə/

Stage 1 describes the actual pronunciation within the supported IPA inventory.
It includes positional allophones, vowel reduction, final devoicing, and assimilation.
The transcription does not restore a spelling or an underlying sound that the speaker does not pronounce.

The tool examples include Russian *мягко* → `mʲaxkʌ`, *мялись* → `mʲælʲɪsʲ`, and *спасибо* → `spɐsʲibʌ`.
These examples retain supported symbols for the positional sounds.
Stage 2 maps `ɐ` and `ʌ` to `a`, and `ɪ` to `i`.
A phonetic distinction can disappear in the scoring letters.
The caller must still supply that distinction in the IPA input.

The mapper rejects bare `ə`, whether it represents a phoneme or an allophone.
If the pronunciation contains schwa, select the nearest full vowel from the supported inventory.
Use the actual quality of that sound in its position.
Do not restore a vowel from the spelling or the underlying form.
The caller supplies this choice because the mapper has no language or word context.

The examples in this guide give these scoring results after the caller selects a full vowel for schwa:

- French *le* → `le`, and *petit* → `peti`, with `ø` or `œ` for the reduced vowel.
- German *bitte* → `bite`, with `e` for the final vowel.
- Hindi *कमल* → `kamal`, with `ɐ` or `a` for the central vowels.
- English *sofa* → `sofa`, and *about* → `abaut`, with `a` for the reduced vowel.

The supported central vowels map as follows: `ɘ → e`, `ɵ → o`, `ɨ → i`, `ʉ → u`, and `ɐ → a`.
Stage 2 removes some phonetic detail, such as length and aspiration, after Stage 1 supplies the pronunciation.
This normalization does not change the phonetic input requirement.

### Input limits

The classic mapper rejects `β`, although the Spanish transcription instructions list it as a positional sound.
The mapper also rejects the notation `N`, `Q`, the linking mark `‿`, and tone digits.
Use supported IPA symbols for Japanese nasal sounds and the length mark `ː` for gemination.

The mapper ignores the voicing marks `◌̥` and `◌̬`.
If devoicing or assimilation changes a sound, select its actual base symbol in Stage 1.
For example, `d̥` maps to `d`, while `t` maps to `t`.

The code accepts `ɸ`, `ʝ`, and the reduced vowels `ɐ`, `ʌ`, and `ɪ`.
The tables specify their scoring letters.
The code also accepts `ʍ`, which maps to `u`.

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
- eng. *house* → /haʊs/ → `xaus` (/h/ → `x`, the same as *heap* → `xip` in *derxi*)
- spa. *gato*, drop the -o ending → /ɡat/ → `gat`
- rus. *спасибо* → `spɐsʲibʌ` → `spasiba` (reduced vowels `ɐ` and `ʌ` map to `a`)
- ara. *kitāb* → /kitaːb/ → `kitab` (drop length)
- ara. *ḥasan* → /ħasan/ → `xasan` (ħ → `x`, like every fricative at/behind the velum)
- fra. *bon* → /bɔ̃/ → `bon`; *tu* → /ty/ → `ti`
- deu. *schön* → /ʃøːn/ → `cen`; *grün* → /ɡʁyːn/ → `grin` (front-rounded /yː/ → `i`, ʁ → `r`)
- jpn. *sushi* → /sɯɕi/ → `suci`
- hin. *cāy* (tea) → /t͡ʃaːj/ → `cai` (affricate→`c`, drop length, j→`i`-glide)
- por. *pão* → /pɐ̃w̃/ → `paun`
- ben. *bhālo* → /bʱalo/ → `balo` (drop breathiness)

## Source input

The `word` field accepts a source-language IPA transcription.
The mapping converts that input to Lojban scoring letters.
The field does not require the caller to know Lojban spelling.
The tool field instructions ask the caller to drop grammatical endings.

The transcription supplies the pronunciation and resolves schwa before mapping.
The tables in this document define the accepted base symbols and their output letters.
Normalization handles the supported diacritics, length marks, nasalization, and affricates.
