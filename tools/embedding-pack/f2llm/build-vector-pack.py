#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path
from typing import Iterable
from urllib.parse import urljoin, urlparse
from urllib.request import urlopen
from importlib.metadata import version

import numpy as np


SCHEMA_VERSION = 1
ARTIFACT_VERSION = "f2llm-vector-pack-windowed-ca-v1"
MODEL_KEY = "f2llm-v2-80m-q4-320"
MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
RUNTIME = "jbotci-webgpu-f2llm"
RUNTIME_VERSION = "0.2.0"
WASM_RUNTIME = "jbotci-onnxruntime-web-f2llm"
WASM_RUNTIME_VERSION = "0.2.0"
POOLING = "mean_normalized_windows"
VECTOR_SPACE_KEY = "jbotci-browser-f2llm-q4-f16-windowed-512-v1"
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
PROGRESS_WINDOW_INTERVAL = 512
BUILT_BY = {
    "runtime": "onnxruntime",
    "provider": "CPUExecutionProvider",
    "dtype": "q4",
    "source": "com.microsoft MatMulNBits/GatherBlockQuantized",
}


def main() -> None:
    args = parse_args()
    # Keep pack inspection and deterministic tool tests independent of model runtimes.
    import onnxruntime as ort
    from transformers import AutoTokenizer
    q4_onnx = Path(args.q4_onnx)
    tokenizer_dir = Path(args.tokenizer_dir) if args.tokenizer_dir else q4_onnx.parent.parent
    output = Path(args.out)
    # lexists, so a dangling symlink at --out also counts as present.
    if os.path.lexists(output) and not args.overwrite:
        # Refuse before any work starts: replacing an existing output
        # silently discarded a finished pack when several runs shared one
        # --out (2026-10-02 dictionary refresh).
        raise SystemExit(
            f"{output} already exists; pass --overwrite to replace it or choose another --out"
        )
    corpus = read_json(Path(args.input))
    stage = Path(args.stage) if args.stage else Path(f"{output}.staging")
    if stage == output:
        raise ValueError("--stage must differ from --out")
    tokenizer = AutoTokenizer.from_pretrained(tokenizer_dir, fix_mistral_regex=True)
    session_options = ort.SessionOptions()
    if args.threads is not None:
        # Throughput per thread falls off past a few threads, so a parallel
        # build runs several models at once, each limited to a few threads.
        session_options.intra_op_num_threads = args.threads
    session = ort.InferenceSession(
        str(q4_onnx), sess_options=session_options, providers=["CPUExecutionProvider"]
    )
    q4_onnx_sha256 = file_sha256(q4_onnx)
    identity = reuse_identity(args, corpus, q4_onnx_sha256, tokenizer)
    previous = load_previous_pack(args, identity)
    # Read and validate the source before clearing scratch or replacing output.
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    compatible_query_runtimes = compatible_runtimes(args.include_wasm_runtime, args.max_sequence_length)

    pack_id = "-".join([
        corpus["inputFormatVersion"],
        ARTIFACT_VERSION,
        short_hash(q4_onnx_sha256),
        short_hash(corpus["inputHash"]),
        args.vector_space_key,
    ])
    pack_root = stage / "models" / args.model_key / "spaces" / args.vector_space_key / "packs" / pack_id
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
            dimensions=args.dimensions,
            max_sequence_length=args.max_sequence_length,
            progress_label=f"{args.model_key} {corpus_id}",
            reusable_rows=previous.get(corpus_id, {}),
        ))

    manifest_url = f"models/{args.model_key}/spaces/{args.vector_space_key}/packs/{pack_id}/manifest.json"
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "artifact_version": ARTIFACT_VERSION,
        "reuse_identity": identity,
        "model_key": args.model_key,
        "model_revision": args.revision or "",
        "web_model": args.model_id,
        "q4_onnx_sha256": q4_onnx_sha256,
        "vector_space_key": args.vector_space_key,
        "pack_id": pack_id,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["inputHash"],
        "max_sequence_length": args.max_sequence_length,
        "pooling": POOLING,
        "max_window_tokens": args.max_sequence_length,
        "built_by": BUILT_BY,
        "dimensions": args.dimensions,
        "element_type": "f16le",
        "normalized": True,
        "distance": "dot",
        "compatible_query_runtimes": compatible_query_runtimes,
        "corpora": corpora,
    }
    write_json(pack_root / "manifest.json", manifest)
    catalog = {
        "schema_version": SCHEMA_VERSION,
        "models": [
            {
                "model_key": args.model_key,
                "vector_spaces": [
                    {
                        "vector_space_key": args.vector_space_key,
                        "latest_pack_id": pack_id,
                        "manifest_url": manifest_url,
                        "pooling": POOLING,
                        "max_window_tokens": args.max_sequence_length,
                        "compatible_query_runtimes": compatible_query_runtimes,
                    },
                ],
            },
        ],
    }
    write_json(stage / "catalog.json", catalog)
    promote(stage, output, args.overwrite)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build precomputed f16 vector packs for the custom F2LLM q4 WebGPU runtime."
    )
    parser.add_argument("--input", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", default=None)
    reuse = parser.add_mutually_exclusive_group()
    reuse.add_argument("--reuse-from", help="previous pack directory or manifest URL")
    reuse.add_argument("--reuse-catalog", help="previous catalog path or URL, selected by model and space")
    reuse.add_argument("--reuse-published", action="store_true", help="reuse the current public web catalog entry")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="replace --out if it already exists (refused by default)",
    )
    parser.add_argument("--q4-onnx", default=DEFAULT_Q4_ONNX)
    parser.add_argument("--tokenizer-dir", default=None)
    parser.add_argument("--model-key", default=MODEL_KEY)
    parser.add_argument("--model-id", default=MODEL_ID)
    parser.add_argument("--dimensions", type=int, default=DIMENSIONS)
    parser.add_argument("--vector-space-key", default=VECTOR_SPACE_KEY)
    parser.add_argument("--max-sequence-length", type=int, default=MAX_SEQUENCE_LENGTH)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument(
        "--threads",
        type=int,
        default=None,
        help="ONNX Runtime intra-op threads (default: one per core)",
    )
    parser.add_argument("--include-wasm-runtime", action="store_true")
    args = parser.parse_args()
    if args.batch_size <= 0:
        raise ValueError("--batch-size must be positive")
    if args.threads is not None and args.threads <= 0:
        raise ValueError("--threads must be positive")
    if args.dimensions <= 0:
        raise ValueError("--dimensions must be positive")
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    return args


