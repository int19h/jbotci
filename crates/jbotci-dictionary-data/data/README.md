# Lensisku Dictionary Snapshot

This directory contains the vendored Lensisku cached dictionary exports owned
and compiled by `jbotci-dictionary-data`. Keeping the build inputs inside the
crate makes every Cargo source package—and therefore the Python sdist—complete
without reaching back into a repository checkout.

## Which export is vendored

The snapshot is Lensisku's **unfiltered** English export — the
`positive_scores_only=false` variant. Keeping the flag on, as jbotci did until
jbotci issue #881, silently dropped every word whose best English definition
scores zero or less: 12,464 of this snapshot's 30,793 embedded words, most of
them simply never voted on.

That export comes from the **authenticated** `/api/export/dictionary` route.
The anonymous `/api/export/cached` route cannot serve it, for two independent
reasons found in Lensisku's own sources:

- its nightly job pre-warms only the `positive_scores_only=true` variant
  (`export_all_dictionaries` says so in a comment), so the unfiltered variant
  is never in the cache and the download 404s; and
- Lensisku migration `V151` invalidates every cached row for a language pair on
  *any* definition edit or vote, so English — its most-edited language — is
  effectively never cached at all, in any variant.

Since upstream migration `V157`, the unfiltered export returns *every*
definition of every word rather than the best one per word, duplicates and
repeat submissions included. `jbotci-dictionary-data` embeds one definition per
word, chosen by Lensisku's own ranking — highest vote score, lowest definition
id to break a tie — in
`ImportedDictionary::retain_best_definition_per_word`. The vendored JSON stays
the verbatim export, so recovering the alternates later needs no re-fetch. The
metadata records both counts: `definition_count` for the file's rows,
`entry_count` for the entries actually embedded.

## Refreshing

Refresh the English JSON snapshot with:

```sh
LENSISKU_USERNAME=... LENSISKU_PASSWORD=... cargo run -r -p xtask-full -- vendor-dictionary
```

Credentials are read from the environment, never from the command line, so they
stay out of process listings and shell history. `LENSISKU_TOKEN` is used
directly when set; otherwise the username and password are exchanged for one at
`/api/auth/login`.

Use `cargo run -r -p xtask-full -- vendor-dictionary --check` in CI or review
workflows. It reads only the working tree: it recomputes the vendored file's
SHA-256, definition count, and embedded entry count and fails on any mismatch
with `dictionary-en.metadata.toml`. It needs no credentials and no network.

`--check-upstream` answers the different question of whether Lensisku now
serves an export unlike the committed one. It fetches, so it needs credentials,
and it never rewrites the vendored files.

## Extracted rafsi (`extracted-rafsi-en.json`)

Hundreds of (mostly experimental) gismu in the Lensisku snapshot propose short
rafsi in their definition or notes prose without ever getting a structured
rafsi record. `extracted-rafsi-en.json` vendors the audited result of a
one-off five-model LLM extraction of those proposals (jbotci issue #768, run
2026-08-06): 55 gismu with 60 rafsi between them. The file carries its own
`provenance` block — run date, the models that voted, a one-line method
description, and a pointer to the extraction tooling and full audit trail
(the local `rafsi-extraction` repository).

`build.rs` merges the table into the parsed snapshot before any index is
built, so the extracted forms are ordinary listed rafsi everywhere
downstream: the rafsi index, `lookup_rafsi`, `short_rafsi_candidates`
availability, lujvo decomposition sources, and every `vlacku` endpoint.

The merge is **fail-closed**. The table was audited against one specific
snapshot, so the build fails, naming the offending word or form, when:

- a listed word is missing from the snapshot;
- a listed word is not a gismu or experimental gismu;
- a listed word already carries structured rafsi in the snapshot;
- a listed form is not a CLL-derivable short rafsi of its word;
- a listed form is already claimed by another entry's listed rafsi or by some
  gismu's universal rafsi form;
- two listed words claim the same form.

### Refresh protocol

When a snapshot refresh (see epic #664) makes the build fail on one of these
checks, **re-audit — never override**:

1. If the snapshot now lists rafsi for a word that also appears in
   `extracted-rafsi-en.json`, compare the two. The snapshot is authoritative:
   delete the word from `extracted-rafsi-en.json`. If the two disagree, say so
   in the commit message so the divergence is on record.
2. If a form is now claimed by another entry, the extracted claim loses unless
   an owner adjudicates otherwise; drop the losing word's form (and the word,
   if that was its only form).
3. If a word vanished or changed word type, drop it.

Do not relax the validations to make a refreshed snapshot build.

### Re-audits on record

**2026-09-01** (issue #881, unfiltered export): 55 words / 60 forms became 37
words / 40 forms.

- 14 words now carry structured rafsi in the snapshot, and in **every** case
  they are exactly the forms the extraction had proposed — no divergence to
  record: `corci`, `ditcu`, `dutso`, `flese`, `jonse`, `kibro`, `sfite`,
  `tonsi`, `vedli`, `vujnu`, `xrotu`, `zandi`, `zucna`, `zviki`.
- 4 words lost their only form to another entry, so they were dropped whole:
  `dzoli`'s `dzo` to `dzodu`, `grava`'s `gav` to `ganvi`, `kligo`'s `kig` to
  `kirgo`, and `losmo`'s `los` to `losxa`. The last is a consequence of the
  unfiltered export itself: `losxa` scores 0, so its claim on `los` was
  invisible while the snapshot kept only positively scored definitions.
