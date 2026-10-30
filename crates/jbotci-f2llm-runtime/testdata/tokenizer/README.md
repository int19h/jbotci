# Tokenizer reference cases

`pinned.json` contains full token-ID lists from the pinned F2LLM tokenizer.
A token ID identifies one vocabulary entry.
Each list includes the final EOS token.
EOS marks the end of a sequence.
The source revision and the SHA-256 digest identify the complete reference tokenizer.
The cases cover contractions, Unicode, ordinary text, and every added token.

The compact fixture keeps every intermediate vocabulary piece and every merge that these cases use.
It retains the reference IDs and merge order.
Its added-token table is complete.
The tests load the fixture through the production tokenizer and compare each complete token-ID list.
These cases need no model weights or inference.

Use a Python environment with `tokenizers==0.23.2`, `numpy`, and `onnx`.
To regenerate the fixture, supply the cached tokenizer file at the pinned revision:

```sh
python tools/f2llm-oracles/generate-tokenizer-goldens.py \
  --tokenizer /path/to/f4a16a11c9f5c8c7e22694653de6ce75430f4538/tokenizer.json \
  --output crates/jbotci-f2llm-runtime/testdata/tokenizer/pinned.json
```

The generator refuses source bytes with another digest.
It exports tokenizer metadata without model tensors or a vector pack.
It compares every expected token-ID list with the complete reference tokenizer before it writes the fixture.

# Compact tokenizer contract

The exporter writes `pre_tokenizer_pattern` and the full `added_tokens` table.
The runtime requires both fields and the exact pinned pattern.
It accepts literal added tokens with `single_word`, `lstrip`, `rstrip`, and `normalized` all false.
It rejects other matching rules, empty token contents, and duplicate IDs or contents.
The `special` flag does not exclude a token from encoding.

Unicode NFC composes equivalent character sequences.
The runtime finds added tokens in the original text, before NFC normalization.
It chooses the leftmost match and the longest token at that position.
Byte-pair encoding merges adjacent token pieces by rank.
The runtime normalizes and encodes each text gap with byte-pair encoding.
It appends EOS after all input tokens, including an input token with the EOS ID.
