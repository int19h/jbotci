#!/usr/bin/env python3
"""Download and verify an immutable published F2LLM WebGPU artifact.

The optional runtime manifest lets the native suite execute a vendored legacy manifest
against current immutable object names when every referenced payload digest is identical.
It never treats a filename match as evidence: payloads are joined by SHA-256 and byte length.
"""

from __future__ import annotations

import argparse
import concurrent.futures
from dataclasses import dataclass
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import shutil
import tempfile
import urllib.parse
import urllib.request


ARTIFACT_IDENTITY_DOMAIN = b"jbotci:f2llm-webgpu:ArtifactIdentityV1\0"
SHAPE_MODEL_FIELDS_DOMAIN = ARTIFACT_IDENTITY_DOMAIN + b"shape-model-fields\0"
TENSOR_ITEMS_DOMAIN = ARTIFACT_IDENTITY_DOMAIN + b"tensor-items\0"
IDENTITY_PINS_PATH = Path(__file__).with_name("webgpu-artifact-identity-pins.json")
MODEL_INTEGER_FIELDS = (
    "vocab_size",
    "hidden_size",
    "num_hidden_layers",
    "num_attention_heads",
    "num_key_value_heads",
    "head_dim",
    "intermediate_size",
)
MODEL_FLOAT_FIELDS = (
    "rms_norm_eps",
    "rope_theta",
)
MODEL_FIELDS = MODEL_INTEGER_FIELDS + MODEL_FLOAT_FIELDS
TENSOR_STORAGE_COMPONENTS = ("qweight", "scales", "zero_points", "data")
TENSOR_METADATA_FIELDS = {"kind", "shape", "group_size", "groups"}


@dataclass(frozen=True)
class ArtifactIdentity:
    digest: str
    shape_model_fields_sha256: str
    tokenizer_canonical_json_sha256: str
    tensor_items_sha256: str
    model_key: str
    dimensions: dict[str, object]
    max_sequence_length: int


def sha256_hex(value: str) -> str:
    if len(value) != 64 or any(character not in "0123456789abcdefABCDEF" for character in value):
        raise argparse.ArgumentTypeError("must be exactly 64 hexadecimal characters")
    return value.lower()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    manifest = parser.add_mutually_exclusive_group()
    manifest.add_argument("--manifest", type=Path)
    manifest.add_argument("--manifest-url")
    action = parser.add_mutually_exclusive_group()
    action.add_argument(
        "--print-artifact-identity",
        action="store_true",
        help="print the manifest's identity and expected-component pin, then exit",
    )
    action.add_argument(
        "--selftest",
        action="store_true",
        help="run offline ArtifactIdentityV1 checks, then exit",
    )
    parser.add_argument("--expected-artifact-identity", type=sha256_hex)
    parser.add_argument("--runtime-manifest", type=Path)
    parser.add_argument("--base-url")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    if args.selftest:
        return args
    if args.manifest is None and args.manifest_url is None:
        parser.error("one of --manifest or --manifest-url is required")
    if args.print_artifact_identity:
        if args.runtime_manifest is not None:
            parser.error("--print-artifact-identity does not accept --runtime-manifest")
        return args
    if args.manifest_url is not None and args.expected_artifact_identity is None:
        parser.error("--manifest-url requires --expected-artifact-identity")
    if args.base_url is None:
        parser.error("--base-url is required when downloading artifacts")
    if args.out is None:
        parser.error("--out is required when downloading artifacts")
    return args


