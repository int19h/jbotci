# Transcribing source words as IPA for `gimfihi`

Each source word you pass to `gimfihi` is a **broad phonetic IPA transcription**
of how the word is actually pronounced in its language — its *sounds*, not its
phonemes, and definitely not its spelling. `gimfihi` scores candidate gismu by
the letters they share with these transcriptions, so an inaccurate transcription
quietly corrupts every score. Treat this step as the real work.

## The rules

**Transcribe the pronunciation, not the spelling — and go as narrow
(allophone-level) as the supported inventory below allows.** If a phoneme has
allophones that are written with different IPA symbols, use the one that matches
how the word is *actually said* in this position — not the dictionary phoneme.
Apply the language's own processes:

- **vowel weakening / reduction** — e.g. Russian *akanye* (unstressed *о*, *а*
  reduce toward `[ɐ]`); English weak syllables;
- **final devoicing** — German, Russian word-final obstruents devoice (`d`→`t`,
  `g`→`k`, …);
- **assimilation** — adjacent sounds colour each other (`nb`→`mb`, `kz`→`gz`,
  nasals taking the place of a following stop, etc.);
- **positional allophones** generally — pick what is spoken, not the citation
  form.

**Drop grammatical endings** that are not part of the root idea — Spanish noun
*-o*/*-a*, common inflections — so the blend keys on the meaningful stem. *gato*
→ `gat`, not `gato`.

**Never write the schwa ə.** It is rejected. A true [ə] carries no
information for the blend, so wherever you would write one, commit to the nearest
full vowel that matches the schwa's *actual* quality in this word (schwa almost
always leans toward some real vowel — open, fronter, rounder). English *sofa*
ends in a roughly `[ɐ]`-like vowel → write `a`.

**Ignore tone and stress.** They do not affect the result; include or omit stress
marks as you like.

**When in doubt, look it up.** Check the word on **Wiktionary** (it usually gives
IPA) and skim the language's **Wikipedia phonology** article for the reduction
and allophony rules. Guessing from spelling is the single most common mistake.

**Double-check before submitting.** Re-read your transcription and confirm you
did not let orthography or an abstract phoneme leak in. Enumerate the relevant
phonological features (reduction, devoicing, assimilation, allophones) and make
sure each is represented in every position.

## Supported IPA inventory

Use standard IPA symbols from this set.
Code spans below contain supported IPA. Source spelling uses italics.
Unlisted sounds appear in ordinary text with a supported substitute.
Diacritics are understood: the tie bar
`◌͡◌`, length `ː`, nasalization `◌̃`, palatalization `ʲ`, labialization `ʷ`,
aspiration `ʰ`, and emphasis/pharyngealization `ˤ` are all fine to include.

**Consonants**

- Plosives: `p b t d k g q`, retroflex `ʈ ɖ`
- Fricatives: `f v θ ð s z ʃ ʒ ɕ ʑ ʂ ʐ ç x ɣ χ ħ h ɦ`
- Affricates: `t͡ʃ d͡ʒ t͡s d͡z t͡ɕ d͡ʑ`
- Nasals: `m n ŋ ɲ ɳ`
- Laterals & rhotics: `l ʎ ɫ`, `r ɾ ɹ ɻ ʀ ʁ ɽ`
- Approximants/glides: `j w ɥ ʋ`

**Vowels**

- Close/near-close: `i y ɨ ʉ ɯ u ɪ ʊ`
- Mid: `e ø ɛ œ ɘ ɜ o ɔ ɤ ɵ ɒ`
- Open: `a æ ɐ ɑ ʌ`
- Nasal vowels (`ɛ̃ ɑ̃ ɔ̃` …) and length (`aː`) are accepted.

There is deliberately **no ə** in the working set — resolve it as above.

## Per-language gotchas

These are the traps that most often produce wrong transcriptions for the common
source languages. Not exhaustive — verify on Wiktionary.

**Russian** — *akanye*: unstressed *о* and *а* reduce toward `[ɐ]` (use `ɐ`, not
`o`/`a`, and never ə); *ikanye*: unstressed *е* toward `[ɪ]`. Consonants before
front vowels and *ь* are **palatalized (soft)** — write the consonant with `ʲ` (нет → `nʲet`). Word-
final obstruents devoice. *ш ж* are retroflex `ʂ ʐ`; *х* is `x`; *ц* is `t͡s`;
*ч* is `t͡ɕ`.

**Spanish**: Drop the noun endings *-o* and *-a*.
Intervocalic *b d g* soften to approximants.
Write `b ð ɣ` within the listed inventory.
Use `b` for the unlisted bilabial sound β.
The trill *rr* is `r`, and the tap *r* is `ɾ`.
Castilian *z* and *c(e,i)* are `θ`. In *seseo* dialects, use `s`.
The letter *ñ* is `ɲ`. The letters *j* and *g(e,i)* are `x`.

**Mandarin** — drop tones. Retroflex series *zh ch sh r* = `ʈ͡ʂ ʈ͡ʂʰ ʂ ɻ`; alveolo-
palatal *j q x* = `t͡ɕ t͡ɕʰ ɕ`; *z c s* = `t͡s t͡sʰ s`. Aspiration is phonemic
(`pʰ tʰ kʰ`). *b d g* are unaspirated `[p t k]`. The vowel in *zi/ci/si*,
*zhi/chi/shi/ri* has a syllabic rhotic sound (write the listed vowel `i`).

**Arabic**: Emphatic consonants *ṣ ḍ ṭ ẓ* are `sˤ dˤ tˤ ðˤ`.
The letter *ق* is uvular `q`. The letters *ح ع* are pharyngeal sounds.
Write `ħ` for *ح*. Use the listed `ħ` for the unlisted voiced sound ʕ in *ع*.
The letters *خ غ* are `x ɣ` or `χ ʁ`.
Drop the feminine ending *ة* (*-a*) in the usual case.

**German** — final devoicing (*Hund* → `hʊnt`). Front-rounded vowels `y ø œ`
(*über*, *schön*). Affricates `p͡f t͡s`. *ch* is `ç` after front vowels / `x`
after back vowels; `r` is usually uvular `ʁ` (and vocalizes to `[ɐ]` in codas).

**French** — nasal vowels `ɛ̃ ɑ̃ ɔ̃ (œ̃)` (*bon* → `bɔ̃`); front-rounded `y ø œ`;
uvular `ʁ`; the glide `ɥ` (*lui*). Most written final consonants are silent.

**Hindi**: Dental stops differ from the retroflex stops `ʈ ɖ`.
Use the listed `t d` for the dental stops, without dental marks.
Use the listed aspiration mark for the aspirated and breathy series, such as `pʰ bʰ`.
The letters *v* and *w* represent `ʋ`.
The inherent vowel *अ* is a short central vowel.
Write its actual full vowel quality instead of schwa.

**Portuguese** — nasal vowels and nasal diphthongs (*pão* → `pɐ̃w̃`); European
Portuguese reduces unstressed vowels heavily; the "strong R" is dorsal (`ʁ`/`χ`,
or `h` in much of Brazil).

**Japanese**: The vowel *u* is unrounded `ɯ`.
The consonant *h* is `ç` before `i` and the unlisted bilabial sound ɸ before `ɯ`.
Use the listed `f` for ɸ.
The consonants *t d* become `t͡ɕ d͡ʑ` before `i` and `t͡s d͡z` before `ɯ`.
The moraic nasal *ん* takes the place of the next consonant, such as `m n ŋ`.

**Malay, Bengali** — Malay is largely as spelled (*c* = `t͡ʃ`, *ny* = `ɲ`, *ng* =
`ŋ`); Bengali has dental vs retroflex stops and a breathy series, plus an
inherent `[ɔ]`-quality vowel — verify individual words.

## Examples (word → IPA)

- English *cat* → `kæt`; *late* → `leɪ̯t`
- Spanish *gato* (drop *-o*) → `gat`
- Mandarin 用心 *yòngxīn* → `jʊŋɕin`
- French *bon* → `bɔ̃`
- Arabic *ḥasan* → `ħasan`
- Russian *мягко* → `mʲaxkʌ`; *спасибо* → `spɐsʲibʌ` (final unstressed *о* → `[ʌ]`
  by akanye, not `o`, and not ə)
