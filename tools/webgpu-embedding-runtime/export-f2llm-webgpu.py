#!/usr/bin/env python3

import argparse
import hashlib
import json
import math
import shutil
from pathlib import Path

import numpy as np
import torch
from huggingface_hub import snapshot_download
from safetensors.torch import load_file
from transformers import AutoConfig


SCHEMA_VERSION = 1
ARTIFACT_VERSION = "0.1.0"
RUNTIME = "jbotci-webgpu-f2llm"
MODEL_KEY = "f2llm-v2-80m-q4-320"
DEFAULT_MODEL_ID = "codefuse-ai/F2LLM-v2-80M"
DEFAULT_MAX_SEQUENCE_LENGTH = 512
DEFAULT_GROUP_SIZE = 32
DEFAULT_SHARD_SIZE = 4 * 1024 * 1024


def main():
    args = parse_args()
    model_dir = Path(args.model_dir) if args.model_dir else Path(
        snapshot_download(args.model, revision=args.revision)
    )
    config = AutoConfig.from_pretrained(model_dir)
    output = Path(args.out)
    stage = Path(args.stage) if args.stage else Path(f"{output}.staging")
    if stage == output:
        raise ValueError("--stage must differ from --out")
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)

    state = load_state_dict(model_dir)
    manifest = build_manifest(config, args)
    manifest["tokenizer"] = write_tokenizer(model_dir, stage)

    tensors = {}
    write_q4_tensor(
        tensors,
        stage,
        "model.embed_tokens.weight",
        required_tensor(state, "model.embed_tokens.weight"),
        args.group_size,
        args.shard_size,
    )
    write_f32_tensor(
        tensors,
        stage,
        "model.norm.weight",
        required_tensor(state, "model.norm.weight"),
        args.shard_size,
    )
    for layer in range(config.num_hidden_layers):
        prefix = f"model.layers.{layer}"
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.input_layernorm.weight",
            required_tensor(state, f"{prefix}.input_layernorm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.post_attention_layernorm.weight",
            required_tensor(state, f"{prefix}.post_attention_layernorm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.self_attn.q_norm.weight",
            required_tensor(state, f"{prefix}.self_attn.q_norm.weight"),
            args.shard_size,
        )
        write_f32_tensor(
            tensors,
            stage,
            f"{prefix}.self_attn.k_norm.weight",
            required_tensor(state, f"{prefix}.self_attn.k_norm.weight"),
            args.shard_size,
        )
        for suffix in [
            "self_attn.q_proj.weight",
            "self_attn.k_proj.weight",
            "self_attn.v_proj.weight",
            "self_attn.o_proj.weight",
            "mlp.gate_proj.weight",
            "mlp.up_proj.weight",
            "mlp.down_proj.weight",
        ]:
            name = f"{prefix}.{suffix}"
            write_q4_tensor(
                tensors,
                stage,
                name,
                required_tensor(state, name),
                args.group_size,
                args.shard_size,
            )
    manifest["tensors"] = tensors
    validate_manifest_shapes(manifest)
    write_json(stage / "manifest.json", manifest)
    promote(stage, output)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Export codefuse-ai/F2LLM-v2-80M as a jbotci custom WebGPU q4 artifact."
    )
    parser.add_argument("--model", default=DEFAULT_MODEL_ID)
    parser.add_argument("--revision", default=None)
    parser.add_argument("--model-dir", default=None)
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", default=None)
    parser.add_argument("--group-size", type=int, default=DEFAULT_GROUP_SIZE)
    parser.add_argument("--shard-size", type=int, default=DEFAULT_SHARD_SIZE)
    parser.add_argument("--max-sequence-length", type=int, default=DEFAULT_MAX_SEQUENCE_LENGTH)
    args = parser.parse_args()
    if args.group_size <= 0:
        raise ValueError("--group-size must be positive")
    if args.shard_size <= 0:
        raise ValueError("--shard-size must be positive")
    if args.max_sequence_length <= 1:
        raise ValueError("--max-sequence-length must be greater than 1")
    return args


def load_state_dict(model_dir):
    files = sorted(model_dir.glob("*.safetensors"))
    if not files:
        raise FileNotFoundError(f"no safetensors files found in {model_dir}")
    state = {}
    for path in files:
        state.update(load_file(path, device="cpu"))
    return state


def build_manifest(config, args):
    rope_theta = config_rope_theta(config)
    return {
        "schema_version": SCHEMA_VERSION,
        "runtime": RUNTIME,
        "artifact_version": ARTIFACT_VERSION,
        "model_key": MODEL_KEY,
        "source_model": args.model,
        "source_revision": args.revision,
        "max_sequence_length": args.max_sequence_length,
        "quantization": {
            "kind": "q4_rowwise_symmetric",
            "group_size": args.group_size,
            "zero_point": 8,
            "scale_dtype": "f32le",
            "pack_order": "low_nibble_first",
        },
        "model": {
            "vocab_size": int(config.vocab_size),
            "hidden_size": int(config.hidden_size),
            "num_hidden_layers": int(config.num_hidden_layers),
            "num_attention_heads": int(config.num_attention_heads),
            "num_key_value_heads": int(config.num_key_value_heads),
            "head_dim": int(config.head_dim),
            "intermediate_size": int(config.intermediate_size),
            "rms_norm_eps": float(config.rms_norm_eps),
            "rope_theta": rope_theta,
        },
    }