def canonical_json(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, allow_nan=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def domain_hash(domain: bytes, value: object) -> str:
    return hashlib.sha256(domain + canonical_json(value)).hexdigest()


def require_dict(value: object, field: str) -> dict[str, object]:
    if not isinstance(value, dict) or not all(isinstance(key, str) for key in value):
        raise ValueError(f"manifest field {field!r} must be an object with string keys")
    return value


def require_positive_int(value: object, field: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise ValueError(f"manifest field {field!r} must be a positive integer")
    return value


def require_sha256(value: object, field: str) -> str:
    if not isinstance(value, str):
        raise ValueError(f"manifest field {field!r} must be a SHA-256 string")
    try:
        return sha256_hex(value)
    except argparse.ArgumentTypeError as error:
        raise ValueError(f"manifest field {field!r} {error}") from error


def model_dimensions(model: dict[str, object], field: str = "model") -> dict[str, object]:
    dimensions: dict[str, object] = {}
    for name in MODEL_INTEGER_FIELDS:
        dimensions[name] = require_positive_int(model.get(name), f"{field}.{name}")
    for name in MODEL_FLOAT_FIELDS:
        value = model.get(name)
        if (
            not isinstance(value, (int, float))
            or isinstance(value, bool)
            or not math.isfinite(value)
            or value <= 0
        ):
            raise ValueError(f"manifest field '{field}.{name}' must be positive and finite")
        dimensions[name] = float(value)
    return dimensions


def tensor_identity_items(manifest: dict[str, object]) -> list[dict[str, object]]:
    tensors = require_dict(manifest.get("tensors"), "tensors")
    items: list[tuple[str, int]] = []
    for tensor_name, tensor_value in tensors.items():
        tensor = require_dict(tensor_value, f"tensors.{tensor_name}")
        unknown_fields = set(tensor) - TENSOR_METADATA_FIELDS - set(TENSOR_STORAGE_COMPONENTS)
        if unknown_fields:
            raise ValueError(
                f"tensor {tensor_name!r} has unsupported fields {sorted(unknown_fields)!r}; "
                "ArtifactIdentityV1 refuses to omit unknown tensor content"
            )
        for component in TENSOR_STORAGE_COMPONENTS:
            component_value = tensor.get(component)
            if component_value is None:
                continue
            chunked = require_dict(component_value, f"tensors.{tensor_name}.{component}")
            chunks = chunked.get("chunks")
            if not isinstance(chunks, list):
                raise ValueError(
                    f"manifest field 'tensors.{tensor_name}.{component}.chunks' must be a list"
                )
            for index, chunk_value in enumerate(chunks):
                chunk = require_dict(
                    chunk_value, f"tensors.{tensor_name}.{component}.chunks[{index}]"
                )
                digest = require_sha256(
                    chunk.get("sha256"),
                    f"tensors.{tensor_name}.{component}.chunks[{index}].sha256",
                )
                byte_length = require_positive_int(
                    chunk.get("byte_length"),
                    f"tensors.{tensor_name}.{component}.chunks[{index}].byte_length",
                )
                items.append((digest, byte_length))
    if not items:
        raise ValueError("manifest must contain at least one tensor chunk")
    return [
        {"sha256": digest, "byte_length": byte_length}
        for digest, byte_length in sorted(items)
    ]


def artifact_identity_from_components(
    *,
    shape_model_fields_sha256: str,
    tokenizer_canonical_json_sha256: str,
    tensor_items_sha256: str,
) -> str:
    return domain_hash(
        ARTIFACT_IDENTITY_DOMAIN,
        {
            "shape_model_fields_sha256": shape_model_fields_sha256,
            "tokenizer_canonical_json_sha256": tokenizer_canonical_json_sha256,
            "tensor_items_sha256": tensor_items_sha256,
        },
    )


def shape_model_fields_sha256(
    *, model_key: str, dimensions: dict[str, object], max_sequence_length: int
) -> str:
    return domain_hash(
        SHAPE_MODEL_FIELDS_DOMAIN,
        {
            "model_key": model_key,
            "dimensions": dimensions,
            "max_sequence_length": max_sequence_length,
        },
    )


def artifact_identity(manifest: dict[str, object]) -> ArtifactIdentity:
    """Derive ArtifactIdentityV1 without ever serializing the whole manifest.

    The top-level domain-separated digest covers three named component digests.
    Shape/model fields and the sorted tensor item list have their own domains;
    this Merkle-style composition both covers every requested semantic value and
    permits truthful mismatch diagnostics from the checked-in component pin.
    """
    model_key = manifest.get("model_key")
    if not isinstance(model_key, str) or not model_key:
        raise ValueError("manifest field 'model_key' must be a non-empty string")
    model = require_dict(manifest.get("model"), "model")
    # This explicit typed selection is the identity boundary: provenance, paths,
    # URLs, and future non-model metadata cannot enter ArtifactIdentityV1.
    dimensions = model_dimensions(model)
    max_sequence_length = require_positive_int(
        manifest.get("max_sequence_length"), "max_sequence_length"
    )
    tokenizer = require_dict(manifest.get("tokenizer"), "tokenizer")
    tokenizer_digest = require_sha256(
        tokenizer.get("canonical_json_sha256"), "tokenizer.canonical_json_sha256"
    )
    shape_model_digest = shape_model_fields_sha256(
        model_key=model_key,
        dimensions=dimensions,
        max_sequence_length=max_sequence_length,
    )
    tensor_digest = domain_hash(TENSOR_ITEMS_DOMAIN, tensor_identity_items(manifest))
    digest = artifact_identity_from_components(
        shape_model_fields_sha256=shape_model_digest,
        tokenizer_canonical_json_sha256=tokenizer_digest,
        tensor_items_sha256=tensor_digest,
    )
    return ArtifactIdentity(
        digest=digest,
        shape_model_fields_sha256=shape_model_digest,
        tokenizer_canonical_json_sha256=tokenizer_digest,
        tensor_items_sha256=tensor_digest,
        model_key=model_key,
        dimensions=dimensions,
        max_sequence_length=max_sequence_length,
    )


def identity_pin(identity: ArtifactIdentity) -> dict[str, object]:
    return {
        "model_key": identity.model_key,
        "dimensions": identity.dimensions,
        "max_sequence_length": identity.max_sequence_length,
        "tokenizer_canonical_json_sha256": identity.tokenizer_canonical_json_sha256,
        "tensor_items_sha256": identity.tensor_items_sha256,
    }


def expected_identity(expected_digest: str) -> ArtifactIdentity:
    try:
        document = json.loads(IDENTITY_PINS_PATH.read_bytes())
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(
            f"cannot read artifact identity pins from {IDENTITY_PINS_PATH}: {error}"
        ) from error
    root = require_dict(document, "identity pins")
    if root.get("schema") != "jbotci-f2llm-webgpu-artifact-identity-pins-v1":
        raise ValueError(f"unsupported artifact identity pin schema in {IDENTITY_PINS_PATH}")
    identities = require_dict(root.get("identities"), "identities")
    pin_value = identities.get(expected_digest)
    if pin_value is None:
        raise ValueError(
            f"expected artifact identity {expected_digest} has no component pin in "
            f"{IDENTITY_PINS_PATH}; regenerate it with --print-artifact-identity"
        )
    pin = require_dict(pin_value, f"identities.{expected_digest}")
    model_key = pin.get("model_key")
    if not isinstance(model_key, str) or not model_key:
        raise ValueError(
            f"identity pin field 'identities.{expected_digest}.model_key' "
            "must be a non-empty string"
        )
    dimensions = require_dict(
        pin.get("dimensions"), f"identities.{expected_digest}.dimensions"
    )
    if set(dimensions) != set(MODEL_FIELDS):
        raise ValueError(
            f"identity pin field 'identities.{expected_digest}.dimensions' "
            f"must contain exactly {MODEL_FIELDS!r}"
        )
    dimensions = model_dimensions(
        dimensions, f"identities.{expected_digest}.dimensions"
    )
    max_sequence_length = require_positive_int(
        pin.get("max_sequence_length"),
        f"identities.{expected_digest}.max_sequence_length",
    )
    tokenizer_digest = require_sha256(
        pin.get("tokenizer_canonical_json_sha256"),
        f"identities.{expected_digest}.tokenizer_canonical_json_sha256",
    )
    shape_model_digest = shape_model_fields_sha256(
        model_key=model_key,
        dimensions=dimensions,
        max_sequence_length=max_sequence_length,
    )
    tensor_digest = require_sha256(
        pin.get("tensor_items_sha256"),
        f"identities.{expected_digest}.tensor_items_sha256",
    )
    digest = artifact_identity_from_components(
        shape_model_fields_sha256=shape_model_digest,
        tokenizer_canonical_json_sha256=tokenizer_digest,
        tensor_items_sha256=tensor_digest,
    )
    if digest != expected_digest:
        raise ValueError(
            f"component pin for {expected_digest} is inconsistent (recomputes as {digest})"
        )
    return ArtifactIdentity(
        digest=digest,
        shape_model_fields_sha256=shape_model_digest,
        tokenizer_canonical_json_sha256=tokenizer_digest,
        tensor_items_sha256=tensor_digest,
        model_key=model_key,
        dimensions=dimensions,
        max_sequence_length=max_sequence_length,
    )


def identity_differences(
    actual: ArtifactIdentity, expected: ArtifactIdentity
) -> list[str]:
    differences: list[str] = []
    if actual.tensor_items_sha256 != expected.tensor_items_sha256:
        differences.append(
            "tensors "
            f"(actual {actual.tensor_items_sha256}, expected {expected.tensor_items_sha256})"
        )
    if (
        actual.tokenizer_canonical_json_sha256
        != expected.tokenizer_canonical_json_sha256
    ):
        differences.append(
            "tokenizer "
            f"(actual {actual.tokenizer_canonical_json_sha256}, "
            f"expected {expected.tokenizer_canonical_json_sha256})"
        )
    if actual.shape_model_fields_sha256 != expected.shape_model_fields_sha256:
        differences.append(
            "shape/model fields "
            f"(actual model_key={actual.model_key!r}, dimensions={actual.dimensions!r}, "
            f"max_sequence_length={actual.max_sequence_length!r}; "
            f"expected model_key={expected.model_key!r}, dimensions={expected.dimensions!r}, "
            f"max_sequence_length={expected.max_sequence_length!r})"
        )
    return differences


def verify_artifact_identity(
    manifest: dict[str, object], expected_digest: str
) -> ArtifactIdentity:
    actual = artifact_identity(manifest)
    expected = expected_identity(expected_digest)
    if actual.digest == expected.digest:
        return actual
    differences = identity_differences(actual, expected)
    if not differences:
        raise RuntimeError("artifact identity mismatch without a component mismatch")
    raise ValueError(
        f"published artifact identity is {actual.digest}, expected {expected.digest}; "
        f"identity components diverged: {'; '.join(differences)}"
    )


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def normalized_relative_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if (
        not value
        or path.is_absolute()
        or any(part in ("", ".", "..") for part in path.parts)
        or "\\" in value
    ):
        raise ValueError(f"artifact path is not normalized and relative: {value!r}")
    return path


def manifest_payloads(manifest: dict[str, object]) -> list[tuple[str, int, str]]:
    tokenizer = manifest["tokenizer"]
    assert isinstance(tokenizer, dict)
    rows = [
        (
            str(tokenizer["url"]),
            int(tokenizer["byte_length"]),
            str(tokenizer["canonical_json_sha256"]),
        )
    ]
    tensors = manifest["tensors"]
    assert isinstance(tensors, dict)
    for tensor in tensors.values():
        assert isinstance(tensor, dict)
        for component in ("qweight", "scales", "zero_points", "data"):
            chunked = tensor.get(component)
            if chunked is None:
                continue
            assert isinstance(chunked, dict)
            chunks = chunked["chunks"]
            assert isinstance(chunks, list)
            for chunk in chunks:
                assert isinstance(chunk, dict)
                rows.append(
                    (
                        str(chunk["url"]),
                        int(chunk["byte_length"]),
                        str(chunk["sha256"]),
                    )
                )
    return rows


def fetch_one(
    *,
    base_url: str,
    source_url: str,
    destination: Path,
    byte_length: int,
    sha256: str,
) -> str:
    if destination.is_file():
        if destination.stat().st_size == byte_length and sha256_file(destination) == sha256:
            return f"verified {destination}"
        raise ValueError(f"existing artifact payload is corrupt: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(
        urllib.parse.urljoin(base_url.rstrip("/") + "/", source_url),
        headers={"User-Agent": "jbotci-f2llm-native-oracle/1"},
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        with tempfile.NamedTemporaryFile(
            dir=destination.parent, prefix=f".{destination.name}.", delete=False
        ) as temporary:
            temporary_path = Path(temporary.name)
            shutil.copyfileobj(response, temporary)
    try:
        if temporary_path.stat().st_size != byte_length:
            raise ValueError(
                f"{source_url} has {temporary_path.stat().st_size} bytes, expected {byte_length}"
            )
        actual = sha256_file(temporary_path)
        if actual != sha256:
            raise ValueError(f"{source_url} SHA-256 is {actual}, expected {sha256}")
        os.replace(temporary_path, destination)
    finally:
        temporary_path.unlink(missing_ok=True)
    return f"downloaded {destination}"


def published_manifest_bytes(args: argparse.Namespace) -> bytes:
    if args.manifest is not None:
        manifest_bytes = args.manifest.read_bytes()
    else:
        request = urllib.request.Request(
            args.manifest_url,
            headers={"User-Agent": "jbotci-f2llm-native-oracle/1"},
        )
        with urllib.request.urlopen(request, timeout=120) as response:
            manifest_bytes = response.read()
    return manifest_bytes


def selftest_manifest() -> dict[str, object]:
    return {
        "model_key": "selftest-model",
        "model": {
            "vocab_size": 16,
            "hidden_size": 8,
            "num_hidden_layers": 2,
            "num_attention_heads": 2,
            "num_key_value_heads": 1,
            "head_dim": 4,
            "intermediate_size": 32,
            "rms_norm_eps": 0.000001,
            "rope_theta": 10000.0,
        },
        "max_sequence_length": 32,
        "source_quantized_onnx": "/private/build-a/model.onnx",
        "source_revision": "revision-a",
        "tokenizer": {
            "url": "/private/build-a/tokenizer.json",
            "canonical_json_sha256": "1" * 64,
        },
        "tensors": {
            "weight": {
                "data": {
                    "chunks": [
                        {
                            "url": "/private/build-a/weight.bin",
                            "byte_offset": 0,
                            "byte_length": 4,
                            "sha256": "2" * 64,
                        }
                    ]
                }
            }
        },
    }


def run_selftest() -> None:
    original = selftest_manifest()
    provenance_changed = json.loads(json.dumps(original))
    provenance_changed["source_quantized_onnx"] = "/different/host/model.onnx"
    provenance_changed["source_revision"] = "revision-b"
    provenance_changed["tokenizer"]["url"] = "tokenizer.immutable.json"
    provenance_changed["tensors"]["weight"]["data"]["chunks"][0]["url"] = (
        "tensors/weight.immutable.bin"
    )
    if artifact_identity(original).digest != artifact_identity(provenance_changed).digest:
        raise AssertionError("provenance or paths changed ArtifactIdentityV1")
    print("selftest: provenance and paths do not affect ArtifactIdentityV1")

    tensor_changed = json.loads(json.dumps(original))
    tensor_changed["tensors"]["weight"]["data"]["chunks"][0]["sha256"] = "3" * 64
    if artifact_identity(original).digest == artifact_identity(tensor_changed).digest:
        raise AssertionError("tensor hash did not change ArtifactIdentityV1")
    print("selftest: a tensor hash change alters ArtifactIdentityV1")

    base = artifact_identity(original)
    tokenizer_changed = json.loads(json.dumps(original))
    tokenizer_changed["tokenizer"]["canonical_json_sha256"] = "4" * 64
    shape_changed = json.loads(json.dumps(original))
    shape_changed["model"]["hidden_size"] = 16
    diagnostic_cases = (
        (tensor_changed, "tensors"),
        (tokenizer_changed, "tokenizer"),
        (shape_changed, "shape/model fields"),
    )
    for changed_manifest, expected_category in diagnostic_cases:
        differences = identity_differences(artifact_identity(changed_manifest), base)
        if len(differences) != 1 or not differences[0].startswith(expected_category):
            raise AssertionError(
                f"expected only {expected_category!r} to diverge, got {differences!r}"
            )
    print("selftest: tensors, tokenizer, and shape/model fields have independent diagnostics")
    print("selftest: PASS")


def print_artifact_identity(manifest: dict[str, object]) -> None:
    identity = artifact_identity(manifest)
    print(f"artifact_identity={identity.digest}")
    print("component_pin=")
    print(
        json.dumps(
            {identity.digest: identity_pin(identity)},
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
    )


def main() -> None:
    args = parse_args()
    if args.selftest:
        run_selftest()
        return
    if args.jobs <= 0:
        raise ValueError("--jobs must be positive")
    published_bytes = published_manifest_bytes(args)
    published = json.loads(published_bytes)
    if not isinstance(published, dict):
        raise ValueError("published manifest root must be an object")
    if args.print_artifact_identity:
        print_artifact_identity(published)
        return
    if args.expected_artifact_identity is not None:
        verify_artifact_identity(published, args.expected_artifact_identity)
    runtime_bytes = (
        args.runtime_manifest.read_bytes()
        if args.runtime_manifest is not None
        else published_bytes
    )
    runtime = json.loads(runtime_bytes)

    available: dict[tuple[int, str], str] = {}
    for url, byte_length, digest in manifest_payloads(published):
        key = (byte_length, digest)
        prior = available.setdefault(key, url)
        if prior != url:
            # Duplicate immutable bytes may legitimately have multiple object names.
            available[key] = min(prior, url)

    requests: list[dict[str, object]] = []
    seen_destinations: set[Path] = set()
    for destination_url, byte_length, digest in manifest_payloads(runtime):
        source_url = available.get((byte_length, digest))
        if source_url is None:
            raise ValueError(
                f"runtime payload {destination_url!r} ({byte_length} bytes, {digest}) "
                "does not exist in the published manifest"
            )
        relative = normalized_relative_path(destination_url)
        destination = args.out.joinpath(*relative.parts)
        if destination in seen_destinations:
            raise ValueError(f"runtime manifest repeats payload path: {destination_url}")
        seen_destinations.add(destination)
        requests.append(
            {
                "base_url": args.base_url,
                "source_url": source_url,
                "destination": destination,
                "byte_length": byte_length,
                "sha256": digest,
            }
        )

    args.out.mkdir(parents=True, exist_ok=True)
    manifest_out = args.out / "manifest.json"
    if manifest_out.exists() and manifest_out.read_bytes() != runtime_bytes:
        raise ValueError(f"existing runtime manifest differs: {manifest_out}")
    manifest_out.write_bytes(runtime_bytes)

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
        futures = [executor.submit(fetch_one, **request) for request in requests]
        for completed, future in enumerate(
            concurrent.futures.as_completed(futures), start=1
        ):
            future.result()
            if completed % 100 == 0:
                print(f"verified {completed}/{len(requests)} payloads", flush=True)
    print(
        f"verified {len(requests)} payloads for {runtime['model_key']} under {args.out}",
        flush=True,
    )


if __name__ == "__main__":
    main()
