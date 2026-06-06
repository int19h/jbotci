#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path
from typing import Iterable

import numpy as np
import onnxruntime as ort
from transformers import AutoTokenizer


SCHEMA_VERSION = 1
MODEL_KEY = "f2llm-v2-80m-q4-320"
MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
RUNTIME = "jbotci-webgpu-f2llm"
RUNTIME_VERSION = "0.1.0"
VECTOR_SPACE_KEY = "jbotci-webgpu-f2llm-q4-f16"
MAX_SEQUENCE_LENGTH = 512
DIMENSIONS = 320
DEFAULT_Q4_ONNX = (
    "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/"
    "f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx"
)
CORPORA = [
    ("vlacku-en", "dictionary", "entry_index"),
    ("cukta-cll", "cll", "chunk_index"),
]


def main() -> None:
    args = parse_args()
    q4_onnx = Path(args.q4_onnx)
    tokenizer_dir = Path(args.tokenizer_dir) if args.tokenizer_dir else q4_onnx.parent.parent
    corpus = read_json(Path(args.input))
    output = Path(args.out)
    stage = Path(args.stage) if args.stage else Path(f"{output}.staging")
    if stage == output:
        raise ValueError("--stage must differ from --out")
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)

    tokenizer = AutoTokenizer.from_pretrained(tokenizer_dir, fix_mistral_regex=True)
    session = ort.InferenceSession(str(q4_onnx), providers=["CPUExecutionProvider"])
    q4_onnx_sha256 = file_sha256(q4_onnx)

    pack_id = "-".join([
        corpus["inputFormatVersion"],
        short_hash(q4_onnx_sha256),
        short_hash(corpus["inputHash"]),
        VECTOR_SPACE_KEY,
    ])
    pack_root = stage / "models" / MODEL_KEY / "spaces" / VECTOR_SPACE_KEY / "packs" / pack_id
    pack_root.mkdir(parents=True)

    corpora = []
    for corpus_id, source_key, id_field in CORPORA:
        corpora.append(write_corpus(
            pack_root=pack_root,
            corpus=corpus,
            corpus_id=corpus_id,
            source_key=source_key,
            id_field=id_field,
            tokenizer=tokenizer,
            session=session,
            batch_size=args.batch_size,
        ))

    manifest_url = f"models/{MODEL_KEY}/spaces/{VECTOR_SPACE_KEY}/packs/{pack_id}/manifest.json"
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "model_key": MODEL_KEY,
        "model_revision": args.revision or "",
        "web_model": MODEL_ID,
        "q4_onnx_sha256": q4_onnx_sha256,
        "vector_space_key": VECTOR_SPACE_KEY,
        "pack_id": pack_id,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["inputHash"],
        "max_sequence_length": MAX_SEQUENCE_LENGTH,
        "built_by": {
            "runtime": "onnxruntime",
            "provider": "CPUExecutionProvider",
            "dtype": "q4",
            "source": "com.microsoft MatMulNBits/GatherBlockQuantized",
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build precomputed f16 vector packs for the custom F2LLM q4 WebGPU runtime."
    )
    parser.add_argument("--input", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", default=None)
    parser.add_argument("--q4-onnx", default=DEFAULT_Q4_ONNX)
    parser.add_argument("--tokenizer-dir", default=None)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--batch-size", type=int, default=8)
    args = parser.parse_args()
    if args.batch_size <= 0:
        raise ValueError("--batch-size must be positive")
    return args


def write_corpus(
    pack_root: Path,
    corpus: dict[str, object],
    corpus_id: str,
    source_key: str,
    id_field: str,
    tokenizer,
    session: ort.InferenceSession,
    batch_size: int,
) -> dict[str, object]:
    docs = corpus.get(source_key, [])
    if not isinstance(docs, list):
        raise ValueError(f"corpus field {source_key} must be an array")
    vectors = embed_texts(
        [str(doc["input"]) for doc in docs],
        tokenizer,
        session,
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


def embed_texts(texts: list[str], tokenizer, session: ort.InferenceSession, batch_size: int) -> np.ndarray:
    vectors = []
    input_names = {item.name for item in session.get_inputs()}
    for start, batch in enumerate_batches(texts, batch_size):
        encoded = tokenizer(
            batch,
            padding=True,
            truncation=True,
            max_length=MAX_SEQUENCE_LENGTH,
            return_tensors="np",
        )
        attention_mask = encoded["attention_mask"].astype(np.int64)
        feeds = {
            "input_ids": encoded["input_ids"].astype(np.int64),
            "attention_mask": attention_mask,
        }
        if "position_ids" in input_names:
            feeds["position_ids"] = position_ids(attention_mask)
        hidden = session.run(None, feeds)[0]
        rows = last_token_pool(hidden, attention_mask).astype(np.float32)
        rows = normalize(rows)
        if rows.shape[1] != DIMENSIONS:
            raise ValueError(f"embedding dimension mismatch: expected {DIMENSIONS}, got {rows.shape[1]}")
        vectors.append(rows)
        print(f"embedded {min(start + len(batch), len(texts))} of {len(texts)}", flush=True)
    if not vectors:
        return np.empty((0, DIMENSIONS), dtype=np.float32)
    return np.concatenate(vectors, axis=0)


def enumerate_batches(items: list[str], batch_size: int) -> Iterable[tuple[int, list[str]]]:
    for index in range(0, len(items), batch_size):
        yield index, items[index:index + batch_size]


def position_ids(attention_mask: np.ndarray) -> np.ndarray:
    positions = np.cumsum(attention_mask, axis=1, dtype=np.int64) - 1
    positions[attention_mask == 0] = 0
    return positions


def last_token_pool(hidden: np.ndarray, attention_mask: np.ndarray) -> np.ndarray:
    lengths = attention_mask.sum(axis=1).astype(np.int64)
    rows = np.arange(hidden.shape[0])
    return hidden[rows, lengths - 1, :]


def normalize(values: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(values, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    return values / norms


def read_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def promote(stage: Path, output: Path) -> None:
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


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def short_hash(value: str) -> str:
    return hashlib.sha256(str(value).encode("utf-8")).hexdigest()[:12]


if __name__ == "__main__":
    main()
