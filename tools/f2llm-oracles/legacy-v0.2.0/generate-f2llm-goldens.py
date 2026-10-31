#!/usr/bin/env python3
"""Generate versioned F2LLM windowing oracles from a published q4 ONNX model."""

from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import onnxruntime as ort
import transformers
from transformers import AutoTokenizer


SCHEMA_VERSION = 2
RUNTIME = "jbotci-webgpu-f2llm"
RUNTIME_VERSION = "0.2.0"
MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
MODEL_KEY = "f2llm-v2-80m-q4-320"
MODEL_REVISION = "f4a16a11c9f5c8c7e22694653de6ce75430f4538"
DIMENSIONS = 320
MAX_SEQUENCE_LENGTH = 512
DEFAULT_BATCH_SIZE = 8
QUERY_PREFIX = (
    "Instruct: Given a question, retrieve passages that can help answer the question.\n"
    "Query: "
)


@dataclass(frozen=True)
class Case:
    name: str
    kind: str
    input: str
    expected_token_count: int | None = None


@dataclass(frozen=True)
class WindowReference:
    case_index: int
    window_index: int
    token_ids: np.ndarray


def main() -> None:
    args = parse_args()
    generator_path = Path(__file__).resolve()
    generator_sha256 = sha256(generator_path.read_bytes())
    onnx_bytes = args.q4_onnx.read_bytes()
    onnx_sha256 = sha256(onnx_bytes)
    require_digest("q4 ONNX", onnx_sha256, args.expected_q4_sha256)

    onnx_manifest_bytes = args.onnx_manifest.read_bytes()
    onnx_manifest_sha256 = sha256(onnx_manifest_bytes)
    require_digest(
        "ONNX manifest",
        onnx_manifest_sha256,
        args.expected_onnx_manifest_sha256,
    )
    onnx_manifest = json.loads(onnx_manifest_bytes)
    validate_onnx_manifest(onnx_manifest, onnx_sha256, len(onnx_bytes), args)

    artifact_manifest_bytes = args.artifact_manifest.read_bytes()
    artifact_manifest_sha256 = sha256(artifact_manifest_bytes)
    require_digest(
        "WebGPU artifact manifest",
        artifact_manifest_sha256,
        args.expected_artifact_manifest_sha256,
    )
    artifact_manifest = json.loads(artifact_manifest_bytes)
    validate_artifact_manifest(artifact_manifest, args)

    tokenizer = AutoTokenizer.from_pretrained(
        args.model,
        revision=args.revision,
        fix_mistral_regex=True,
    )
    cases = build_cases(tokenizer)
    case_windows = [
        token_windows(case.input, tokenizer, args.max_sequence_length) for case in cases
    ]
    for case, windows in zip(cases, case_windows, strict=True):
        token_count = sum(len(window) for window in windows)
        if case.expected_token_count is not None and token_count != case.expected_token_count:
            raise ValueError(
                f"{case.name} token count mismatch: "
                f"expected {case.expected_token_count}, got {token_count}"
            )

    references = [
        WindowReference(case_index, window_index, window)
        for case_index, windows in enumerate(case_windows)
        for window_index, window in enumerate(windows)
    ]
    validate_batch_boundary_cases(cases, references, args.batch_size)
    vectors_by_case, vectors_by_window, execution_batches = encode_windows(
        args,
        tokenizer,
        references,
        len(cases),
    )

    output_cases = []
    for case_index, (case, windows, embedding) in enumerate(
        zip(cases, case_windows, vectors_by_case, strict=True)
    ):
        embedding = embedding.astype("<f4", copy=False)
        window_embeddings = []
        for window_index, window in enumerate(windows):
            window_embedding = vectors_by_window[(case_index, window_index)].astype(
                "<f4", copy=False
            )
            window_embeddings.append(
                {
                    "embedding": [float(value) for value in window_embedding],
                    "embedding_f32le_sha256": sha256(window_embedding.tobytes()),
                }
            )
        output_cases.append(
            {
                "name": case.name,
                "kind": case.kind,
                "input": case.input,
                "input_sha256": sha256(case.input.encode("utf-8")),
                "token_ids": [
                    int(token_id) for window in windows for token_id in window
                ],
                "token_count": sum(len(window) for window in windows),
                "windows": [
                    [int(token_id) for token_id in window] for window in windows
                ],
                "window_token_counts": [len(window) for window in windows],
                "window_embeddings": window_embeddings,
                "embedding": [float(value) for value in embedding],
                "embedding_f32le_sha256": sha256(embedding.tobytes()),
            }
        )

    output = {
        "schema_version": SCHEMA_VERSION,
        "reference": {
            "generator": {
                "repository_path": "tools/f2llm-oracles/generate-f2llm-goldens.py",
                "sha256": generator_sha256,
            },
            "published_onnx": {
                "r2_manifest_key": args.onnx_manifest_r2_key,
                "manifest_sha256": onnx_manifest_sha256,
                "r2_model_key": args.q4_onnx_r2_key,
                "model_sha256": onnx_sha256,
                "model_byte_length": len(onnx_bytes),
                "runtime": "onnxruntime",
                "runtime_version": ort.__version__,
                "provider": "CPUExecutionProvider",
            },
            "target_artifact": {
                "r2_manifest_key": args.artifact_manifest_r2_key,
                "manifest_sha256": artifact_manifest_sha256,
                "runtime": artifact_manifest["runtime"],
                "artifact_version": artifact_manifest["artifact_version"],
            },
            "tokenizer": {
                "model": args.model,
                "revision": args.revision,
                "transformers_version": transformers.__version__,
                "eos_token_id": int(tokenizer.eos_token_id),
                "pad_token_id": (
                    None if tokenizer.pad_token_id is None else int(tokenizer.pad_token_id)
                ),
            },
        },
        "model_key": args.model_key,
        "runtime": RUNTIME,
        "runtime_version": RUNTIME_VERSION,
        "dimensions": args.dimensions,
        "max_sequence_length": args.max_sequence_length,
        "window_pooling": "last-token",
        "pooling": "mean_normalized_windows",
        "normalized": True,
        "cosine_threshold": args.cosine_threshold,
        "execution": {
            "window_batch_size": args.batch_size,
            "window_count": len(references),
            "batch_count": len(execution_batches),
            "batches": execution_batches,
        },
        "cases": output_cases,
    }
    write_json(args.out, output)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Generate F2LLM 0.2.0 windowing goldens from immutable published "
            "ONNX and WebGPU manifests."
        )
    )
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--q4-onnx", type=Path, required=True)
    parser.add_argument("--onnx-manifest", type=Path, required=True)
    parser.add_argument("--artifact-manifest", type=Path, required=True)
    parser.add_argument("--expected-q4-sha256", required=True)
    parser.add_argument("--expected-onnx-manifest-sha256", required=True)
    parser.add_argument("--expected-artifact-manifest-sha256", required=True)
    parser.add_argument(
        "--q4-onnx-r2-key",
        default="models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx",
    )
    parser.add_argument(
        "--onnx-manifest-r2-key",
        default="models/f2llm-v2-80m-onnx-q4/v1/manifest.json",
    )
    parser.add_argument(
        "--artifact-manifest-r2-key",
        default="models/f2llm-v2-80m-webgpu/v1/manifest.json",
    )
    parser.add_argument("--model", default=MODEL_ID)
    parser.add_argument("--model-key", default=MODEL_KEY)
    parser.add_argument("--revision", default=MODEL_REVISION)
    parser.add_argument("--dimensions", type=int, default=DIMENSIONS)
    parser.add_argument("--max-sequence-length", type=int, default=MAX_SEQUENCE_LENGTH)
    parser.add_argument("--batch-size", type=int, default=DEFAULT_BATCH_SIZE)
    parser.add_argument("--cosine-threshold", type=float, default=0.999)
    args = parser.parse_args()
    if args.batch_size <= 0:
        raise ValueError("--batch-size must be positive")
    if args.dimensions <= 0:
        raise ValueError("--dimensions must be positive")
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    if not 0.0 < args.cosine_threshold <= 1.0:
        raise ValueError("--cosine-threshold must be in (0, 1]")
    return args


