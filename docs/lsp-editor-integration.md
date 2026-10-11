# Lojban language server and editor integration

`jbotci lsp --stdio` provides the language server through the CLI binary.
The server uses `async-lsp` for the protocol adapter.
The VS Code extension connects to that process.

## Components

The implementation separates document analysis from the protocol:

- `crates/jbotci-ide` provides document snapshots and queries.
- `apps/jbotci/src/lsp.rs` handles JSON-RPC over standard input and output.
- `editors/vscode` provides the VS Code client and language configuration.
- `crates/jbotci-web-core` supplies the shared morphology and syntax analysis.

The IDE crate uses no input/output, asynchronous runtime, or `lsp-types`.
It keeps queries separate from transport and position encoding.
It uses the same language libraries as the web application.
The server does not require `jbotci-server`.

## Document analysis

A snapshot is an immutable analysis of one document version.
`DocumentSnapshot` holds the source, version, line index, recovered morphology, recovered syntax, and diagnostics.
It also stores data for highlighting, inlays, selection ranges, and folding ranges.

The server accepts incremental text changes and applies them as source edits.
The server first prepares morphology and attempts an incremental diagnostic update.
If that update produces a provisional result, the server publishes it and waits until the 200 ms debounce deadline.
A provisional result supplies diagnostics before full syntax analysis completes.

The deadline starts when the server schedules the analysis.
An initial document or a failed incremental gate proceeds directly to full syntax analysis.
A debounce interval delays repeated work while edits arrive.
The server discards analysis results whose document version or generation no longer matches.

A source span is a range in the original text.
The shared analysis preserves source spans through morphology and syntax recovery.
Queries use these spans to address the original document.
A broken parse can still provide diagnostics and syntax features.

## Permissive lexer

The morphology parser has a fixed separator set in `segment::is_separator`.
It includes whitespace, period characters, and these punctuation characters:

```text
? ! ; : - ( ) [ ] { } < > " « » “ ” ‘ ⟨ ⟩ ⦗ ⦘ ⁅ ⁆ ⦇ ⦈ ⟦ ⟧ ⦉ ⦊ ⟪ ⟫
```

That set applies in both strict and permissive modes.

The dialect configuration controls an additional permissive tier.
Unicode punctuation, symbols, emoji, and emoji components act as word separators, except for explicitly reserved characters.
`PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS` lists the reservations:

- Period, comma, and apostrophe.
- ASCII digits and `$`.
- Subscript digits U+2080–U+2089 and signs U+208A–U+208B.
- Brackets U+2E24–U+2E25 and non-breaking hyphen U+2011.

The reservations restrict only the additional tier.
ASCII hyphen remains a separator in both modes.
Strict mode reports `InvalidCharacter` for characters that only the permissive tier accepts.
A punctuation character inside a word, such as `@`, can split that word in permissive mode.

The segmenter uses the original text and keeps source spans.
It does not strip characters before analysis.
When it ignores additional characters, it emits one Advice diagnostic with code `morphology.advice.ignored-characters`.
The diagnostic counts only those characters and labels the first one.
The language server presents Advice as Hint.
The lexer does not identify Markdown regions such as URLs, code fences, or HTML.

## Position mapping

`LineIndex` maps byte, character, and UTF-16 offsets to line and column positions.
The protocol adapter negotiates UTF-8 or UTF-16 with the client.
UTF-16 is the fallback encoding.

The index handles CRLF line endings and characters outside the basic multilingual plane.
The analysis creates `SourceSpan` values with byte and character offsets and empty optional `start` and `end` line positions.
`LineIndex` supplies the client positions.
Queries keep source offsets separate from protocol positions.
The adapter converts positions at the protocol boundary.

## Diagnostics

The server supports pull diagnostics and push diagnostics.
Pull diagnostics answer client requests.
Push diagnostics send notifications without a diagnostic request.
The server selects the route from client capabilities.

Morphology and syntax diagnostics share the `jbotci-diagnostics` model.
The adapter maps Error to Error, Warning to Warning, and Advice to Hint.
The adapter uses the primary label for the range and exposes other labels as related information.
Diagnostic notes use the shared text segments for Markdown or text output.
The adapter retains diagnostic codes and identifies the phase in `source`, such as `jbotci/morphology` or `jbotci/syntax`.

### Expected alternatives

The underlying expectation set and the detailed note must contain the complete expected alternatives.
A scoped summary must have the complete detailed note beside it.
Name each alternative by the rule whose next element remains unconsumed.
Do not name it by an ancestor whose earlier fields are already complete.
This naming rule applies to both the summary and the detailed note.

`syntax_expectation_summary_constructs` limits the summary to constructs relevant to the enclosing scope, free modifiers, and end of input.
It applies this filter only when the expectation set contains a construct relevant to that scope.
`syntax_detailed_segments` presents the full expectation set without this filter.
The summary and the detailed note can therefore contain different numbers of alternatives.

## Hover

Hover finds the word at the cursor through recovered morphology spans.
The dictionary supplies definitions, glosses, places, and rafsi where available.
The hover range covers the dictionary unit that the card describes.
Card headings put the word classification, including cmavo selma'o, on the headword line.
The shared renderer omits layout-only labels.

The renderer handles word groups as follows:

