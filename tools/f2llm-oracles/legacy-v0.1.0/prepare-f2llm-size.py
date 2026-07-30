#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


PYTHON = Path("/home/int19h.linux/git/jbotci-f2llm-quant/.venv/bin/python")
OPTIMUM = Path("/home/int19h.linux/git/jbotci-f2llm-quant/.venv/bin/optimum-cli")
QUANT_ROOT = Path("/home/int19h.linux/git/jbotci-f2llm-quant")
QUANT_SCRIPTS = QUANT_ROOT / "scripts"
ARTIFACTS_ROOT = QUANT_ROOT / "artifacts"
THIS_DIR = Path(__file__).resolve().parent
PROTOTYPE_ROOT = THIS_DIR.parent
LOCAL_ASSETS = PROTOTYPE_ROOT / "local-assets"


@dataclass(frozen=True)
class ModelSpec:
    id: str
    hf_model: str
    model_key: str
    artifact_dir_name: str
    dimensions: int

    @property
    def safe_name(self) -> str:
        return self.model_key.replace(".", "_")


SPECS = {
    "160m": ModelSpec(
        id="160m",
        hf_model="codefuse-ai/F2LLM-v2-160M",
        model_key="f2llm-v2-160m-q4-640",
        artifact_dir_name="f2llm-v2-160m-webgpu",
        dimensions=640,
    ),
    "330m": ModelSpec(
        id="330m",
        hf_model="codefuse-ai/F2LLM-v2-330M",
        model_key="f2llm-v2-330m-q4-896",
        artifact_dir_name="f2llm-v2-330m-webgpu",
        dimensions=896,
    ),
    "0.6b": ModelSpec(
        id="0.6b",
        hf_model="codefuse-ai/F2LLM-v2-0.6B",
        model_key="f2llm-v2-0.6b-q4-1024",
        artifact_dir_name="f2llm-v2-0.6b-webgpu",
        dimensions=1024,
    ),
}


def main() -> None:
    args = parse_args()
    if not PYTHON.exists():
        raise SystemExit(f"missing quantization venv Python: {PYTHON}")
    if not OPTIMUM.exists():
        raise SystemExit(f"missing optimum-cli: {OPTIMUM}")
    summaries = []
    for size in args.sizes:
        spec = SPECS[size]
        summaries.append(prepare_size(spec, args))
    summary_path = LOCAL_ASSETS / "prepare-summary.json"
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summaries, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {summary_path}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Prepare local multi-size F2LLM q4 WebGPU prototype assets.")
    parser.add_argument(
        "--sizes",
        nargs="+",
        choices=sorted(SPECS),
        default=["160m", "330m", "0.6b"],
    )
    parser.add_argument("--force-export", action="store_true")
    parser.add_argument("--force-quantize", action="store_true")
    parser.add_argument("--force-webgpu", action="store_true")
    parser.add_argument("--force-goldens", action="store_true")
    parser.add_argument("--skip-validate", action="store_true")
    parser.add_argument("--opset", type=int, default=18)
    parser.add_argument("--block-size", type=int, default=32)
    parser.add_argument("--shard-size", type=int, default=4 * 1024 * 1024)
    parser.add_argument("--threshold", type=float, default=0.999)
    return parser.parse_args()