def build_cases(tokenizer) -> list[Case]:
    cases = [
        Case("empty", "edge", ""),
        Case("non-ascii", "edge", "naïve café — coi ro do 🙂"),
        Case("query-coi-ro-do", "query", QUERY_PREFIX + "coi ro do"),
        Case("query-klama-zarci", "query", QUERY_PREFIX + "mi klama le zarci"),
        Case(
            "document-klama-definition",
            "document",
            (
                "title: klama | text: x1 comes/goes to destination x2 from origin x3 "
                "via route x4 using means x5"
            ),
        ),
        Case("batch-filler-5", "batch-boundary", "batch boundary filler five"),
        Case("batch-filler-6", "batch-boundary", "batch boundary filler six"),
        Case("batch-last-slot", "batch-boundary", "last item in the first batch"),
        Case("batch-next-slot", "batch-boundary", "first item in the second batch"),
    ]
    for token_count in (511, 512, 513):
        cases.append(
            Case(
                f"token-length-{token_count}",
                "token-boundary",
                exact_token_count_input(tokenizer, token_count),
                expected_token_count=token_count,
            )
        )
    cases.append(
        Case(
            "multi-window-1025",
            "multi-window",
            exact_token_count_input(tokenizer, 1025),
            expected_token_count=1025,
        )
    )
    return cases


