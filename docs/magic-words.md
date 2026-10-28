# Magic words

Magic words act on words before syntax analysis.
Jbotci processes them from left to right in morphology.
A unit is one surviving word, quotation, compound, or letter word.
Later operators act on complete units.
They do not inspect words inside a completed unit.

This model controls word operations in jbotci.
It overrides conflicting descriptions in CLL chapter 19 and the camxes `*_sa` syntax rules.
The [gencmu word stream](https://github.com/int19h/gencmu/blob/main/grammars/lojban/common/stream.md) defines the reference model.
The rules in this guide describe the operations that jbotci supports.
A fault is an invalid unit that an eraser can remove.

## Reading order and hesitation

An operator acts on the surviving units to its left.
Erased material stays erased, and a later operator skips it.
The ordinary words that remain then enter syntax analysis.
An erasure can therefore repair a sequence that does not form a valid syntax tree.

Y is hesitation noise outside quoted material.
It does not supply an ordinary operand for SI or an SA match.
A Y run followed by BU forms the Y letter word, which has class BY.
SA recognizes that exception when it reads its next-word key.

## ZO

ZO quotes the next parsed word.
It skips bare hesitation before that target.
That word executes no operation, even if it is another magic word.
The quote forms one unit with class ZO for SA matching.

For example, `zo si` quotes SI.
In `mi zo si si broda`, the final SI erases the whole quote.
The surviving words are `mi broda`.

## ZOI

ZOI reads an opening delimiter and a raw body through the matching closing delimiter.
The body can contain non-Lojban text.
Words inside the body execute no operation.

The quote, both delimiters, and its body form one unit.
The opener supplies its matching class for SA.
For example, `mi zoi gy. si sa su .gy. si broda` leaves `mi broda`.
The SI after the closing delimiter removes the complete quote.

An unclosed ZOI quote reports a morphology error.
Neither an eraser inside its raw body nor a FAhO there closes the quote.
The `la'o` opener uses the same delimited-quote machinery.

## LOhU and LEhU

LOhU quotes Lojban words through the next LEhU.
The body does not need to form a grammatical phrase.
Its magic words execute no operation.

A completed quote forms one unit with matching class LOhU.
For example, `mi lo'u si sa su le'u si broda` leaves `mi broda`.
The SI after LEhU removes the complete quotation.

LEhU closes the quote and is not an elidable terminator.
An LEhU outside a quote creates an unopened-quote fault.
An incomplete LOhU quote remains a sequence for syntax diagnostics.
Jbotci does not execute erasers inside that returned sequence.

## LOhAI and LEhAI

A replacement quote forms one unit of raw words.
It contains up to two runs, each opened by `lo'ai` or `sa'ai`, followed by `le'ai`.
Both opening words have class LOhAI.
A bare `le'ai` also forms a complete unit, with class LEhAI.

No magic word executes inside a completed replacement quote.
Indicators stay raw words inside the body.
Hesitation remains noise, and the raw reader joins Y with a following BU into BY.
For example, `mi lo'ai zo fa'o si le'ai klama` keeps the complete quote and the surrounding words.
The internal ZO, FAhO, and SI execute no operation.

SI removes the complete quote.
SA matches its opening class and never examines its body.
For example, `mi lo'ai do le'ai sa lo'ai ti le'ai klama` keeps `mi`.
SU cannot find an internal LU, NIhO, TUhE, or TO boundary.
Thus `mi lo'ai lu le'ai su klama` leaves `klama`.
BU and a following ZEI take the complete quote as their base.

A free modifier adds information outside predicate arguments.
Syntax reads the quote as a free modifier.
In `lo'ai do sa'ai mi le'ai`, the first run quotes old text and the second quotes new text.
A leading `sa'ai` supplies new text without old text.
A bare `le'ai` supplies the close-only form.
Jbotci warns that this syntax is experimental.

After an opening `lo'ai`, a second `lo'ai` remains part of the quoted old text.
With a leading `sa'ai`, both runs form the quoted new text.
Emphasis marks the opening word, and indicators after the quote attach to its closing word.
Free modifiers after LEhAI attach to that closing word too.
The [gencmu replacement quote rules](https://github.com/int19h/gencmu/blob/main/grammars/lojban/common/lohai.md) define the word boundary.

If no complete quote starts at an active opening marker, that marker becomes a fault.
A later eraser can remove it.
For example, `mi lo'ai si klama` leaves `mi klama`.
If BU or ZEI takes that unfinished marker as its base, the marker remains a raw word instead.
Thus `lo'ai bu` forms a letter word, and `lo'ai zei broda` forms a compound.
ZO and the right operand reader of ZEI also take raw markers without opening a quote.

## ZEI

ZEI combines the preceding unit with the next word as read.
The right word executes no operation.
The resulting compound forms one BRIVLA unit.
With a missing operand, ZEI creates a BRIVLA fault unit.

For example, `mi zei fa'o` forms a compound instead of ending the text.
A later SI removes the whole compound.
In `mi zei do bu`, BU takes the completed compound as its base.

Repeated ZEI operations extend the preceding compound.
SA can match that compound as BRIVLA.
An SA search does not find the ZEI or its operands inside it.

## BU

BU turns the preceding unit into a letter word of class BY.
Its base can be a completed quote, compound, or another letter word.
The result forms one unit for later operators.

For example, `zo broda bu` names a letter from the quoted word.
`mi zei do bu` names a letter from the compound.
A later SI removes either letter word in one operation.

SA followed by BY can match a preceding letter word.
SA followed by BU cannot find the BU inside that letter word.
For example, `mi bu sa bu` erases the letter word and leaves BU without a base.
The dangling BU becomes a fault unit of class BY.
A following SI, SA, or SU can erase that fault.
For example, `mi bu sa bu si` leaves no words and no fault.

## SI

SI erases the preceding surviving unit.
It skips hesitation and material that an earlier eraser removes.
With no preceding unit, it erases nothing.

A run of SI words removes one unit for each SI.
For example, `mi broda brode si si klama` leaves `mi klama`.
A completed quote, compound, or letter word counts as one unit.

## SA

SA reads a key from the next word before that word executes.
The key is its word class, or selma'o.
SA erases through the nearest preceding surviving unit with a matching class.
The replacement word remains and executes normally.
The erased span includes the matched unit and all later surviving units before SA.

The class comes from the word inventory, not a syntax construct.
Bare brivla and bare cmevla have separate matching classes.
The CBM feature does not merge those classes for erasure.
An unclassified cmavo matches nothing, even an earlier copy of the same spelling.
If a cmavo belongs to more than one class, a shared class permits a match.
Each reach in a counted run must share that same class.

A run of N SA words selects the Nth nearest match.
With no match or too few matches, SA clears everything before it.
At the end of the text, SA has no key and clears everything before it.

A quote keeps its opener class for matching.
A ZEI compound has BRIVLA, and a letter word has BY.
SA never searches inside these units.
In `sa .ebu`, the key comes from `.e`, before BU forms a letter word.
The Y+BU exception supplies BY before the erasure.

SA before SI or SU clears its prefix, then that eraser acts on the empty prefix.
SA before FAhO clears its prefix, then FAhO ends the text.
No word after active FAhO supplies another key.

Examples:

| Input | Surviving words or result |
| --- | --- |
| `mi broda le brode sa ti` | `ti` |
| `lo broda sa broda` | `lo broda` |
| `mi broda sa brode` | `mi brode` |
| `mi broda gi'e klama da de di sa na gi'e prami` | `na gi'e prami`, which syntax rejects |
| `le le broda ku brode le broda sa sa le brodi` | `le le brodi`, which syntax rejects |
| `broda sa` | No surviving words |
| `mi broda sa fa'o ignored` | No surviving words |

The camxes `*_sa` rules repair constructs inside syntax.
The first five table examples show the differences from this model.
Those readings are known camxes bugs, not the rule that jbotci implements.

## SU

SU erases back to the nearest surviving NIhO, LU, TUhE, or TO boundary.
The boundary word remains.
With no boundary, SU clears everything before it.

A boundary inside an opaque quote or compound does not stop SU.
A later closing word does not cancel an earlier surviving boundary.
For example, `mi broda ni'o do brode su ti brodi` retains `mi broda ni'o ti brodi`.

## FAhO

Active FAhO ends the text immediately.
Jbotci ignores all later input.
FAhO does not repair the preceding syntax or erase words to find a grammatical prefix.

A quoted FAhO or a right operand of ZEI executes no end-of-text operation.
For example, `zo fa'o` remains a quote.
`mi zei fa'o` remains a compound.

## Fault units

BU without a base, ZEI with a missing operand, unopened LEhU, and unfinished LOhAI create the same kind of fault unit.
A fault keeps its source span, matching class, and original error.
It stays in the stream until an eraser removes it or morphology ends.

BU and ZEI can take a fault as their preceding unit.
Their result remains a fault, with the class of the constructed result.
The original error and diagnostic anchor remain intact.
For example, `bu zei mi si` erases one complete fault unit.

A surviving fault causes a morphology error at the end of the pass.
Active FAhO also ends that pass, so an eraser after FAhO cannot remove the fault.
ZO with a missing target and ZOI with a missing delimiter remain reader errors.
Such a quote does not expose its contents for later erasure.

## Words that mark other words

BAhE remains a separate word during these operations.
It marks the next word in syntax analysis.
For example, `broda ba'e si` leaves `broda`.
`mi ba'e fa'o` leaves a stranded BAhE and fails syntax.

UI and CAI indicators also remain separate for word operations.
Their later syntax attachment does not change the units that an eraser sees.
DAhO, FUhE, and FUhO follow that same word-operation rule.
