#!/usr/bin/env python3

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as functional
from transformers import AutoModel, AutoTokenizer


MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
MAX_SEQUENCE_LENGTH = 512
DIMENSIONS = 320
QUERY_PREFIX = "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: "
CASES = [
    {
        "name": "query-coi-ro-do",
        "kind": "query",
        "input": QUERY_PREFIX + "coi ro do",
    },
    {
        "name": "query-klama-zarci",
        "kind": "query",
        "input": QUERY_PREFIX + "mi klama le zarci",
    },
    {
        "name": "query-unicode-punctuation",
        "kind": "query",
        "input": QUERY_PREFIX + "xu do djica lo cidja - \"lojban\"?",
    },
    {
        "name": "document-klama-definition",
        "kind": "document",
        "input": "title: klama | text: x1 comes/goes to destination x2 from origin x3 via route x4 using means x5",
    },
    {
        "name": "empty",
        "kind": "edge",
        "input": "",
    },
]


def main():
    args = parse_args()
    device = torch.device(args.device)
    tokenizer = AutoTokenizer.from_pretrained(args.model, revision=args.revision)
    model = AutoModel.from_pretrained(
        args.model,
        revision=args.revision,
        torch_dtype=torch.float32,
    ).to(device)
    model.eval()

    inputs = [case["input"] for case in CASES]
    tokenized = tokenizer(
        inputs,
        padding=True,
        truncation=True,
        max_length=args.max_sequence_length,
        return_tensors="pt",
    )
    model_inputs = {name: value.to(device) for name, value in tokenized.items()}
    with torch.no_grad():
        hidden = model(**model_inputs).last_hidden_state
    lengths = model_inputs["attention_mask"].sum(dim=1) - 1
    rows = hidden[torch.arange(hidden.shape[0], device=device), lengths].float()
    rows = functional.normalize(rows, p=2, dim=1).cpu().numpy()
    if rows.shape[1] != DIMENSIONS:
        raise ValueError(f"embedding dimension mismatch: expected {DIMENSIONS}, got {rows.shape[1]}")

    output_cases = []
    for case, vector in zip(CASES, rows, strict=True):
        token_ids = tokenizer.encode(
            case["input"],
            truncation=True,
            max_length=args.max_sequence_length,
        )
        vector = vector.astype("<f4", copy=False)
        output_cases.append({
            "name": case["name"],
            "kind": case["kind"],
            "input": case["input"],
            "input_sha256": sha256(case["input"].encode("utf-8")),
            "token_ids": token_ids,
            "token_count": len(token_ids),
            "embedding": [float(value) for value in vector],
            "embedding_f32le_sha256": sha256(vector.tobytes()),
        })

    output = {
        "schema_version": 1,
        "reference": {
            "runtime": "python-transformers",
            "model": args.model,
            "revision": args.revision or "",
            "device": str(device),
            "torch_dtype": "f32",
        },
        "model_key": "f2llm-v2-80m-q4-320",
        "runtime": "jbotci-webgpu-f2llm",
        "runtime_version": "0.1.0",
        "dimensions": DIMENSIONS,
        "max_sequence_length": args.max_sequence_length,
        "pooling": "last-token",
        "normalized": True,
        "cosine_threshold": args.cosine_threshold,
        "cases": output_cases,
    }
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(output, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Generate PyTorch/Transformers F2LLM embedding goldens for browser WebGPU checks."
    )
    parser.add_argument("--out", default=".jbotci-build/f2llm-webgpu-goldens/goldens.json")
    parser.add_argument("--model", default=MODEL_ID)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--max-sequence-length", type=int, default=MAX_SEQUENCE_LENGTH)
    parser.add_argument("--cosine-threshold", type=float, default=0.95)
    args = parser.parse_args()
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    if not 0.0 < args.cosine_threshold <= 1.0:
        raise ValueError("--cosine-threshold must be in (0, 1]")
    return args


def sha256(data):
    return hashlib.sha256(data).hexdigest()


if __name__ == "__main__":
    main()
