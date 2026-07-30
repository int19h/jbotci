#!/usr/bin/env python3
"""Instrument a scratch copy of the WASM runtime for embedding-only WebGPU evidence."""

from __future__ import annotations

import argparse
from pathlib import Path


REQUIRED_FEATURES = """        let required_features = if adapter.features().contains(wgpu::Features::SHADER_F16) {
            wgpu::Features::SHADER_F16
        } else {
            return Err("F2LLM WebGPU vector scoring requires the shader-f16 feature".to_owned());
        };
"""

EMBEDDING_ONLY_FEATURES = """        let required_features = wgpu::Features::empty();
"""

PRECOMPILE_LOOP = """        for (name, shader) in webgpu_pipeline_sources() {
            let scopes = self.push_gpu_error_scopes();
"""

EMBEDDING_ONLY_PRECOMPILE_LOOP = """        for (name, shader) in webgpu_pipeline_sources() {
            if name == "vectorDotF16" {
                continue;
            }
            let scopes = self.push_gpu_error_scopes();
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime-source", type=Path, required=True)
    return parser.parse_args()


def replace_exactly_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise ValueError(f"expected exactly one {label} block, found {count}")
    return text.replace(old, new, 1)


def main() -> None:
    args = parse_args()
    source = args.runtime_source.resolve()
    scratch_root = Path("/build/jbotci/scratch").resolve()
    if not source.is_relative_to(scratch_root):
        raise ValueError(f"refusing to instrument non-scratch path: {source}")

    original = source.read_text(encoding="utf-8")
    instrumented = replace_exactly_once(
        original,
        REQUIRED_FEATURES,
        EMBEDDING_ONLY_FEATURES,
        "SHADER_F16 request",
    )
    instrumented = replace_exactly_once(
        instrumented,
        PRECOMPILE_LOOP,
        EMBEDDING_ONLY_PRECOMPILE_LOOP,
        "pipeline precompile loop",
    )
    source.write_text(instrumented, encoding="utf-8")


if __name__ == "__main__":
    main()
