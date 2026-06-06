#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
import onnxruntime as ort
from transformers import AutoTokenizer


MODEL_KEY = "f2llm-v2-80m-q4-320"
RUNTIME = "jbotci-webgpu-f2llm"
RUNTIME_VERSION = "0.1.0"
VECTOR_SPACE_KEY = "jbotci-webgpu-f2llm-q4-f16"
MAX_SEQUENCE_LENGTH = 512
DIMENSIONS = 320
DEFAULT_Q4_ONNX = (
    "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/"
    "f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx"
)
CORPORA = {
    "vlacku-en": ("dictionary", "entry_index", "dictionaryHash"),
    "cukta-cll": ("cll", "chunk_index", "cllHash"),
}


def main() -> None:
    args = parse_args()
    pack = Path(args.pack)
    corpus = read_json(Path(args.corpus))
    catalog = read_json(pack / "catalog.json")
    vector_space = catalog_vector_space(catalog)
    manifest_path = pack / vector_space["manifest_url"]
    manifest = read_json(manifest_path)
    validate_manifest(manifest, corpus)
    pack_root = manifest_path.parent

    tokenizer_dir = Path(args.tokenizer_dir) if args.tokenizer_dir else Path(args.q4_onnx).parent.parent
    tokenizer = AutoTokenizer.from_pretrained(tokenizer_dir, fix_mistral_regex=True)
    session = ort.InferenceSession(str(args.q4_onnx), providers=["CPUExecutionProvider"])

    comparisons = []
    for corpus_manifest in manifest["corpora"]:
        corpus_id = corpus_manifest["corpus_id"]
        source_key, id_field, hash_field = CORPORA[corpus_id]
        docs = corpus[source_key]
        items_path = pack_root / corpus_manifest["items_url"]
        vector_path = pack_root / corpus_manifest["vector_url"]
        verify_file_sha256(items_path, corpus_manifest["items_sha256"])
        verify_file_sha256(vector_path, corpus_manifest["vector_sha256"])
        items = read_json(items_path)
        vectors = read_vectors(vector_path, corpus_manifest["row_count"])
        validate_corpus_manifest(corpus_manifest, docs, items, vectors, corpus[hash_field], id_field)
        for row in sample_rows(corpus_manifest["row_count"], args.sample_rows):
            expected = embed_texts([docs[row]["input"]], tokenizer, session)[0]
            actual = vectors[row].astype(np.float32)
            actual = normalize(actual.reshape(1, -1))[0]
            cosine = float(np.dot(expected, actual))
            comparisons.append({
                "corpus_id": corpus_id,
                "row": row,
                "cosine": cosine,
            })
            if cosine < args.threshold:
                raise AssertionError(
                    f"{corpus_id} row {row} cosine {cosine:.6f} is below threshold {args.threshold:.6f}"
                )

    print(json.dumps({
        "validated_pack": manifest["pack_id"],
        "model_key": manifest["model_key"],
        "q4_onnx_sha256": manifest.get("q4_onnx_sha256"),
        "comparisons": comparisons,
    }, indent=2))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate a generated F2LLM q4 f16le vector pack.")
    parser.add_argument("--pack", default=".jbotci-build/r2-web-embeddings-f2llm", type=Path)
    parser.add_argument("--corpus", default=".jbotci-build/web-embedding-corpus.json", type=Path)
    parser.add_argument("--q4-onnx", default=DEFAULT_Q4_ONNX, type=Path)
    parser.add_argument("--tokenizer-dir", default=None)
    parser.add_argument("--sample-rows", type=int, default=3)
    parser.add_argument("--threshold", type=float, default=0.999)
    args = parser.parse_args()
    if args.sample_rows <= 0:
        raise ValueError("--sample-rows must be positive")
    if not 0.0 < args.threshold <= 1.0:
        raise ValueError("--threshold must be in (0, 1]")
    return args


def catalog_vector_space(catalog: dict[str, object]) -> dict[str, object]:
    assert catalog.get("schema_version") == 1
    models = [model for model in catalog.get("models", []) if model.get("model_key") == MODEL_KEY]
    if len(models) != 1:
        raise AssertionError(f"catalog must contain exactly one {MODEL_KEY} model entry")
    spaces = [
        space
        for space in models[0].get("vector_spaces", [])
        if space.get("vector_space_key") == VECTOR_SPACE_KEY
    ]
    if len(spaces) != 1:
        raise AssertionError(f"catalog must contain exactly one {VECTOR_SPACE_KEY} vector space")
    return spaces[0]