def exact_token_count_input(tokenizer, token_count: int) -> str:
    if token_count <= 1:
        raise ValueError("exact token count must leave room for the appended EOS token")
    unit_ids = tokenizer("a", add_special_tokens=False, truncation=False)["input_ids"]
    separated_unit_ids = tokenizer(" a", add_special_tokens=False, truncation=False)[
        "input_ids"
    ]
    if len(unit_ids) != 1 or len(separated_unit_ids) != 1:
        raise ValueError(
            "the pinned tokenizer no longer maps `a` and ` a` to one token; "
            "the explicit boundary construction must be reviewed"
        )
    text = " ".join("a" for _ in range(token_count - 1))
    actual = tokenizer(text, add_special_tokens=False, truncation=False)["input_ids"]
    if len(actual) + 1 != token_count:
        raise ValueError(
            f"explicit boundary construction produced {len(actual) + 1} tokens, "
            f"expected {token_count}"
        )
    return text


def token_windows(text: str, tokenizer, max_sequence_length: int) -> list[np.ndarray]:
    token_ids = list(
        tokenizer(str(text), add_special_tokens=False, truncation=False)["input_ids"]
    )
    eos_token_id = tokenizer.eos_token_id
    if eos_token_id is None:
        raise ValueError("F2LLM tokenizer does not expose eos_token_id")
    token_ids.append(int(eos_token_id))
    return [
        np.asarray(
            token_ids[start : start + max_sequence_length],
            dtype=np.int64,
        )
        for start in range(0, len(token_ids), max_sequence_length)
    ]


def validate_batch_boundary_cases(
    cases: list[Case],
    references: list[WindowReference],
    batch_size: int,
) -> None:
    by_name = {
        case.name: next(
            index
            for index, reference in enumerate(references)
            if reference.case_index == case_index
        )
        for case_index, case in enumerate(cases)
    }
    last_slot = by_name["batch-last-slot"]
    next_slot = by_name["batch-next-slot"]
    if last_slot % batch_size != batch_size - 1:
        raise ValueError("batch-last-slot is not the last item of an execution batch")
    if next_slot != last_slot + 1 or next_slot % batch_size != 0:
        raise ValueError("batch-next-slot is not the first item of the next execution batch")


def encode_windows(
    args: argparse.Namespace,
    tokenizer,
    references: list[WindowReference],
    case_count: int,
) -> tuple[list[np.ndarray], dict[tuple[int, int], np.ndarray], list[dict[str, object]]]:
    session = ort.InferenceSession(
        str(args.q4_onnx),
        providers=["CPUExecutionProvider"],
    )
    input_names = {item.name for item in session.get_inputs()}
    vectors_by_case: list[list[np.ndarray]] = [[] for _ in range(case_count)]
    vectors_by_window: dict[tuple[int, int], np.ndarray] = {}
    execution_batches = []
    for batch_index, (start, batch) in enumerate(
        enumerate_batches(references, args.batch_size)
    ):
        windows = [reference.token_ids for reference in batch]
        input_ids, attention_mask = padded_window_batch(windows, tokenizer)
        feeds = {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
        }
        if "position_ids" in input_names:
            feeds["position_ids"] = position_ids(attention_mask)
        output = session.run(None, feeds)[0]
        rows = pool_onnx_output(output, attention_mask).astype(np.float32)
        rows = normalize(rows)
        if rows.shape != (len(batch), args.dimensions):
            raise ValueError(
                f"embedding shape mismatch: expected {(len(batch), args.dimensions)}, "
                f"got {rows.shape}"
            )
        batch_references = []
        for reference, row in zip(batch, rows, strict=True):
            vectors_by_case[reference.case_index].append(row)
            vectors_by_window[(reference.case_index, reference.window_index)] = row
            batch_references.append(
                {
                    "case_index": reference.case_index,
                    "window_index": reference.window_index,
                }
            )
        execution_batches.append(
            {
                "batch_index": batch_index,
                "first_window": start,
                "window_count": len(batch),
                "padded_sequence_length": int(input_ids.shape[1]),
                "windows": batch_references,
            }
        )
    pooled = [
        mean_pool_normalized(window_vectors, args.dimensions)
        for window_vectors in vectors_by_case
    ]
    return pooled, vectors_by_window, execution_batches


