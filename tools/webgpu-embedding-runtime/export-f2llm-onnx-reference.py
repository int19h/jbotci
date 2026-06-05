#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

import torch
import torch.nn.functional as functional
from transformers import AutoModel, AutoTokenizer


MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
MAX_SEQUENCE_LENGTH = 512


class F2LlmEmbeddingModule(torch.nn.Module):
    def __init__(self, model):
        super().__init__()
        self.model = model

    def forward(self, input_ids, attention_mask):
        output = self.model(
            input_ids=input_ids,
            attention_mask=attention_mask,
            use_cache=False,
        ).last_hidden_state
        lengths = attention_mask.sum(dim=1) - 1
        rows = output[torch.arange(output.shape[0], device=output.device), lengths].float()
        return functional.normalize(rows, p=2, dim=1)


def main():
    args = parse_args()
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    model_path = out_dir / "model.onnx"
    tokenizer = AutoTokenizer.from_pretrained(args.model, revision=args.revision)
    model = AutoModel.from_pretrained(
        args.model,
        revision=args.revision,
        torch_dtype=torch.float32,
    )
    model.eval()
    wrapper = F2LlmEmbeddingModule(model).eval()
    sample = tokenizer(
        "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: coi ro do",
        truncation=True,
        max_length=args.max_sequence_length,
        return_tensors="pt",
    )
    with torch.no_grad():
        torch.onnx.export(
            wrapper,
            (sample["input_ids"], sample["attention_mask"]),
            model_path,
            input_names=["input_ids", "attention_mask"],
            output_names=["embedding"],
            dynamic_axes={
                "input_ids": {0: "batch", 1: "sequence"},
                "attention_mask": {0: "batch", 1: "sequence"},
                "embedding": {0: "batch"},
            },
            opset_version=args.opset,
            do_constant_folding=True,
            dynamo=False,
        )
    manifest = {
        "schema_version": 1,
        "model": args.model,
        "revision": args.revision or "",
        "runtime": "onnxruntime-web",
        "url": "model.onnx",
        "max_sequence_length": args.max_sequence_length,
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["embedding"],
        "pooling": "last-token",
        "normalized": True,
        "opset": args.opset,
        "byte_length": model_path.stat().st_size,
    }
    (out_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def parse_args():
    parser = argparse.ArgumentParser(
        description="Export a browser ONNX reference model for F2LLM embedding correctness checks."
    )
    parser.add_argument("--out", default=".jbotci-build/f2llm-onnx-reference/v1")
    parser.add_argument("--model", default=MODEL_ID)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--max-sequence-length", type=int, default=MAX_SEQUENCE_LENGTH)
    parser.add_argument("--opset", type=int, default=18)
    args = parser.parse_args()
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    return args


if __name__ == "__main__":
    main()
