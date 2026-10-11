# Lensisku Dictionary Snapshot

This directory contains the vendored Lensisku cached dictionary exports owned
and compiled by `jbotci-dictionary-data`. Keeping the build inputs inside the
crate makes every Cargo source package complete without reaching back into a
repository checkout.

## Which export is vendored

The snapshot uses the unfiltered English export with `positive_scores_only=false`.
It includes words whose best English definition scores zero or less.

That export comes from the **authenticated** `/api/export/dictionary` route.
The anonymous `/api/export/cached` route cannot serve it, for two independent
reasons found in Lensisku's own sources:

- its nightly job pre-warms only the `positive_scores_only=true` variant
  (`export_all_dictionaries` says so in a comment), so the unfiltered variant
  is never in the cache and the download 404s; and
- Lensisku migration `V151` invalidates every cached row for a language pair on
  *any* definition edit or vote, so English — its most-edited language — is
  effectively never cached at all, in any variant.

The unfiltered export returns every definition of each word, including duplicates and repeat submissions. `jbotci-dictionary-data` embeds one definition per
word, chosen by Lensisku's own ranking — highest vote score, lowest definition
id to break a tie — in
`ImportedDictionary::retain_best_definition_per_word`. The vendored JSON stays
the verbatim export, including definitions that the importer does not select. The
metadata records both counts: `definition_count` for the file's rows,
`entry_count` for the entries actually embedded.

The export also holds rows that are not words. Lensisku's `wiki` word type
marks a free-form article stored in its word table. The importer counts such rows in `definition_count` but never
embeds them, so they reach neither word lookup nor the embedding corpus. A
`wiki` row that carries rafsi or a selma'o fails the import, because dropping
it would silently lose a word-level claim. Any other unknown word type also
fails the import, so that a new word class gets a deliberate classification.

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

`extracted-rafsi-en.json` contains audited short rafsi from definition and notes prose.
These forms have no structured rafsi record in the snapshot.
The file records its provenance, models, method, and extraction tooling.

`build.rs` merges the table into the parsed snapshot before any index is
built, so the extracted forms are ordinary listed rafsi everywhere
downstream: the rafsi index, `lookup_rafsi`, `short_rafsi_candidates`
availability, lujvo decomposition sources, and every `vlacku` endpoint.

The merge is **fail-closed**. The table was audited against one specific
snapshot, so the build fails, naming the offending word or form, when:

- a listed word is missing from the snapshot;
- a listed word is not a gismu or experimental gismu;
- a listed word already carries structured rafsi other than its own derived
  4-letter form in the snapshot — on *any*
  of its definition rows, selected or not, since upstream attaches rafsi per
  row and a claim may sit on a row best-definition selection drops;
- a listed form is not a CLL-derivable short rafsi of its word;
- a listed form is already claimed by another word's listed rafsi — again on
  any of its definition rows, selected or not — or by an embedded gismu's
  universal rafsi form;
- two listed words claim the same form.

### Refresh protocol

When a snapshot refresh makes the build fail on one of these
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

### Rafsi classification and selection

Lensisku records rafsi in two columns: `rafsi` for official assignments
and `experimental_rafsi` for experimental ones. The importer keeps each form
with the standing of its column, so the standing belongs to the rafsi, not to
the word. Lensisku also attaches rafsi to each definition rather than to the
word. An entry takes its rafsi from its selected definition only, because the
definitions of one word can be unrelated. So the
experimental rafsi `maz`, which a user-contributed definition of the official
cmavo `ma` proposes, does not reach jbotci's `ma`. The fail-closed audit still
reads every row, so it is deliberately stricter than the rafsi index: the
extracted table can never claim `maz`, but gimfihi reports `maz` as free
because no selected definition holds it. A word whose own standing is
experimental cannot hold an official rafsi, and a form listed in both columns
has no single standing; the import rejects both.
Lensisku also lists a gismu's 4-letter rafsi (the gismu minus its final vowel)
as a structured rafsi. jbotci derives that form itself, so the importer
discards exactly the derived form.