def compatible_runtimes(include_wasm_runtime: bool, max_sequence_length: int) -> list[dict[str, object]]:
    runtimes = [
        {
            "runtime": RUNTIME,
            "version": RUNTIME_VERSION,
            "dtype": "q4",
            "device": "webgpu",
            "pooling": POOLING,
            "max_window_tokens": max_sequence_length,
        },
    ]
    if include_wasm_runtime:
        runtimes.append({
            "runtime": WASM_RUNTIME,
            "version": WASM_RUNTIME_VERSION,
            "dtype": "q4",
            "device": "wasm",
            "pooling": POOLING,
            "max_window_tokens": max_sequence_length,
        })
    return runtimes


def reuse_identity(args, corpus, model_hash: str, tokenizer) -> dict[str, object]:
    import onnx

    # The graph alone does not identify the tokenizer or numerical backend.
    # Hash the effective tokenizer after loading, including the special-token IDs.
    tokenizer_state = {
        "backend": tokenizer.backend_tokenizer.to_str(),
        "special_tokens": tokenizer.special_tokens_map,
        "eos": tokenizer.eos_token_id,
        "pad": tokenizer.pad_token_id,
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "artifact_version": ARTIFACT_VERSION,
        "model_key": args.model_key,
        "web_model": args.model_id,
        "model_revision": args.revision or "",
        "q4_onnx_sha256": model_hash,
        # External ONNX weights are not covered by the graph file's hash.
        "loaded_onnx_sha256": sha256(onnx.load(args.q4_onnx, load_external_data=True).SerializeToString(deterministic=True)),
        "vector_space_key": args.vector_space_key,
        "input_format_version": corpus["inputFormatVersion"],
        "max_sequence_length": args.max_sequence_length,
        "max_window_tokens": args.max_sequence_length,
        "pooling": POOLING,
        "dimensions": args.dimensions,
        "element_type": "f16le",
        "normalized": True,
        "distance": "dot",
        "tokenizer_sha256": sha256(json_bytes(tokenizer_state)),
        "windowing": "append-eos-contiguous-nonoverlapping-last-token-l2-mean-l2-v1",
        "backend": {name: version(name) for name in ("onnx", "onnxruntime", "transformers", "tokenizers", "numpy")},
        "provider": "CPUExecutionProvider",
        "built_by": BUILT_BY,
    }


