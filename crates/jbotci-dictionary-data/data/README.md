# Lensisku Dictionary Snapshot

This directory contains the vendored Lensisku cached dictionary exports owned
and compiled by `jbotci-dictionary-data`. Keeping the build inputs inside the
crate makes every Cargo source package—and therefore the Python sdist—complete
without reaching back into a repository checkout.

Refresh the English JSON snapshot with:

```sh
cargo xtask vendor-dictionary
```

Use `cargo xtask vendor-dictionary --check` in CI or review workflows to verify
that the current cached export still validates without rewriting the vendored
files.

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
