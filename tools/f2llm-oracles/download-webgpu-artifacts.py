#!/usr/bin/env python3
"""Download and verify an immutable published F2LLM WebGPU artifact.

The optional runtime manifest lets the native suite execute a vendored legacy manifest
against current immutable object names when every referenced payload digest is identical.
It never treats a filename match as evidence: payloads are joined by SHA-256 and byte length.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tempfile
import urllib.parse
import urllib.request


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    manifest = parser.add_mutually_exclusive_group(required=True)
    manifest.add_argument("--manifest", type=Path)
    manifest.add_argument("--manifest-url")
    parser.add_argument("--expected-manifest-sha256")
    parser.add_argument("--runtime-manifest", type=Path)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=8)
    return parser.parse_args()


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
    if args.expected_manifest_sha256 is not None:
        actual = hashlib.sha256(manifest_bytes).hexdigest()
        if actual != args.expected_manifest_sha256:
            raise ValueError(
                f"published manifest SHA-256 is {actual}, "
                f"expected {args.expected_manifest_sha256}"
            )
    elif args.manifest_url is not None:
        raise ValueError("--manifest-url requires --expected-manifest-sha256")
    return manifest_bytes


def main() -> None:
    args = parse_args()
    if args.jobs <= 0:
        raise ValueError("--jobs must be positive")
    published_bytes = published_manifest_bytes(args)
    published = json.loads(published_bytes)
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
