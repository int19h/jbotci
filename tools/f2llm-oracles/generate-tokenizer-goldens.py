#!/usr/bin/env python3
"""Create tokenizer-only reference cases from a local pinned tokenizer file."""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import tempfile
from pathlib import Path

from tokenizers import Tokenizer

REVISION = "f4a16a11c9f5c8c7e22694653de6ce75430f4538"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tokenizer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.tokenizer.read_bytes()
    if hashlib.sha256(source).hexdigest() != "7e295e5bb91a3d35335f92fa4294a6e4e0ab4aa586db853e14312a62135bfddc":
        raise ValueError("the tokenizer bytes do not match the pinned revision")
    raw = json.loads(source)
    reference = Tokenizer.from_file(str(args.tokenizer))
    exporter_path = Path(__file__).resolve().parents[1] / "embedding-pack/f2llm/export-webgpu-from-onnx-q4.py"
    spec = importlib.util.spec_from_file_location("f2llm_exporter", exporter_path)
    exporter = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(exporter)
    # Export only tokenizer metadata. No model tensor or pack is read or written.
    with tempfile.TemporaryDirectory(dir=args.output.parent) as directory:
        manifest = exporter.write_tokenizer(args.tokenizer.parent, Path(directory))
        compact = json.loads((Path(directory) / manifest["url"]).read_text())
    texts = [
        "", "'Steve", "'Trump", "THEY'RE", "I'M", "I'm", "hello world", "mi klama le zarci",
        "café", "cafe\u0301", "你好 🌍", " \n\t ", "<THINK>", "before<think>after<|im_end|>",
        "<think></think><|im_end|><|im_end|>", "é<think>e\u0301", " <think>  ",
        *[token["content"] for token in raw["added_tokens"]],
        "".join(token["content"] for token in raw["added_tokens"]),
    ]
    pairs = [tuple(pair.split(" ") if isinstance(pair, str) else pair) for pair in raw["model"]["merges"]]
    ranks = {pair: index for index, pair in enumerate(pairs)}
    used_pairs = set()
    used_pieces = set()
    cases = []
    for text in texts:
        ids = []
        remaining = text
        while remaining:
            hits = [(remaining.find(token["content"]), token) for token in raw["added_tokens"]]
            hits = [(start, token) for start, token in hits if start >= 0]
            if hits:
                start, token = min(hits, key=lambda hit: (hit[0], -len(hit[1]["content"])))
                gap = remaining[:start]
                remaining = remaining[start + len(token["content"]):]
            else:
                gap, remaining, token = remaining, "", None
            for word, _ in reference.pre_tokenizer.pre_tokenize_str(reference.normalizer.normalize_str(gap)):
                pieces = list(word)
                used_pieces.update(pieces)
                while len(pieces) > 1:
                    neighbors = list(zip(pieces, pieces[1:]))
                    best = min(neighbors, key=lambda pair: ranks.get(pair, float("inf")))
                    if best not in ranks:
                        break
                    used_pairs.add(best)
                    joined = []
                    index = 0
                    while index < len(pieces):
                        if tuple(pieces[index:index + 2]) == best:
                            joined.append("".join(best))
                            index += 2
                        else:
                            joined.append(pieces[index])
                            index += 1
                    pieces = joined
                    used_pieces.update(pieces)
                ids.extend(raw["model"]["vocab"][piece] for piece in pieces)
            if token is not None:
                ids.append(token["id"])
        ids.append(151645)
        expected = reference.encode(text).ids
        assert ids == expected, (text, ids, expected)
        cases.append({"text": text, "token_ids": expected})
    # Keep the complete merge closure of these cases with the original IDs and rank order.
    compact["vocab"] = {piece: raw["model"]["vocab"][piece] for piece in sorted(used_pieces)}
    compact["merges"] = [list(pair) for pair in pairs if pair in used_pairs]
    result = {"source_model": "codefuse-ai/F2LLM-v2-80M", "source_revision": REVISION,
              "tokenizer_sha256": hashlib.sha256(source).hexdigest(),
              "tokenizer": compact, "cases": cases}
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(f"{len(cases)} cases, {len(used_pieces)} vocabulary pieces, {len(used_pairs)} merges")


if __name__ == "__main__":
    main()