def prepare_size(spec: ModelSpec, args: argparse.Namespace) -> dict[str, object]:
    print(f"\n=== {spec.id}: {spec.hf_model} ===", flush=True)
    fp32_dir = ARTIFACTS_ROOT / f"{spec.safe_name}-fp32-export-transformers"
    fp32_model = fp32_dir / "model.onnx"
    matmul_q4 = ARTIFACTS_ROOT / f"{spec.safe_name}-q4-matmul-hqq32" / "onnx" / "model_matmul_q4.onnx"
    packaged_dir = ARTIFACTS_ROOT / f"{spec.safe_name}-q4-hqq32-transformersjs"
    q4_model = packaged_dir / "onnx" / "model_q4.onnx"
    local_q4 = LOCAL_ASSETS / "q4-onnx" / spec.model_key / "model_q4.onnx"
    webgpu_artifact = LOCAL_ASSETS / "models" / spec.artifact_dir_name / "v1"
    goldens = LOCAL_ASSETS / "goldens" / spec.model_key / "goldens.json"

    if args.force_export or not fp32_model.exists():
        shutil.rmtree(fp32_dir, ignore_errors=True)
        run([
            str(OPTIMUM),
            "export",
            "onnx",
            "--model",
            spec.hf_model,
            "--library-name",
            "transformers",
            "--task",
            "feature-extraction",
            "--opset",
            str(args.opset),
            "--monolith",
            str(fp32_dir),
        ])
    else:
        print(f"reuse fp32 ONNX: {fp32_model}")

    if args.force_quantize or not matmul_q4.exists():
        shutil.rmtree(matmul_q4.parent.parent, ignore_errors=True)
        run([
            str(PYTHON),
            str(QUANT_SCRIPTS / "quantize_q4.py"),
            "--input",
            str(fp32_model),
            "--output",
            str(matmul_q4),
            "--block-size",
            str(args.block_size),
            "--algorithm",
            "hqq",
            "--op-types",
            "MatMul",
        ])
    else:
        print(f"reuse q4 MatMul ONNX: {matmul_q4}")

    if args.force_quantize or not q4_model.exists():
        shutil.rmtree(packaged_dir, ignore_errors=True)
        q4_tmp = packaged_dir.with_name(f"{packaged_dir.name}.tmp") / "onnx" / "model_q4.onnx"
        shutil.rmtree(q4_tmp.parent.parent, ignore_errors=True)
        run([
            str(PYTHON),
            str(QUANT_SCRIPTS / "quantize_q4.py"),
            "--input",
            str(matmul_q4),
            "--output",
            str(q4_tmp),
            "--block-size",
            str(args.block_size),
            "--algorithm",
            "default",
            "--op-types",
            "Gather",
        ])
        run([
            str(PYTHON),
            str(QUANT_SCRIPTS / "prepare_transformers_js_repo.py"),
            "--fp32-export",
            str(fp32_dir),
            "--q4-model",
            str(q4_tmp),
            "--output",
            str(packaged_dir),
        ])
        shutil.rmtree(q4_tmp.parent.parent, ignore_errors=True)
    else:
        print(f"reuse packaged q4 ONNX: {q4_model}")

    ensure_symlink(q4_model, local_q4)

    if args.force_webgpu or not (webgpu_artifact / "manifest.json").exists():
        run([
            str(PYTHON),
            str(THIS_DIR / "export-f2llm-webgpu-from-onnx-q4.py"),
            "--onnx-model",
            str(q4_model),
            "--model-root",
            str(packaged_dir),
            "--model-key",
            spec.model_key,
            "--source-model",
            spec.hf_model,
            "--out",
            str(webgpu_artifact),
            "--shard-size",
            str(args.shard_size),
        ])
    else:
        print(f"reuse WebGPU artifact: {webgpu_artifact}")

    if args.force_goldens or not goldens.exists():
        run([
            str(PYTHON),
            str(THIS_DIR / "generate-f2llm-goldens.py"),
            "--model",
            spec.hf_model,
            "--model-key",
            spec.model_key,
            "--dimensions",
            str(spec.dimensions),
            "--q4-onnx",
            str(q4_model),
            "--tokenizer-dir",
            str(packaged_dir),
            "--out",
            str(goldens),
            "--cosine-threshold",
            str(args.threshold),
            "--q4-onnx-threshold",
            str(args.threshold),
        ])
    else:
        print(f"reuse goldens: {goldens}")

    if not args.skip_validate:
        run([
            str(PYTHON),
            str(THIS_DIR / "validate-f2llm-webgpu-artifact.py"),
            "--artifact",
            str(webgpu_artifact),
            "--q4-onnx",
            str(q4_model),
            "--goldens",
            str(goldens),
            "--threshold",
            str(args.threshold),
        ])

    return {
        "id": spec.id,
        "hf_model": spec.hf_model,
        "model_key": spec.model_key,
        "dimensions": spec.dimensions,
        "fp32_onnx": str(fp32_model),
        "q4_onnx": str(q4_model),
        "local_q4_onnx": str(local_q4),
        "webgpu_artifact": str(webgpu_artifact),
        "goldens": str(goldens),
        "webgpu_manifest_bytes": (webgpu_artifact / "manifest.json").stat().st_size,
        "q4_onnx_bytes": q4_model.stat().st_size,
    }


def ensure_symlink(source: Path, link: Path) -> None:
    link.parent.mkdir(parents=True, exist_ok=True)
    if link.is_symlink() or link.exists():
        if link.resolve() == source.resolve():
            return
        link.unlink()
    relative_source = os.path.relpath(source, link.parent)
    link.symlink_to(relative_source)
    print(f"linked {link} -> {relative_source}")


def run(command: list[str]) -> None:
    print("+ " + " ".join(command), flush=True)
    subprocess.run(command, check=True)


if __name__ == "__main__":
    main()