def config_rope_theta(config):
    direct = getattr(config, "rope_theta", None)
    if direct is not None:
        return float(direct)
    values = config.to_dict()
    if values.get("rope_theta") is not None:
        return float(values["rope_theta"])
    rope_scaling = getattr(config, "rope_scaling", None)
    if isinstance(rope_scaling, dict) and rope_scaling.get("rope_theta") is not None:
        return float(rope_scaling["rope_theta"])
    rope_parameters = getattr(config, "rope_parameters", None)
    if isinstance(rope_parameters, dict) and rope_parameters.get("rope_theta") is not None:
        return float(rope_parameters["rope_theta"])
    raise ValueError("model config is missing rope_theta")


def write_tokenizer(model_dir, out_root):
    tokenizer_path = model_dir / "tokenizer.json"
    with tokenizer_path.open("r", encoding="utf-8") as file:
        tokenizer = json.load(file)
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
    path = out_root / "tokenizer.compact.json"
    path.write_bytes(data)
    return {
        "url": "tokenizer.compact.json",
        "byte_length": len(data),
        "canonical_json_sha256": sha256(data),
    }


def required_tensor(state, name):
    candidates = [name]
    if name.startswith("model."):
        candidates.append(name.removeprefix("model."))
    else:
        candidates.append(f"model.{name}")
    for candidate in candidates:
        if candidate in state:
            return state[candidate]
    raise KeyError(f"model is missing required tensor {name}")


def write_q4_tensor(tensors, out_root, name, tensor, group_size, shard_size):
    array = tensor.detach().to(torch.float32).cpu().contiguous().numpy()
    if array.ndim != 2:
        raise ValueError(f"{name} must be a rank-2 matrix")
    qweight, scales, groups = quantize_q4_rowwise(array, group_size)
    tensor_root = out_root / "tensors" / safe_tensor_path(name)
    tensors[name] = {
        "kind": "q4_rowwise",
        "shape": [int(array.shape[0]), int(array.shape[1])],
        "group_size": group_size,
        "groups": groups,
        "qweight": write_chunked(out_root, tensor_root, "qweight", qweight.tobytes(), shard_size),
        "scales": write_chunked(
            out_root,
            tensor_root,
            "scales.f32",
            scales.astype("<f4", copy=False).tobytes(),
            shard_size,
        ),
    }


def write_f32_tensor(tensors, out_root, name, tensor, shard_size):
    array = tensor.detach().to(torch.float32).cpu().contiguous().numpy().astype("<f4", copy=False)
    tensor_root = out_root / "tensors" / safe_tensor_path(name)
    tensors[name] = {
        "kind": "f32",
        "shape": [int(dim) for dim in array.shape],
        "data": write_chunked(out_root, tensor_root, "data.f32", array.tobytes(), shard_size),
    }


def quantize_q4_rowwise(array, group_size):
    rows, cols = array.shape
    groups = math.ceil(cols / group_size)
    qweight = np.zeros((rows * cols + 1) // 2, dtype=np.uint8)
    scales = np.zeros((rows, groups), dtype=np.float32)
    for row in range(rows):
        for group in range(groups):
            start = group * group_size
            end = min(cols, start + group_size)
            values = array[row, start:end]
            max_abs = float(np.max(np.abs(values))) if values.size else 0.0
            scale = max_abs / 7.0 if max_abs > 0.0 else 1.0
            scales[row, group] = scale
            quantized = np.clip(np.rint(values / scale) + 8, 0, 15).astype(np.uint8)
            for index, value in enumerate(quantized):
                element = row * cols + start + index
                byte_index = element // 2
                if element % 2 == 0:
                    qweight[byte_index] = (qweight[byte_index] & 0xF0) | int(value)
                else:
                    qweight[byte_index] = (qweight[byte_index] & 0x0F) | (int(value) << 4)
    return qweight, scales.reshape(-1), groups


def write_chunked(out_root, root, basename, data, shard_size):
    root.mkdir(parents=True, exist_ok=True)
    chunks = []
    if not data:
        raise ValueError(f"{root / basename} has no data")
    for index, offset in enumerate(range(0, len(data), shard_size)):
        chunk = data[offset:offset + shard_size]
        if len(data) <= shard_size:
            rel = root / f"{basename}.bin"
        else:
            rel = root / f"{basename}.part{index:04d}.bin"
        rel.write_bytes(chunk)
        chunks.append({
            "url": str(rel.relative_to(out_root)).replace("\\", "/"),
            "byte_offset": offset,
            "byte_length": len(chunk),
            "sha256": sha256(chunk),
        })
    return {
        "byte_length": len(data),
        "chunks": chunks,
    }


def validate_manifest_shapes(manifest):
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
        actual = tensors[name]["shape"]
        if actual != shape:
            raise ValueError(f"{name} shape mismatch: expected {shape}, got {actual}")


def write_json(path, value):
    path.write_bytes(json.dumps(value, indent=2, ensure_ascii=False).encode("utf-8") + b"\n")


def canonical_json(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


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


def safe_tensor_path(name):
    return name.replace(".", "/")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


if __name__ == "__main__":
    main()
