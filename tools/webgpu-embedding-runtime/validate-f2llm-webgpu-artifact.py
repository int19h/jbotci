#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch


DIMENSIONS = 320


class ArtifactModel:
    def __init__(self, root: Path):
        self.root = root
        self.manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
        self.model = self.manifest["model"]
        self.tensors: dict[str, torch.Tensor] = {}
        for name, spec in self.manifest["tensors"].items():
            self.tensors[name] = self.load_tensor(spec)

    def load_tensor(self, spec: dict[str, object]) -> torch.Tensor:
        kind = spec["kind"]
        if kind == "f32":
            shape = tuple(spec["shape"])
            return torch.from_numpy(np.frombuffer(self.read_chunked(spec["data"]), dtype="<f4").copy().reshape(shape))
        if kind == "q4_onnx_gather":
            return self.dequantize_onnx_gather(spec)
        if kind == "q4_onnx_matmul":
            return self.dequantize_onnx_matmul(spec)
        raise ValueError(f"unsupported tensor kind: {kind}")

    def read_chunked(self, chunked: dict[str, object]) -> bytes:
        data = bytearray(int(chunked["byte_length"]))
        for chunk in chunked["chunks"]:
            raw = (self.root / chunk["url"]).read_bytes()
            start = int(chunk["byte_offset"])
            data[start:start + int(chunk["byte_length"])] = raw
        return bytes(data)

    def dequantize_onnx_gather(self, spec: dict[str, object]) -> torch.Tensor:
        rows, cols = map(int, spec["shape"])
        group_size = int(spec["group_size"])
        groups = int(spec["groups"])
        q = unpack_u4(self.read_chunked(spec["qweight"]), rows * cols).reshape(rows, groups, group_size)
        scales = np.frombuffer(self.read_chunked(spec["scales"]), dtype="<f4").reshape(rows, groups)
        zero_points = unpack_u4(self.read_chunked(spec["zero_points"]), rows * groups).reshape(rows, groups)
        values = (q.astype(np.float32) - zero_points.astype(np.float32)[:, :, None]) * scales[:, :, None]
        return torch.from_numpy(values.reshape(rows, cols).copy())

    def dequantize_onnx_matmul(self, spec: dict[str, object]) -> torch.Tensor:
        rows, cols = map(int, spec["shape"])
        group_size = int(spec["group_size"])
        groups = int(spec["groups"])
        q = unpack_u4(self.read_chunked(spec["qweight"]), rows * groups * group_size)
        q = q.reshape(rows, groups, group_size)[:, :, :group_size]
        values = q.reshape(rows, groups, group_size).astype(np.float32)
        scales = np.frombuffer(self.read_chunked(spec["scales"]), dtype="<f4").reshape(rows, groups)
        zero_points = np.frombuffer(self.read_chunked(spec["zero_points"]), dtype="<f4").reshape(rows, groups)
        dequantized = (values - zero_points[:, :, None]) * scales[:, :, None]
        return torch.from_numpy(dequantized.reshape(rows, groups * group_size)[:, :cols].copy())

    def embed_tokens(self, token_ids: list[int]) -> torch.Tensor:
        return self.tensors["model.embed_tokens.weight"][torch.tensor(token_ids, dtype=torch.long)]

    def linear(self, name: str, values: torch.Tensor) -> torch.Tensor:
        return values @ self.tensors[name].T

    def rms_norm(self, values: torch.Tensor, name: str) -> torch.Tensor:
        weight = self.tensors[name]
        eps = float(self.model["rms_norm_eps"])
        inv_rms = torch.rsqrt(values.square().mean(dim=-1, keepdim=True) + eps)
        return values * inv_rms * weight

    def forward(self, token_ids: list[int]) -> torch.Tensor:
        hidden = self.embed_tokens(token_ids)
        for layer in range(int(self.model["num_hidden_layers"])):
            hidden = self.forward_layer(layer, hidden)
        hidden = self.rms_norm(hidden, "model.norm.weight")
        embedding = hidden[-1].float()
        return embedding / embedding.norm(p=2)

    def forward_layer(self, layer: int, hidden: torch.Tensor) -> torch.Tensor:
        prefix = f"model.layers.{layer}"
        q_heads = int(self.model["num_attention_heads"])
        kv_heads = int(self.model["num_key_value_heads"])
        head_dim = int(self.model["head_dim"])
        attn_norm = self.rms_norm(hidden, f"{prefix}.input_layernorm.weight")
        q = self.linear(f"{prefix}.self_attn.q_proj.weight", attn_norm).reshape(-1, q_heads, head_dim)
        k = self.linear(f"{prefix}.self_attn.k_proj.weight", attn_norm).reshape(-1, kv_heads, head_dim)
        v = self.linear(f"{prefix}.self_attn.v_proj.weight", attn_norm).reshape(-1, kv_heads, head_dim)
        q = self.rms_norm(q.reshape(-1, head_dim), f"{prefix}.self_attn.q_norm.weight").reshape_as(q)
        k = self.rms_norm(k.reshape(-1, head_dim), f"{prefix}.self_attn.k_norm.weight").reshape_as(k)
        q = rope(q, float(self.model["rope_theta"]))
        k = rope(k, float(self.model["rope_theta"]))
        attended = causal_attention(q, k, v).reshape(hidden.shape[0], q_heads * head_dim)
        projected = self.linear(f"{prefix}.self_attn.o_proj.weight", attended)
        post_attention = hidden + projected
        mlp_norm = self.rms_norm(post_attention, f"{prefix}.post_attention_layernorm.weight")
        gate = self.linear(f"{prefix}.mlp.gate_proj.weight", mlp_norm)
        up = self.linear(f"{prefix}.mlp.up_proj.weight", mlp_norm)
        down = self.linear(f"{prefix}.mlp.down_proj.weight", torch.nn.functional.silu(gate) * up)
        return post_attention + down