def read_source(source: str) -> bytes:
    if urlparse(source).scheme in ("http", "https"):
        with urlopen(source, timeout=120) as response:
            return response.read()
    return Path(source).read_bytes()


def source_child(source: str, child: str) -> str:
    # A manifest names files below its own directory, never arbitrary local files.
    if not child or urlparse(child).scheme or "\\" in child or any(
        part in ("", ".", "..") for part in child.split("/")
    ):
        raise ValueError(f"invalid relative pack path: {child!r}")
    if urlparse(source).scheme in ("http", "https"):
        return urljoin(source, child)
    return str(Path(source).parent / child)


def load_previous_pack(args, identity: dict[str, object]) -> dict[str, dict[str, np.ndarray]]:
    catalog_source = (
        "https://assets.jbotci.app/embeddings/web/v1/catalog.json"
        if args.reuse_published else args.reuse_catalog
    )
    source = args.reuse_from
    if catalog_source:
        catalog = json.loads(read_source(catalog_source))
        entries = [
            space
            for model in catalog["models"] if model["model_key"] == args.model_key
            for space in model["vector_spaces"] if space["vector_space_key"] == args.vector_space_key
        ]
        if len(entries) != 1:
            raise ValueError("reuse catalog must contain exactly one matching model and vector space")
        source = source_child(catalog_source, entries[0]["manifest_url"])
    if not source:
        return {}
    if Path(source).is_dir():
        source = str(Path(source) / "manifest.json")
    manifest = json.loads(read_source(source))
    if manifest.get("reuse_identity") != identity:
        raise ValueError("incompatible reuse pack: reuse_identity differs or is missing (legacy pack)")
    for field in (
        "schema_version", "artifact_version", "model_key", "web_model", "model_revision",
        "q4_onnx_sha256", "vector_space_key", "input_format_version", "max_sequence_length",
        "max_window_tokens", "pooling", "dimensions", "element_type", "normalized", "distance", "built_by",
    ):
        if manifest.get(field) != identity[field]:
            raise ValueError(f"incompatible reuse pack: {field} differs")
    result = {}
    for corpus in manifest["corpora"]:
        corpus_id = corpus["corpus_id"]
        if corpus_id in result:
            raise ValueError(f"duplicate reuse corpus: {corpus_id}")
        files = {}
        for kind in ("items", "vector"):
            data = read_source(source_child(source, corpus[f"{kind}_url"]))
            if sha256(data) != corpus[f"{kind}_sha256"]:
                raise ValueError(f"{corpus_id}: {kind} SHA-256 mismatch")
            files[kind] = data
        count, dimensions = corpus["row_count"], identity["dimensions"]
        if (corpus["input_format_version"] != identity["input_format_version"]
                or corpus["dimensions"] != dimensions or count < 0
                or len(files["vector"]) != count * dimensions * 2
                or corpus["vector_byte_len"] != len(files["vector"])):
            raise ValueError(f"{corpus_id}: invalid reuse vector shape")
        items = json.loads(files["items"])
        if len(items) != count:
            raise ValueError(f"{corpus_id}: invalid reuse item count")
        vectors = np.frombuffer(files["vector"], dtype="<f2").reshape(count, dimensions)
        if not np.isfinite(vectors).all():
            raise ValueError(f"{corpus_id}: non-finite reuse vectors")
        rows = {}
        for row, item in enumerate(items):
            digest = item["input_hash"]
            if item["row"] != row or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
                raise ValueError(f"{corpus_id}: invalid reuse row or input hash")
            if digest in rows and not np.array_equal(rows[digest], vectors[row]):
                raise ValueError(f"{corpus_id}: conflicting vectors for the same input hash")
            rows[digest] = vectors[row]
        result[corpus_id] = rows
    if set(result) != {corpus_id for corpus_id, _, _ in CORPORA}:
        raise ValueError("reuse pack must contain both corpora")
    return result


