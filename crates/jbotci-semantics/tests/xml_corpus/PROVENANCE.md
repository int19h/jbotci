# Frozen SFN-XML parity corpus

This directory vendors the authoritative SFN-XML adoption corpus from
`int19h/tersmu-dsl-research` commit
`e25eeaf09bab4f14eea98e73cd1244ac464346da`.

- `*.frozen.json` are the 46 comparable canonical source graphs from
  `experiments/phase-a/battery-renders-smusni/fresh-json-v2`.
- `*.xml.txt` are the corresponding frozen outputs from
  `experiments/phase-a/battery-renders-xml`.

The pinned research directories also contain `b56-quote`, which the research
`README-xml.md` identifies as a separate XML check witness rather than a
PM-verified comparison golden. It is therefore intentionally excluded from
this 46-document product parity corpus.

Before these files were imported, a fresh run from the clean research checkout
at the pinned commit was performed:

```text
PYTHONDONTWRITEBYTECODE=1 python3 render_xml.py --check --output-dir <scratch>
diff -qr <scratch> experiments/phase-a/battery-renders-xml
```

The prototype checked all 47 research documents, including the separate
`b56-quote` witness, and `diff` reported no differences. A
relative-name/content `sha256sum` manifest for both the fresh output and frozen
directory had aggregate SHA-256
`556811096b4581a2bda2bd492962cf44d94d96b4e578463fa9d99ced2a03d229`.
Thus the imported 46-file subset also matches the fresh prototype bytes. The
product tests pin independent ordered name/content hashes for both imported
file families.

The files are frozen evidence. Update them only after a separately reviewed
notation decision and a fresh, pinned-oracle parity proof.