def unpack_u4(data: bytes, element_count: int) -> np.ndarray:
    raw = np.frombuffer(data, dtype=np.uint8)
    values = np.empty(element_count, dtype=np.uint8)
    values[0::2] = raw[: math.ceil(element_count / 2)] & 0x0F
    if element_count > 1:
        values[1::2] = (raw[: element_count // 2] >> 4) & 0x0F
    return values


def rope(values: torch.Tensor, theta: float) -> torch.Tensor:
    seq, heads, head_dim = values.shape
    half = head_dim // 2
    result = values.clone()
    positions = torch.arange(seq, dtype=values.dtype).reshape(seq, 1, 1)
    dims = torch.arange(half, dtype=values.dtype).reshape(1, 1, half)
    angles = positions / torch.pow(torch.tensor(theta, dtype=values.dtype), (dims * 2.0) / head_dim)
    cos = torch.cos(angles)
    sin = torch.sin(angles)
    first = values[:, :, :half]
    second = values[:, :, half:]
    result[:, :, :half] = first * cos - second * sin
    result[:, :, half:] = second * cos + first * sin
    return result


def causal_attention(q: torch.Tensor, k: torch.Tensor, v: torch.Tensor) -> torch.Tensor:
    seq, q_heads, head_dim = q.shape
    kv_heads = k.shape[1]
    group_size = q_heads // kv_heads
    output = torch.empty_like(q)
    scale = 1.0 / math.sqrt(head_dim)
    for token in range(seq):
        for q_head in range(q_heads):
            kv_head = q_head // group_size
            scores = torch.empty(token + 1, dtype=q.dtype)
            for key_token in range(token + 1):
                scores[key_token] = torch.dot(q[token, q_head], k[key_token, kv_head]) * scale
            probs = torch.softmax(scores, dim=0)
            weighted = torch.zeros(head_dim, dtype=q.dtype)
            for key_token in range(token + 1):
                weighted += probs[key_token] * v[key_token, kv_head]
            output[token, q_head] = weighted
    return output


def onnx_embedding(session: ort.InferenceSession, token_ids: list[int]) -> np.ndarray:
    input_ids = np.array([token_ids], dtype=np.int64)
    attention_mask = np.ones_like(input_ids)
    feeds = {
        "input_ids": input_ids,
        "attention_mask": attention_mask,
    }
    if "position_ids" in {item.name for item in session.get_inputs()}:
        feeds["position_ids"] = np.arange(len(token_ids), dtype=np.int64).reshape(1, -1)
    output = session.run(None, feeds)[0]
    if output.ndim == 3:
        vector = output[0, len(token_ids) - 1, :]
    else:
        vector = output[0]
    vector = vector.astype(np.float32)
    return vector / np.linalg.norm(vector)


def compare(left: np.ndarray, right: np.ndarray) -> dict[str, float]:
    return {
        "cosine": float(np.dot(left, right) / (np.linalg.norm(left) * np.linalg.norm(right))),
        "max_abs_diff": float(np.max(np.abs(left - right))),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", default=".jbotci-build/f2llm-v2-80m-webgpu/v1", type=Path)
    parser.add_argument(
        "--q4-onnx",
        default="/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx",
        type=Path,
    )
    parser.add_argument("--goldens", default=".jbotci-build/f2llm-webgpu-goldens/goldens.json", type=Path)
    parser.add_argument("--threshold", default=0.999, type=float)
    args = parser.parse_args()

    artifact = ArtifactModel(args.artifact)
    session = ort.InferenceSession(str(args.q4_onnx), providers=["CPUExecutionProvider"])
    goldens = json.loads(args.goldens.read_text(encoding="utf-8"))
    results = []
    for case in goldens["cases"]:
        token_ids = [int(token) for token in case["token_ids"]]
        actual = artifact.forward(token_ids).detach().numpy().astype(np.float32)
        expected = onnx_embedding(session, token_ids)
        comparison = compare(actual, expected)
        result = {
            "name": case["name"],
            **comparison,
            "passed": comparison["cosine"] >= args.threshold,
        }
        results.append(result)
    summary = {
        "threshold": args.threshold,
        "passed": all(result["passed"] for result in results),
        "results": results,
    }
    print(json.dumps(summary, indent=2))
    if not summary["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