def write_corpus(
    pack_root: Path,
    corpus: dict[str, object],
    corpus_id: str,
    source_key: str,
    id_field: str,
    tokenizer,
    session: ort.InferenceSession,
    batch_size: int,
    dimensions: int,
    max_sequence_length: int,
    progress_label: str,
    reusable_rows: dict[str, np.ndarray] | None = None,
) -> dict[str, object]:
    docs = corpus.get(source_key, [])
    if not isinstance(docs, list):
        raise ValueError(f"corpus field {source_key} must be an array")
    reusable_rows = reusable_rows or {}
    # Verify the exact text rather than trusting an unchecked export's inputHash.
    for doc in docs:
        if sha256(doc["input"].encode("utf-8")) != doc["inputHash"]:
            raise ValueError(f"{corpus_id}: inputHash does not match exact input text")
    missing = [row for row, doc in enumerate(docs) if doc["inputHash"] not in reusable_rows]
    fresh = embed_texts(
        [docs[row]["input"] for row in missing],
        tokenizer,
        session,
        batch_size,
        dimensions,
        max_sequence_length,
        progress_label,
    )
    vectors = np.empty((len(docs), dimensions), dtype="<f2")
    fresh_rows = iter(fresh)
    for row, doc in enumerate(docs):
        reused = reusable_rows.get(doc["inputHash"])
        vectors[row] = reused if reused is not None else next(fresh_rows)
    print(f"{progress_label}: reused {len(docs) - len(missing)} rows, embedded {len(missing)} rows", flush=True)
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
    items_bytes = json_bytes(items)
    items_sha256 = sha256(items_bytes)
    items_name = f"items.{items_sha256}.json"
    (corpus_dir / items_name).write_bytes(items_bytes)
    vector_bytes = vectors.astype("<f2", copy=False).tobytes()
    vector_sha256 = sha256(vector_bytes)
    vector_name = f"vectors.{vector_sha256}.f16"
    (corpus_dir / vector_name).write_bytes(vector_bytes)
    return {
        "corpus_id": corpus_id,
        "input_format_version": corpus["inputFormatVersion"],
        "input_hash": corpus["dictionaryHash"] if corpus_id == "vlacku-en" else corpus["cllHash"],
        "row_count": len(docs),
        "dimensions": dimensions,
        "items_url": f"corpora/{corpus_id}/{items_name}",
        "items_sha256": items_sha256,
        "vector_url": f"corpora/{corpus_id}/{vector_name}",
        "vector_byte_len": len(vector_bytes),
        "vector_sha256": vector_sha256,
    }