def validate_manifest(manifest: dict[str, object], corpus: dict[str, object]) -> None:
    expected = {
        "schema_version": 1,
        "model_key": MODEL_KEY,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["inputHash"],
        "max_sequence_length": MAX_SEQUENCE_LENGTH,
        "dimensions": DIMENSIONS,
        "element_type": "f16le",
        "normalized": True,
        "distance": "dot",
    }
    for field, value in expected.items():
        if manifest.get(field) != value:
            raise AssertionError(f"manifest {field} mismatch: expected {value!r}, got {manifest.get(field)!r}")
    compatible = manifest.get("compatible_query_runtimes", [])
    expected_runtime = {
        "runtime": RUNTIME,
        "version": RUNTIME_VERSION,
        "dtype": "q4",
        "device": "webgpu",
    }
    if expected_runtime not in compatible:
        raise AssertionError(f"manifest lacks compatible runtime {expected_runtime!r}")
    corpus_ids = {item.get("corpus_id") for item in manifest.get("corpora", [])}
    if corpus_ids != set(CORPORA):
        raise AssertionError(f"manifest corpora mismatch: {corpus_ids!r}")


def validate_corpus_manifest(
    manifest: dict[str, object],
    docs: list[dict[str, object]],
    items: list[dict[str, object]],
    vectors: np.ndarray,
    input_hash: str,
    id_field: str,
) -> None:
    row_count = len(docs)
    if manifest.get("input_hash") != input_hash:
        raise AssertionError(f"{manifest['corpus_id']} input hash mismatch")
    if manifest.get("row_count") != row_count:
        raise AssertionError(f"{manifest['corpus_id']} row count mismatch")
    if manifest.get("dimensions") != DIMENSIONS:
        raise AssertionError(f"{manifest['corpus_id']} dimensions mismatch")
    if manifest.get("vector_byte_len") != row_count * DIMENSIONS * 2:
        raise AssertionError(f"{manifest['corpus_id']} vector byte length mismatch")
    if len(items) != row_count:
        raise AssertionError(f"{manifest['corpus_id']} items row count mismatch")
    if vectors.shape != (row_count, DIMENSIONS):
        raise AssertionError(f"{manifest['corpus_id']} vector shape mismatch: {vectors.shape}")
    for row, (item, doc) in enumerate(zip(items, docs, strict=True)):
        if item.get("row") != row:
            raise AssertionError(f"{manifest['corpus_id']} item row {row} has wrong row")
        if item.get(id_field) != doc["id"]:
            raise AssertionError(f"{manifest['corpus_id']} item row {row} has wrong id")
        if item.get("input_hash") != doc["inputHash"]:
            raise AssertionError(f"{manifest['corpus_id']} item row {row} has wrong input hash")


def read_vectors(path: Path, row_count: int) -> np.ndarray:
    data = path.read_bytes()
    expected = row_count * DIMENSIONS * 2
    if len(data) != expected:
        raise AssertionError(f"{path} byte length mismatch: expected {expected}, got {len(data)}")
    return np.frombuffer(data, dtype="<f2").reshape(row_count, DIMENSIONS)


def embed_texts(texts: list[str], tokenizer, session: ort.InferenceSession) -> np.ndarray:
    input_names = {item.name for item in session.get_inputs()}
    encoded = tokenizer(
        texts,
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
    return normalize(last_token_pool(hidden, attention_mask).astype(np.float32))


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


def sample_rows(row_count: int, count: int) -> list[int]:
    candidates = [0, 1, row_count // 2, row_count - 1]
    rows = []
    for row in candidates:
        if 0 <= row < row_count and row not in rows:
            rows.append(row)
        if len(rows) >= count:
            break
    return rows


def read_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def verify_file_sha256(path: Path, expected: str) -> None:
    actual = file_sha256(path)
    if actual != expected:
        raise AssertionError(f"{path} SHA-256 mismatch: expected {expected}, got {actual}")


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


if __name__ == "__main__":
    main()
