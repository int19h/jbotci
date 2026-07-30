#!/usr/bin/env python3
"""Verify that issue #656 pins change only `modalArguments` to `adjuncts`.

The verifier covers both inline and SHA-256-only tersmu fixture pins by running
the exact base and current binaries, parsing their canonical JSON graphs, and
recursively applying the one allowed object-key rename to the base graph.  It
also checks the four changed Phase-B frozen graphs and rejects fixture edits
outside the tersmu expectation.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import tomllib
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FIXTURE_ROOT = Path("tests/fixtures")
FROZEN_ROOT = Path("crates/jbotci-semantics/tests/phaseb_corpus")
OLD_KEY = "modalArguments"
NEW_KEY = "adjuncts"


@dataclass(frozen=True)
class Verification:
    path: Path
    pin_kind: str
    renamed_keys: int


@dataclass(frozen=True)
class RenderedGraph:
    graph: Any
    canonical_json: str


def run(*args: str, input_text: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        check=False,
        text=True,
        input=input_text,
        capture_output=True,
    )


def git_source(base_ref: str, path: Path) -> str:
    result = run("git", "show", f"{base_ref}:{path.as_posix()}")
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return result.stdout


def changed_paths(base_ref: str, root: Path) -> list[Path]:
    result = run("git", "diff", "--name-only", base_ref, "--", root.as_posix())
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return sorted(
        Path(line)
        for line in result.stdout.splitlines()
        if line and Path(line).is_file()
    )


def changed_fixture_paths(base_ref: str) -> list[Path]:
    paths = changed_paths(base_ref, FIXTURE_ROOT)
    unexpected = [path for path in paths if path.suffix != ".toml"]
    if unexpected:
        raise ValueError(f"non-TOML fixture changes: {unexpected}")
    return paths


def changed_frozen_paths(base_ref: str) -> list[Path]:
    paths = changed_paths(base_ref, FROZEN_ROOT)
    return [path for path in paths if path.name.endswith(".frozen.json")]


def rename_keys(value: Any) -> tuple[Any, int]:
    if isinstance(value, list):
        items = []
        count = 0
        for item in value:
            renamed, item_count = rename_keys(item)
            items.append(renamed)
            count += item_count
        return items, count
    if not isinstance(value, dict):
        return value, 0

    result: dict[str, Any] = {}
    count = 0
    for key, item in value.items():
        new_key = NEW_KEY if key == OLD_KEY else key
        if new_key in result:
            raise ValueError(
                f"key rename collision: object contains both {OLD_KEY!r} and {NEW_KEY!r}"
            )
        renamed, item_count = rename_keys(item)
        result[new_key] = renamed
        count += item_count + (key == OLD_KEY)
    return result, count


def without_tersmu(document: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(document)
    output = result.get("expectations", {}).get("output")
    if isinstance(output, dict):
        output.pop("tersmu", None)
        if not output:
            result["expectations"].pop("output")
    return result


def tersmu_pin_kind(document: dict[str, Any]) -> str:
    tersmu = document["expectations"]["output"]["tersmu"]
    value = tersmu["json"]
    if isinstance(value, str):
        return "inline"
    if isinstance(value, dict) and set(value) == {"sha256"}:
        return "sha256"
    raise ValueError(f"unsupported tersmu JSON expectation shape: {value!r}")


def fixture_input(path: Path, document: dict[str, Any]) -> str:
    if "lojban" in document:
        return str(document["lojban"])
    if "lojban-filename" in document:
        return (path.parent / str(document["lojban-filename"])).read_text()
    raise ValueError(f"{path}: fixture has no Lojban input")


def render(binary: Path, path: Path, document: dict[str, Any]) -> RenderedGraph:
    command = [
        str(binary),
        "tersmu",
        "--color=never",
        "--format",
        "json",
    ]
    if dialect := document.get("dialect"):
        command.extend(("--dialect", str(dialect)))
    tersmu = document["expectations"]["output"]["tersmu"]
    if tersmu.get("story-time") is True:
        command.append("--story-time")
    command.extend(("--file", "/dev/stdin"))
    result = run(*command, input_text=fixture_input(path, document))
    if result.returncode != 0:
        raise ValueError(
            f"{path}: {binary} failed with status {result.returncode}:\n"
            f"{result.stderr[-2000:]}"
        )
    canonical_json = result.stdout.removesuffix("\n")
    try:
        graph = json.loads(canonical_json)
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: {binary} emitted invalid JSON: {error}") from error
    return RenderedGraph(graph=graph, canonical_json=canonical_json)


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def replace_once(source: str, old: str, new: str, *, path: Path) -> str:
    if source.count(old) != 1:
        raise ValueError(
            f"{path}: expected exactly one occurrence of the changed pin, "
            f"found {source.count(old)}"
        )
    return source.replace(old, new, 1)


def verify_fixture(
    base_ref: str,
    base_binary: Path,
    current_binary: Path,
    path: Path,
) -> Verification:
    base_source = git_source(base_ref, path)
    current_source = path.read_text()
    base_document = tomllib.loads(base_source)
    current_document = tomllib.loads(current_source)
    if without_tersmu(base_document) != without_tersmu(current_document):
        raise ValueError(f"{path}: changed beyond its tersmu expectation")

    base_rendered = render(base_binary, path, base_document)
    current_rendered = render(current_binary, path, current_document)
    expected_graph, renamed_count = rename_keys(base_rendered.graph)
    if renamed_count == 0:
        raise ValueError(f"{path}: changed tersmu pin has no {OLD_KEY!r} key")
    if expected_graph != current_rendered.graph:
        raise ValueError(f"{path}: current graph differs beyond the allowed key rename")

    pin_kind = tersmu_pin_kind(current_document)
    if pin_kind == "inline":
        base_pin = base_document["expectations"]["output"]["tersmu"]["json"]
        current_pin = current_document["expectations"]["output"]["tersmu"]["json"]
        expected_pin = base_pin.replace(f'"{OLD_KEY}"', f'"{NEW_KEY}"')
        if expected_pin != current_pin:
            raise ValueError(f"{path}: inline pin is not a literal key-only substitution")
        pinned_graph = json.loads(current_pin)
        if pinned_graph != current_rendered.graph:
            raise ValueError(f"{path}: inline current tersmu pin is stale")
        expected_source = replace_once(base_source, base_pin, current_pin, path=path)
    else:
        base_hash = base_document["expectations"]["output"]["tersmu"]["json"]["sha256"]
        current_hash = current_document["expectations"]["output"]["tersmu"]["json"]["sha256"]
        actual_base_hash = sha256(base_rendered.canonical_json)
        actual_current_hash = sha256(current_rendered.canonical_json)
        if base_hash != actual_base_hash:
            raise ValueError(f"{path}: baseline SHA-256 pin is stale")
        if current_hash != actual_current_hash:
            raise ValueError(f"{path}: current SHA-256 pin is stale")
        expected_source = replace_once(base_source, base_hash, current_hash, path=path)
    if expected_source != current_source:
        raise ValueError(f"{path}: fixture text changed beyond its tersmu pin")
    return Verification(path=path, pin_kind=pin_kind, renamed_keys=renamed_count)


def verify_frozen(base_ref: str, path: Path) -> Verification:
    base_source = git_source(base_ref, path)
    current_source = path.read_text()
    base_graph = json.loads(base_source)
    current_graph = json.loads(current_source)
    expected_graph, renamed_count = rename_keys(base_graph)
    if renamed_count == 0:
        raise ValueError(f"{path}: changed frozen graph has no {OLD_KEY!r} key")
    if expected_graph != current_graph:
        raise ValueError(f"{path}: frozen graph differs beyond the allowed key rename")
    expected_source = base_source.replace(f'"{OLD_KEY}"', f'"{NEW_KEY}"')
    if expected_source != current_source:
        raise ValueError(f"{path}: frozen graph text is not a literal key-only substitution")
    return Verification(path=path, pin_kind="frozen", renamed_keys=renamed_count)


def baseline_paths_containing(base_ref: str, root: Path) -> set[Path]:
    result = run(
        "git",
        "grep",
        "-l",
        f'"{OLD_KEY}"',
        base_ref,
        "--",
        root.as_posix(),
    )
    if result.returncode not in (0, 1):
        raise ValueError(result.stderr.strip())
    prefix = f"{base_ref}:"
    return {
        Path(line.removeprefix(prefix))
        for line in result.stdout.splitlines()
        if line
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--base-ref",
        default="740cfe18eee650d8c7fc5aa7cae16e9f08743bec",
    )
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=8)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.jobs < 1:
        raise ValueError("--jobs must be at least 1")
    for binary in (args.base_binary, args.current_binary):
        if not binary.is_file():
            raise ValueError(f"binary does not exist: {binary}")

    fixtures = changed_fixture_paths(args.base_ref)
    inline_baseline = baseline_paths_containing(args.base_ref, FIXTURE_ROOT)
    missing_inline = inline_baseline.difference(fixtures)
    if missing_inline:
        raise ValueError(
            "baseline inline pins containing modalArguments were not regenerated: "
            + ", ".join(str(path) for path in sorted(missing_inline))
        )

    with ThreadPoolExecutor(max_workers=args.jobs) as executor:
        fixture_results = list(
            executor.map(
                lambda path: verify_fixture(
                    args.base_ref,
                    args.base_binary,
                    args.current_binary,
                    path,
                ),
                fixtures,
            )
        )
    frozen_paths = changed_frozen_paths(args.base_ref)
    frozen_baseline = baseline_paths_containing(args.base_ref, FROZEN_ROOT)
    missing_frozen = frozen_baseline.difference(frozen_paths)
    if missing_frozen:
        raise ValueError(
            "baseline frozen graphs containing modalArguments were not regenerated: "
            + ", ".join(str(path) for path in sorted(missing_frozen))
        )
    frozen_results = [
        verify_frozen(args.base_ref, path)
        for path in frozen_paths
    ]

    results = fixture_results + frozen_results
    kinds = {
        kind: sum(result.pin_kind == kind for result in results)
        for kind in ("inline", "sha256", "frozen")
    }
    print(
        "issue-656 fixture migration verified: "
        f"fixture_pins={len(fixture_results)} "
        f"inline={kinds['inline']} "
        f"sha256={kinds['sha256']} "
        f"frozen={kinds['frozen']} "
        f"renamed_keys={sum(result.renamed_keys for result in results)}"
    )


if __name__ == "__main__":
    main()