def embed_texts(
    texts: list[str],
    tokenizer,
    session: ort.InferenceSession,
    batch_size: int,
    dimensions: int,
    max_sequence_length: int,
    progress_label: str,
) -> np.ndarray:
    doc_windows = [token_windows(text, tokenizer, max_sequence_length) for text in texts]
    window_refs = [
        (doc_index, window)
        for doc_index, windows in enumerate(doc_windows)
        for window in windows
    ]
    vectors_by_doc: list[list[np.ndarray]] = [[] for _ in texts]
    input_names = {item.name for item in session.get_inputs()}
    for start, batch in enumerate_batches(window_refs, batch_size):
        doc_indices = [doc_index for doc_index, _ in batch]
        windows = [window for _, window in batch]
        input_ids, attention_mask = padded_window_batch(windows, tokenizer)
        feeds = {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
        }
        if "position_ids" in input_names:
            feeds["position_ids"] = position_ids(attention_mask)
        hidden = session.run(None, feeds)[0]
        rows = last_token_pool(hidden, attention_mask).astype(np.float32)
        rows = normalize(rows)
        if rows.shape[1] != dimensions:
            raise ValueError(f"embedding dimension mismatch: expected {dimensions}, got {rows.shape[1]}")
        for doc_index, row in zip(doc_indices, rows, strict=True):
            vectors_by_doc[doc_index].append(row)
        done_windows = min(start + len(batch), len(window_refs))
        if done_windows == len(window_refs) or done_windows % PROGRESS_WINDOW_INTERVAL == 0:
            completed_docs = sum(1 for vectors in vectors_by_doc if vectors)
            print(
                # The label names the model and corpus, so the output of
                # several concurrent builds stays attributable.
                f"{progress_label}: embedded {done_windows} of {len(window_refs)} windows "
                f"for {completed_docs} of {len(texts)} documents",
                flush=True,
            )
    if not vectors_by_doc:
        return np.empty((0, dimensions), dtype=np.float32)
    return np.stack([
        mean_pool_normalized(window_vectors, dimensions)
        for window_vectors in vectors_by_doc
    ], axis=0)


def token_windows(text: str, tokenizer, max_sequence_length: int) -> list[np.ndarray]:
    token_ids = list(tokenizer(str(text), add_special_tokens=False, truncation=False)["input_ids"])
    eos_token_id = tokenizer.eos_token_id
    if eos_token_id is None:
        raise ValueError("F2LLM tokenizer does not expose eos_token_id")
    token_ids.append(int(eos_token_id))
    return [
        np.asarray(token_ids[start:start + max_sequence_length], dtype=np.int64)
        for start in range(0, len(token_ids), max_sequence_length)
    ]


def padded_window_batch(windows: list[np.ndarray], tokenizer) -> tuple[np.ndarray, np.ndarray]:
    if not windows:
        raise ValueError("cannot embed an empty window batch")
    max_len = max(len(window) for window in windows)
    pad_token_id = tokenizer.pad_token_id
    if pad_token_id is None:
        pad_token_id = tokenizer.eos_token_id
    if pad_token_id is None:
        raise ValueError("F2LLM tokenizer does not expose pad_token_id or eos_token_id")
    input_ids = np.full((len(windows), max_len), int(pad_token_id), dtype=np.int64)
    attention_mask = np.zeros((len(windows), max_len), dtype=np.int64)
    for row, window in enumerate(windows):
        if len(window) == 0:
            raise ValueError("token window cannot be empty")
        input_ids[row, :len(window)] = window
        attention_mask[row, :len(window)] = 1
    return input_ids, attention_mask


def enumerate_batches(items: list[object], batch_size: int) -> Iterable[tuple[int, list[object]]]:
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


def mean_pool_normalized(window_vectors: list[np.ndarray], dimensions: int) -> np.ndarray:
    if not window_vectors:
        raise ValueError("document produced no token windows")
    stacked = np.stack(window_vectors, axis=0).astype(np.float32)
    if stacked.shape[1] != dimensions:
        raise ValueError(f"embedding dimension mismatch: expected {dimensions}, got {stacked.shape[1]}")
    mean = stacked.mean(axis=0, dtype=np.float32).reshape(1, dimensions)
    return normalize(mean)[0]


def read_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(json_bytes(value))


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def promote(stage: Path, output: Path, overwrite: bool) -> None:
    # Check again at the end of a long run: an output that appeared meanwhile
    # (for example from an overlapping run) is not replaced either.
    if os.path.lexists(output) and not overwrite:
        raise SystemExit(
            f"{output} appeared during the build; pass --overwrite to replace it"
        )
    # `<output>.previous` is this function's own rollback copy, never a build
    # output, so it is always removed.
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
