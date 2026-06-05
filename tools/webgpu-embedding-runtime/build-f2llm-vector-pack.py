#!/usr/bin/env python3

import argparse
import hashlib
import json
import shutil
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as functional
from transformers import AutoModel, AutoTokenizer


SCHEMA_VERSION = 1
MODEL_KEY = "f2llm-v2-80m-q4-320"
MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
RUNTIME = "jbotci-webgpu-f2llm"
RUNTIME_VERSION = "0.1.0"
VECTOR_SPACE_KEY = "jbotci-webgpu-f2llm-q4-f16"
MAX_SEQUENCE_LENGTH = 512
DIMENSIONS = 320
CORPORA = [
    ("vlacku-en", "dictionary", "entry_index"),
    ("cukta-cll", "cll", "chunk_index"),
]


def main():
    args = parse_args()
    corpus = read_json(Path(args.input))
    output = Path(args.out)
    stage = Path(args.stage) if args.stage else Path(f"{output}.staging")
    if stage == output:
        raise ValueError("--stage must differ from --out")
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)

    device = torch.device(args.device)
    tokenizer = AutoTokenizer.from_pretrained(args.model, revision=args.revision)
    model = AutoModel.from_pretrained(
        args.model,
        revision=args.revision,
        torch_dtype=torch.float32,
    ).to(device)
    model.eval()

    pack_id = "-".join([
        corpus["inputFormatVersion"],
        short_hash(args.revision or "default"),
        short_hash(corpus["inputHash"]),
        VECTOR_SPACE_KEY,
    ])
    pack_root = stage / "models" / MODEL_KEY / "spaces" / VECTOR_SPACE_KEY / "packs" / pack_id
    pack_root.mkdir(parents=True)

    corpora = []
    for corpus_id, source_key, id_field in CORPORA:
        corpora.append(write_corpus(
            pack_root,
            corpus,
            corpus_id,
            source_key,
            id_field,
            tokenizer,
            model,
            device,
            args.batch_size,
        ))

    manifest_url = f"models/{MODEL_KEY}/spaces/{VECTOR_SPACE_KEY}/packs/{pack_id}/manifest.json"
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "model_key": MODEL_KEY,
        "source_model_key": corpus.get("modelKey"),
        "model_revision": args.revision or "",
        "web_model": MODEL_ID,
        "vector_space_key": VECTOR_SPACE_KEY,
        "pack_id": pack_id,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["inputHash"],
        "max_sequence_length": MAX_SEQUENCE_LENGTH,
        "built_by": {
            "runtime": "python-transformers",
            "version": "transformers",
            "dtype": "f32",
            "device": str(device),
        },
        "dimensions": DIMENSIONS,
        "element_type": "f16le",
        "normalized": True,
        "distance": "dot",
        "compatible_query_runtimes": [
            {
                "runtime": RUNTIME,
                "version": RUNTIME_VERSION,
                "dtype": "q4",
                "device": "webgpu",
            },
        ],
        "corpora": corpora,
    }
    write_json(pack_root / "manifest.json", manifest)
    catalog = {
        "schema_version": SCHEMA_VERSION,
        "models": [
            {
                "model_key": MODEL_KEY,
                "vector_spaces": [
                    {
                        "vector_space_key": VECTOR_SPACE_KEY,
                        "latest_pack_id": pack_id,
                        "manifest_url": manifest_url,
                        "compatible_query_runtimes": manifest["compatible_query_runtimes"],
                    },
                ],
            },
        ],
    }
    write_json(stage / "catalog.json", catalog)
    promote(stage, output)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Build precomputed f16 vector packs for the custom F2LLM WebGPU runtime."
    )
    parser.add_argument("--input", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", default=None)
    parser.add_argument("--model", default=MODEL_ID)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--batch-size", type=int, default=8)
    args = parser.parse_args()
    if args.batch_size <= 0:
        raise ValueError("--batch-size must be positive")
    return args


def write_corpus(pack_root, corpus, corpus_id, source_key, id_field, tokenizer, model, device, batch_size):
    docs = corpus.get(source_key, [])
    vectors = embed_texts(
        [doc["input"] for doc in docs],
        tokenizer,
        model,
        device,
        batch_size,
    )
    corpus_dir = pack_root / "corpora" / corpus_id
    corpus_dir.mkdir(parents=True)
    items = [
        {
            id_field: doc["id"],
            "input_hash": doc["inputHash"],
            "kind": doc.get("kind"),
            "row": row,
        }
        for row, doc in enumerate(docs)
    ]
    items_path = corpus_dir / "items.json"
    write_json(items_path, items)
    vector_bytes = vectors.astype("<f2", copy=False).tobytes()
    vector_path = corpus_dir / "vectors.f16"
    vector_path.write_bytes(vector_bytes)
    return {
        "corpus_id": corpus_id,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["dictionaryHash"] if corpus_id == "vlacku-en" else corpus["cllHash"],
        "row_count": len(docs),
        "dimensions": DIMENSIONS,
        "items_url": f"corpora/{corpus_id}/items.json",
        "items_sha256": sha256(items_path.read_bytes()),
        "vector_url": f"corpora/{corpus_id}/vectors.f16",
        "vector_byte_len": len(vector_bytes),
        "vector_sha256": sha256(vector_bytes),
    }


def embed_texts(texts, tokenizer, model, device, batch_size):
    vectors = []
    for start in range(0, len(texts), batch_size):
        batch = texts[start:start + batch_size]
        inputs = tokenizer(
            batch,
            padding=True,
            truncation=True,
            max_length=MAX_SEQUENCE_LENGTH,
            return_tensors="pt",
        )
        inputs = {name: value.to(device) for name, value in inputs.items()}
        with torch.no_grad():
            hidden = model(**inputs).last_hidden_state
        lengths = inputs["attention_mask"].sum(dim=1) - 1
        rows = hidden[torch.arange(hidden.shape[0], device=device), lengths].float()
        rows = functional.normalize(rows, p=2, dim=1)
        if rows.shape[1] != DIMENSIONS:
            raise ValueError(f"embedding dimension mismatch: expected {DIMENSIONS}, got {rows.shape[1]}")
        vectors.append(rows.cpu().numpy())
        print(f"embedded {min(start + len(batch), len(texts))} of {len(texts)}", flush=True)
    if not vectors:
        return np.empty((0, DIMENSIONS), dtype=np.float32)
    return np.concatenate(vectors, axis=0)


def read_json(path):
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def promote(stage, output):
    backup = Path(f"{output}.previous")
    shutil.rmtree(backup, ignore_errors=True)
    if output.exists():
        output.rename(backup)
    try:
        stage.rename(output)
    except Exception:
        if backup.exists() and not output.exists():
            backup.rename(output)
        raise
    shutil.rmtree(backup, ignore_errors=True)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def short_hash(value):
    return hashlib.sha256(str(value).encode("utf-8")).hexdigest()[:12]


if __name__ == "__main__":
    main()
