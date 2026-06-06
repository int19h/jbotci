#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
from pathlib import Path

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper


SCHEMA_VERSION = 1
ARTIFACT_VERSION = "0.2.0"
RUNTIME = "jbotci-webgpu-f2llm"
DEFAULT_MODEL_KEY = "f2llm-v2-80m-q4-320"
DEFAULT_SOURCE_MODEL = "codefuse-ai/F2LLM-v2-80M"
DEFAULT_MAX_SEQUENCE_LENGTH = 512
DEFAULT_SHARD_SIZE = 4 * 1024 * 1024
DEFAULT_ONNX_MODEL = (
    "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/"
    "f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx"
)
MATMUL_NODE_RE = re.compile(
    r"^/layers\.(?P<layer>\d+)/(?P<section>self_attn|mlp)/"
    r"(?P<projection>q_proj|k_proj|v_proj|o_proj|gate_proj|up_proj|down_proj)/MatMul_Q4$"
)


def main() -> None:
    args = parse_args()
    onnx_model_path = Path(args.onnx_model)
    model_root = Path(args.model_root) if args.model_root else onnx_model_path.parent.parent
    output = Path(args.out)
    stage = Path(args.stage) if args.stage else Path(f"{output}.staging")
    if stage == output:
        raise ValueError("--stage must differ from --out")
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)

    model = onnx.load(onnx_model_path, load_external_data=False)
    initializers = {initializer.name: initializer for initializer in model.graph.initializer}
    config = read_json(model_root / "config.json")
    manifest = build_manifest(config, args, onnx_model_path)
    manifest["tokenizer"] = write_tokenizer(model_root, stage)

    tensors: dict[str, object] = {}
    write_embedding_tensor(tensors, stage, model, initializers, args.shard_size)
    write_f32_tensor(
        tensors,
        stage,
        "model.norm.weight",
        required_initializer_array(initializers, "norm.weight"),
        args.shard_size,
    )
    for layer in range(int(config["num_hidden_layers"])):
        prefix = f"model.layers.{layer}"
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.input_layernorm.weight",
            required_initializer_array(initializers, f"layers.{layer}.input_layernorm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.post_attention_layernorm.weight",
            required_initializer_array(initializers, f"layers.{layer}.post_attention_layernorm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.self_attn.q_norm.weight",
            required_initializer_array(initializers, f"layers.{layer}.self_attn.q_norm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.self_attn.k_norm.weight",
            required_initializer_array(initializers, f"layers.{layer}.self_attn.k_norm.weight"),
            args.shard_size,
        )
    write_matmul_tensors(tensors, stage, model, initializers, args.shard_size)
    manifest["tensors"] = tensors
    validate_manifest_shapes(manifest)
    write_json(stage / "manifest.json", manifest)
    promote(stage, output)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export F2LLM q4 ONNX weights as the jbotci custom WebGPU artifact."
    )
    parser.add_argument("--onnx-model", default=DEFAULT_ONNX_MODEL)
    parser.add_argument("--model-root", default=None)
    parser.add_argument("--model-key", default=DEFAULT_MODEL_KEY)
    parser.add_argument("--source-model", default=DEFAULT_SOURCE_MODEL)
    parser.add_argument("--source-revision", default=None)
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", default=None)
    parser.add_argument("--shard-size", type=int, default=DEFAULT_SHARD_SIZE)
    parser.add_argument("--max-sequence-length", type=int, default=DEFAULT_MAX_SEQUENCE_LENGTH)
    args = parser.parse_args()
    if args.shard_size <= 0:
        raise ValueError("--shard-size must be positive")
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    return args


def build_manifest(config: dict[str, object], args: argparse.Namespace, onnx_model_path: Path) -> dict[str, object]:
    return {
        "schema_version": SCHEMA_VERSION,
        "runtime": RUNTIME,
        "artifact_version": ARTIFACT_VERSION,
        "model_key": args.model_key,
        "source_model": args.source_model,
        "source_revision": args.source_revision,
        "source_quantized_onnx": str(onnx_model_path),
        "max_sequence_length": args.max_sequence_length,
        "quantization": {
            "kind": "onnx_hqq_q4_block32",
            "matmul_operator": "com.microsoft.MatMulNBits",
            "gather_operator": "com.microsoft.GatherBlockQuantized",
            "bits": 4,
            "block_size": 32,
            "dequantization": "(q - zero_point) * scale",
            "pack_order": "low_nibble_first",
        },
        "model": {
            "vocab_size": int(config["vocab_size"]),
            "hidden_size": int(config["hidden_size"]),
            "num_hidden_layers": int(config["num_hidden_layers"]),
            "num_attention_heads": int(config["num_attention_heads"]),
            "num_key_value_heads": int(config["num_key_value_heads"]),
            "head_dim": int(config["head_dim"]),
            "intermediate_size": int(config["intermediate_size"]),
            "rms_norm_eps": float(config["rms_norm_eps"]),
            "rope_theta": float(config["rope_theta"]),
        },
    }


def write_tokenizer(model_root: Path, out_root: Path) -> dict[str, object]:
    tokenizer = read_json(model_root / "tokenizer.json")
    compact = {
        "schema_version": SCHEMA_VERSION,
        "tokenizer_type": "qwen2-byte-bpe",
        "normalizer": "NFC",
        "pre_tokenizer": "qwen2_regex_bytelevel",
        "post_processor": "append_eos",
        "vocab": tokenizer["model"]["vocab"],
        "merges": tokenizer["model"]["merges"],
        "special_tokens": {
            "eos_token": "<|im_end|>",
            "eos_id": 151645,
        },
    }
    data = canonical_json(compact)
    digest = sha256(data)
    path = out_root / f"tokenizer.{digest[:16]}.compact.json"
    path.write_bytes(data)
    return {
        "url": path.name,
        "byte_length": len(data),
        "canonical_json_sha256": digest,
    }


def write_embedding_tensor(
    tensors: dict[str, object],
    out_root: Path,
    model: onnx.ModelProto,
    initializers: dict[str, onnx.TensorProto],
    shard_size: int,
) -> None:
    node = single_node(model, "GatherBlockQuantized")
    attrs = node_attrs(node)
    if attrs.get("block_size") != 32 or attrs.get("gather_axis") != 0 or attrs.get("quantize_axis") != 1:
        raise ValueError(f"unsupported GatherBlockQuantized attrs: {attrs}")
    qweight_name, indices_name, scales_name, zero_points_name = node.input
    if indices_name != "input_ids":
        raise ValueError(f"unexpected GatherBlockQuantized indices input: {indices_name}")
    qweight = required_initializer(initializers, qweight_name)
    scales = required_initializer_array(initializers, scales_name).astype("<f4", copy=False)
    zero_points = required_initializer(initializers, zero_points_name)
    shape = list(qweight.dims)
    if len(shape) != 2 or shape[1] % attrs["block_size"] != 0:
        raise ValueError(f"unsupported embedding qweight shape: {shape}")
    groups = shape[1] // attrs["block_size"]
    if list(scales.shape) != [shape[0], groups]:
        raise ValueError(f"embedding scales shape mismatch: expected {[shape[0], groups]}, got {list(scales.shape)}")
    if list(zero_points.dims) != [shape[0], groups]:
        raise ValueError(
            f"embedding zero_points shape mismatch: expected {[shape[0], groups]}, got {list(zero_points.dims)}"
        )
    tensor_root = out_root / "tensors" / safe_tensor_path("model.embed_tokens.weight")
    tensors["model.embed_tokens.weight"] = {
        "kind": "q4_onnx_gather",
        "shape": [int(shape[0]), int(shape[1])],
        "group_size": int(attrs["block_size"]),
        "groups": int(groups),
        "qweight": write_chunked(
            out_root,
            tensor_root,
            "qweight",
            packed_uint4_initializer(qweight, shape[0] * shape[1]),
            shard_size,
        ),
        "scales": write_chunked(out_root, tensor_root, "scales.f32", scales.tobytes(), shard_size),
        "zero_points": write_chunked(
            out_root,
            tensor_root,
            "zero_points.u4",
            packed_uint4_initializer(zero_points, shape[0] * groups),
            shard_size,
        ),
    }


def write_matmul_tensors(
    tensors: dict[str, object],
    out_root: Path,
    model: onnx.ModelProto,
    initializers: dict[str, onnx.TensorProto],
    shard_size: int,
) -> None:
    seen = set()
    for node in model.graph.node:
        if node.op_type != "MatMulNBits":
            continue
        match = MATMUL_NODE_RE.match(node.name)
        if match is None:
            raise ValueError(f"unsupported MatMulNBits node name: {node.name}")
        attrs = node_attrs(node)
        if attrs.get("bits") != 4 or attrs.get("block_size") != 32:
            raise ValueError(f"unsupported MatMulNBits attrs for {node.name}: {attrs}")
        layer = int(match.group("layer"))
        section = match.group("section")
        projection = match.group("projection")
        tensor_name = f"model.layers.{layer}.{section}.{projection}.weight"
        qweight_name, scales_name, zero_points_name = node.input[1:4]
        qweight = required_initializer(initializers, qweight_name)
        scales = required_initializer_array(initializers, scales_name).astype("<f4", copy=False).reshape(-1)
        zero_points = required_initializer_array(initializers, zero_points_name).astype("<f4", copy=False).reshape(-1)
        out_cols = int(attrs["N"])
        in_cols = int(attrs["K"])
        groups = math.ceil(in_cols / int(attrs["block_size"]))
        expected_qweight_shape = [out_cols, groups, int(attrs["block_size"]) // 2]
        if list(qweight.dims) != expected_qweight_shape:
            raise ValueError(
                f"{tensor_name} qweight shape mismatch: expected {expected_qweight_shape}, got {list(qweight.dims)}"
            )
        if scales.size != out_cols * groups:
            raise ValueError(f"{tensor_name} scales length mismatch: expected {out_cols * groups}, got {scales.size}")
        if zero_points.size != out_cols * groups:
            raise ValueError(
                f"{tensor_name} zero_points length mismatch: expected {out_cols * groups}, got {zero_points.size}"
            )
        tensor_root = out_root / "tensors" / safe_tensor_path(tensor_name)
        tensors[tensor_name] = {
            "kind": "q4_onnx_matmul",
            "shape": [out_cols, in_cols],
            "group_size": int(attrs["block_size"]),
            "groups": groups,
            "qweight": write_chunked(
                out_root,
                tensor_root,
                "qweight",
                bytes(required_initializer(initializers, qweight_name).raw_data),
                shard_size,
            ),
            "scales": write_chunked(out_root, tensor_root, "scales.f32", scales.tobytes(), shard_size),
            "zero_points": write_chunked(
                out_root,
                tensor_root,
                "zero_points.f32",
                zero_points.tobytes(),
                shard_size,
            ),
        }
        seen.add(tensor_name)
    if len(seen) == 0:
        raise ValueError("q4 ONNX graph contains no MatMulNBits nodes")


def write_f32_tensor(
    tensors: dict[str, object],
    out_root: Path,
    name: str,
    array: np.ndarray,
    shard_size: int,
) -> None:
    data = array.astype("<f4", copy=False)
    tensor_root = out_root / "tensors" / safe_tensor_path(name)
    tensors[name] = {
        "kind": "f32",
        "shape": [int(dim) for dim in data.shape],
        "data": write_chunked(out_root, tensor_root, "data.f32", data.tobytes(), shard_size),
    }


def packed_uint4_initializer(initializer: onnx.TensorProto, element_count: int) -> bytes:
    if initializer.data_type != TensorProto.UINT4:
        raise ValueError(f"{initializer.name} must be UINT4, got data_type {initializer.data_type}")
    if initializer.raw_data:
        data = bytes(initializer.raw_data)
        expected = (element_count + 1) // 2
        if len(data) != expected:
            raise ValueError(f"{initializer.name} raw length mismatch: expected {expected}, got {len(data)}")
        return data
    values = numpy_helper.to_array(initializer).astype(np.uint8, copy=False).reshape(-1)
    if values.size != element_count:
        raise ValueError(f"{initializer.name} element count mismatch: expected {element_count}, got {values.size}")
    packed = bytearray((element_count + 1) // 2)
    for index, value in enumerate(values):
        if value > 15:
            raise ValueError(f"{initializer.name} contains non-uint4 value {value}")
        if index % 2 == 0:
            packed[index // 2] = int(value)
        else:
            packed[index // 2] |= int(value) << 4
    return bytes(packed)


def single_node(model: onnx.ModelProto, op_type: str) -> onnx.NodeProto:
    nodes = [node for node in model.graph.node if node.op_type == op_type]
    if len(nodes) != 1:
        raise ValueError(f"expected exactly one {op_type} node, found {len(nodes)}")
    return nodes[0]


def node_attrs(node: onnx.NodeProto) -> dict[str, object]:
    return {attribute.name: helper.get_attribute_value(attribute) for attribute in node.attribute}


def required_initializer(initializers: dict[str, onnx.TensorProto], name: str) -> onnx.TensorProto:
    initializer = initializers.get(name)
    if initializer is None:
        raise KeyError(f"q4 ONNX graph is missing initializer {name}")
    return initializer


def required_initializer_array(initializers: dict[str, onnx.TensorProto], name: str) -> np.ndarray:
    return numpy_helper.to_array(required_initializer(initializers, name))


def validate_manifest_shapes(manifest: dict[str, object]) -> None:
    model = manifest["model"]
    tensors = manifest["tensors"]
    expected_q = model["num_attention_heads"] * model["head_dim"]
    expected_kv = model["num_key_value_heads"] * model["head_dim"]
    expected = {
        "model.embed_tokens.weight": [model["vocab_size"], model["hidden_size"]],
        "model.norm.weight": [model["hidden_size"]],
    }
    for layer in range(model["num_hidden_layers"]):
        prefix = f"model.layers.{layer}"
        expected.update({
            f"{prefix}.input_layernorm.weight": [model["hidden_size"]],
            f"{prefix}.post_attention_layernorm.weight": [model["hidden_size"]],
            f"{prefix}.self_attn.q_norm.weight": [model["head_dim"]],
            f"{prefix}.self_attn.k_norm.weight": [model["head_dim"]],
            f"{prefix}.self_attn.q_proj.weight": [expected_q, model["hidden_size"]],
            f"{prefix}.self_attn.k_proj.weight": [expected_kv, model["hidden_size"]],
            f"{prefix}.self_attn.v_proj.weight": [expected_kv, model["hidden_size"]],
            f"{prefix}.self_attn.o_proj.weight": [model["hidden_size"], expected_q],
            f"{prefix}.mlp.gate_proj.weight": [model["intermediate_size"], model["hidden_size"]],
            f"{prefix}.mlp.up_proj.weight": [model["intermediate_size"], model["hidden_size"]],
            f"{prefix}.mlp.down_proj.weight": [model["hidden_size"], model["intermediate_size"]],
        })
    for name, shape in expected.items():
        if name not in tensors:
            raise ValueError(f"manifest is missing {name}")
        actual = tensors[name]["shape"]
        if actual != shape:
            raise ValueError(f"{name} shape mismatch: expected {shape}, got {actual}")


def read_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: object) -> None:
    path.write_bytes(json.dumps(value, indent=2, ensure_ascii=False).encode("utf-8") + b"\n")


def canonical_json(value: object) -> bytes:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def write_chunked(out_root: Path, root: Path, basename: str, data: bytes, shard_size: int) -> dict[str, object]:
    root.mkdir(parents=True, exist_ok=True)
    chunks = []
    if not data:
        raise ValueError(f"{root / basename} has no data")
    for index, offset in enumerate(range(0, len(data), shard_size)):
        chunk = data[offset:offset + shard_size]
        digest = sha256(chunk)
        if len(data) <= shard_size:
            rel = root / f"{basename}.{digest[:16]}.bin"
        else:
            rel = root / f"{basename}.part{index:04d}.{digest[:16]}.bin"
        rel.write_bytes(chunk)
        chunks.append({
            "url": str(rel.relative_to(out_root)).replace("\\", "/"),
            "byte_offset": offset,
            "byte_length": len(chunk),
            "sha256": digest,
        })
    return {
        "byte_length": len(data),
        "chunks": chunks,
    }


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


def safe_tensor_path(name: str) -> str:
    return name.replace("/", "_").replace(".", "/")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


if __name__ == "__main__":
    main()
