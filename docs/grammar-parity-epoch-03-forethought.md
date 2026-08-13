# Grammar parity epoch 3: forethought connectives

This note records the durable implementation dispositions for GitHub issues
#814 and #832. The rolling-Zantufa source is commit
`d5a5065c924429f6af12578d2900135b10cf1373`; its
`zantufa-1.9999.peg` SHA-256 is
`79e7a1da12552fb12457612e0f6e43a19ef263ee48466d9af6cf2a7ce86736d1`.
The camxes-standard source is ilmentufa commit
`778ea138f7d150121ca722db7536ce3b123943ac`; its `camxes.peg` SHA-256
is `a76803f447c15710a0d39283ea139b697d86fa162430b7b50ba14ae6dd60eb37`.

## JOIK consumer dispositions

| Consumer | Disposition | Grammar enforcement / owner |
| --- | --- | --- |
| Relative-clause connections | Include | Shared `relative_clause_connective` reaches typed Zantufa JOIK arms under `ZantufaConnectives`. |
| Selbri continuations | Include | Shared `relation_afterthought_connective` reaches the feature-gated arms. |
| FA/NU tanru-unit lists | Include | Shared `joik_connective` remains available at the existing list consumers. |
| Statement joins | Include | Standard and paragraph connective families have feature-gated typed arms; paragraph ordering preserves closed intervals first. |
| Standalone fragments | Include | Existing connective fragment consumers inherit the typed shared arms. |
| Operand connections | Include | `operand_connective` reaches the feature-gated arms. |
| Zantufa tag joins | Include | Epoch-2 `zantufa_tag_continuation` reaches the typed arms. |
| Forethought GEK openings | Include | GI-first JOIK and typed whole-Zantufa-tag alternatives are feature-gated. |
| Term connections | Exclude NA-led only | Lookahead gates at sumti and term continuation consumers preserve `mi na joi do broda` exactly. Other typed Zantufa JOIK shapes remain available. |
| Deferred rolling consumer forms outside the generated v1 model | Defer | Owned by the grammar-parity epoch for the enclosing construct; no untyped or heuristic route is added here. |

## Waivers and probes

The durable fixture provenance contains the exact commands and outcomes. The
single batch probe used during implementation was:

```sh
node - <<'NODE'
const p=require('/home/int19h.linux/git/grammar-review/upstream/gerna_cipra/js/zantufa-1.9999.js');
for (const s of ['ga bo mi klama gi do klama', 'gi ba mi klama gi do klama',
  'gi joi bo mi klama gi do klama', 'gi na joi mi klama gi do klama',
  "gi ga'o joi ga'o mi klama gi do klama", "gi ga'o bi'i mi klama gi do klama",
  'mi klama i na joi do klama', 'mi na joi do broda',
  'gi ji bo mi klama gi do klama', "gi zi'e bo mi klama gi do klama",
  'ga nai bo mi klama gi do klama', 'ba gi nai bo mi klama gi do klama']) {
  try { p.parse(s); console.log('ACCEPT', s) } catch (e) { console.log('REJECT', s) }
}
NODE
```

All listed surfaces were accepted by the pinned rolling parser. `ji` and `zi'e`
remain unsupported lexical-repurposing waivers because jbotci classifies them
outside JOI. `ga nai bo` and `tag gi nai bo` remain deliberate structural
attachment-model divergences: rolling Zantufa accepts NAI as a free/UI
attachment, but neither source grammar has a connector node that owns both NAI
and BO. The grammar therefore uses disjoint baseline NAI/no-BO and Zantufa
BO/no-NAI variants; the DSL cannot assert over two parsed optional fields.

NA-led and one-sided-GAhO Zantufa JOIK nodes are accepted syntactically but
produce an explicit unsupported-semantics error. Paired endpoints and ordinary
JOI/BIhI forms continue through existing lowering. A follow-up issue owns full
semantic lowering of the unsupported shapes.