- A dictionary-attested cmavo sequence replaces the constituent card with the longest contiguous sequence that contains the cursor.
- A ZEI compound uses its own dictionary card when available, otherwise its component cards.
- A lujvo uses its dictionary entry when available, otherwise its decomposition and component cards.
- A fu'ivla without a dictionary entry receives morphology information.
- A cmevla receives morphology information.
- A foreign quotation payload receives no hover card.

Contiguity comes from the morphology word stream.
Only whitespace and periods can separate spans in a dictionary-attested cmavo sequence.
Equal-length candidates retain source order.
The shared sequence index derives its keys from morphology, independently of dictionary tags.
Compact, spaced, and mixed headwords use the same component keys.

The dictionary index also serves the web Blocks view.
Blocks uses a global longest-first partition, but hover selects the sequence around the cursor.
For `ba pu ba`, Blocks groups `ba pu`, while hover on the final `ba` describes `pu ba`.

## Completion

Completion combines grammar expectations, dictionary entries, and document-local names.
The parser supplies reason groups for continuations, nested constructs, and constructs that end before another starts.
It supplies expectations for both incomplete and completed source prefixes through `expected_continuations_with_time_limit`.
`SyntaxExpectation` groups typed tokens under `ContinueCurrent`, `StartNested`, or `EndThenStart` reasons.

The cursor query offers two interpretations of the trailing word-forming text.
It can extend that text as a prefix or continue after completed morphology words.
Completed-word continuations rank before prefix extensions.
The morphology parser rejects candidates that merge with adjacent source into a different word.

Expected cmavo and selma'o expand to their words.
Expected brivla and selbri words expand to dictionary brivla.
Expected pro-sumti and letter words expand to the KOhA and BY sets.
Expected quotations expand to quotation-opening cmavo.
`EndOfInput` and named expectations add no candidates.
Expected cmevla use names from the document.
The dictionary supplies normalized prefix ranges through its sorted word index.
Foreign quotation payloads receive no completion, while unrestricted word quotations use an unfiltered word list.

The grammar query has a one-second time limit.
If grammar information is unavailable, completion uses morphology-valid candidates.
The result records the source of each candidate.
Cancellation can stop a completion worker.

The adapter inserts bare words without automatic pause periods.
It assigns completion kinds and reason text.
Brivla use Function, pro-sumti use Variable, cmavo and letter words use Keyword, cmevla use Value, and terminators use Operator.
The label description contains the short gloss.
Dictionary documentation resolves through `completionItem/resolve`.

## Semantic tokens

Semantic tokens are spans with morphology-derived highlighting categories.
The categories cover word classes and groups of selma'o.
The protocol adapter returns full-document tokens.
The token modifier list is empty.

The tokenizer uses morphology spans rather than spaces or regular expressions.
It therefore handles text without spaces between words.
Position conversion uses the negotiated client encoding.

## Inlays and tree regions

Inlays add labels at source positions without changing the text.
The server supports structure brackets, word boundaries, and rafsi boundaries.
Each kind has an independent configuration value.

The VS Code extension passes `jbotci.inlays` as `initializationOptions.inlays`:

```json
{
  "structureBrackets": true,
  "wordBoundaries": false,
  "rafsiBoundaries": true
}
```

`structureBrackets` also accepts a profile object with `profile`, `maxNestingDepth`, and `constructs`.
The server accepts `initializationOptions.structureInlays` and emits a deprecation warning.
It refuses initialization that supplies both shapes.

Structure brackets use source fragments from the recovered syntax tree.
Selection ranges provide nested source regions for Expand and Shrink Selection.
Folding ranges provide source regions for code folding.

## Reference and place analysis

`GeneratedReferenceAnalysis::analyze` accepts a completely valid `GeneratedTextSyntax`.
Its syntax index records node parents, depth, preorder, and source spans.
Reference edges record targets and the rules that resolve them.
Place analysis provides frames, sumti assignments, and queries such as `frames_for_node` and `first_argument_for_place`.

The recovered tree renderer adds references only when `try_into_valid()` succeeds for the whole tree.
A recovery error anywhere prevents reference and place analysis throughout the document.
The language server provides morphology and syntax features without this analysis.
See [#408](https://github.com/int19h/jbotci/issues/408) for work on analysis of recovered trees.

## VS Code and Markdown

The extension registers `.jbo` files as Lojban.
Its word pattern includes apostrophes and commas.
Files ending in `.jbo.md` retain the Markdown language and built-in Markdown features.

The extension adds Lojban providers to eligible Markdown documents.
`jbotci.enableInAllMarkdown` enables these providers for all Markdown documents.
URLs, code fences, and HTML can produce diagnostics because the server analyzes them as Lojban text.

The server path comes from `jbotci.serverPath` or `PATH`.
The extension starts the server when an eligible document opens.
See [the extension README](../editors/vscode/README.md) for configuration and packaging commands.

## Tests

IDE tests cover document queries, position conversion, diagnostics, completion, highlighting, inlays, and tree regions.
Protocol tests drive `jbotci lsp` through standard input and output.
The tests cover initialization, document changes, requests, and response positions.

Position tests include multibyte and astral characters.
Completion tests exercise grammar filtering and morphology boundaries.
Expectation changes require the same review as parser fixture changes.