def padded_window_batch(
    windows: list[np.ndarray],
    tokenizer,
) -> tuple[np.ndarray, np.ndarray]:
    if not windows:
        raise ValueError("cannot embed an empty window batch")
    max_len = max(len(window) for window in windows)
    pad_token_id = tokenizer.pad_token_id
    if pad_token_id is None:
        pad_token_id = tokenizer.eos_token_id
    if pad_token_id is None:
        raise ValueError("F2LLM tokenizer does not expose pad_token_id or eos_token_id")
    input_ids = np.full(
        (len(windows), max_len),
        int(pad_token_id),
        dtype=np.int64,
    )
    attention_mask = np.zeros((len(windows), max_len), dtype=np.int64)
    for row, window in enumerate(windows):
        if len(window) == 0:
            raise ValueError("token window cannot be empty")
        input_ids[row, : len(window)] = window
        attention_mask[row, : len(window)] = 1
    return input_ids, attention_mask


def enumerate_batches(
    items: list[WindowReference],
    batch_size: int,
) -> Iterable[tuple[int, list[WindowReference]]]:
    for index in range(0, len(items), batch_size):
        yield index, items[index : index + batch_size]


def position_ids(attention_mask: np.ndarray) -> np.ndarray:
    positions = np.cumsum(attention_mask, axis=1, dtype=np.int64) - 1
    positions[attention_mask == 0] = 0
    return positions


def pool_onnx_output(output: np.ndarray, attention_mask: np.ndarray) -> np.ndarray:
    if output.ndim == 2:
        return output
    if output.ndim == 3:
        lengths = attention_mask.sum(axis=1).astype(np.int64)
        return output[np.arange(output.shape[0]), lengths - 1, :]
    raise ValueError(f"unsupported ONNX output shape: {output.shape}")


def normalize(values: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(values, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    return values / norms


def mean_pool_normalized(
    window_vectors: list[np.ndarray],
    dimensions: int,
) -> np.ndarray:
    if not window_vectors:
        raise ValueError("case produced no token windows")
    stacked = np.stack(window_vectors, axis=0).astype(np.float32)
    if stacked.shape[1] != dimensions:
        raise ValueError(
            f"embedding dimension mismatch: expected {dimensions}, got {stacked.shape[1]}"
        )
    mean = stacked.mean(axis=0, dtype=np.float32).reshape(1, dimensions)
    return normalize(mean)[0]


def validate_onnx_manifest(
    manifest: dict[str, object],
    model_sha256: str,
    model_byte_length: int,
    args: argparse.Namespace,
) -> None:
    expected = {
        "schema_version": 1,
        "runtime": "jbotci-onnxruntime-web-f2llm",
        "artifact_version": RUNTIME_VERSION,
        "model_key": args.model_key,
        "source_model": args.model,
        "model_url": "model_q4.onnx",
        "model_byte_length": model_byte_length,
        "model_sha256": model_sha256,
        "max_sequence_length": args.max_sequence_length,
        "dimensions": args.dimensions,
    }
    for key, expected_value in expected.items():
        if manifest.get(key) != expected_value:
            raise ValueError(
                f"ONNX manifest field {key!r}: "
                f"expected {expected_value!r}, got {manifest.get(key)!r}"
            )


def validate_artifact_manifest(
    manifest: dict[str, object],
    args: argparse.Namespace,
) -> None:
    expected = {
        "schema_version": 1,
        "runtime": RUNTIME,
        "artifact_version": RUNTIME_VERSION,
        "model_key": args.model_key,
        "source_model": args.model,
        "max_sequence_length": args.max_sequence_length,
    }
    for key, expected_value in expected.items():
        if manifest.get(key) != expected_value:
            raise ValueError(
                f"WebGPU artifact manifest field {key!r}: "
                f"expected {expected_value!r}, got {manifest.get(key)!r}"
            )
    model = manifest.get("model")
    if not isinstance(model, dict) or model.get("hidden_size") != args.dimensions:
        raise ValueError("WebGPU artifact dimensions do not match --dimensions")


def require_digest(label: str, actual: str, expected: str) -> None:
    if actual != expected:
        raise ValueError(f"{label} SHA-256 mismatch: expected {expected}, got {actual}")


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    )


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


if __name__ == "__main__":
    main()
