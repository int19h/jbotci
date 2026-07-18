#!/usr/bin/env python3
"""Import Bickr's generated OpenRouter capability map as deterministic JSON."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


ENTRY = re.compile(r'^\s*\["(?P<model>(?:[^"\\]|\\.)*)",(?P<capabilities>\{.*\})\],?\s*$')
CAPABILITY_FIELDS = (
    "prefill",
    "structuredOutputs",
    "requiredToolCalls",
    "disabledReasoning",
    "cacheControl",
)


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bickr",
        type=Path,
        default=Path.home() / "git" / "bickr",
        help="Bickr checkout containing the generated capability map",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "openrouter-model-capabilities.json",
        help="JSON snapshot to replace",
    )
    return parser.parse_args()


def main() -> None:
    args = arguments()
    source_relative = Path("packages/shared/src/openrouter-model-capabilities.generated.ts")
    source = args.bickr / source_relative
    models: dict[str, dict[str, bool]] = {}
    for line_number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), start=1):
        match = ENTRY.match(line)
        if match is None:
            continue
        model = json.loads(f'"{match.group("model")}"')
        raw = json.loads(match.group("capabilities"))
        missing = [field for field in CAPABILITY_FIELDS if field not in raw]
        if missing:
            raise ValueError(f"{source}:{line_number}: missing fields: {', '.join(missing)}")
        non_boolean = [field for field in CAPABILITY_FIELDS if type(raw[field]) is not bool]
        if non_boolean:
            raise ValueError(
                f"{source}:{line_number}: non-boolean fields: {', '.join(non_boolean)}"
            )
        if model in models:
            raise ValueError(f"{source}:{line_number}: duplicate model {model!r}")
        models[model] = {field: raw[field] for field in CAPABILITY_FIELDS}

    if not models:
        raise ValueError(f"{source}: no generated capability entries found")
    models = dict(sorted(models.items()))

    source_commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=args.bickr,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    subprocess.run(
        ["git", "diff", "--quiet", "HEAD", "--", source_relative],
        cwd=args.bickr,
        check=True,
    )
    snapshot = {
        "_provenance": {
            "source": f"https://github.com/int19h/bickr/blob/{source_commit}/{source_relative}",
            "refresh": "Run Bickr scripts/probe-openrouter-model-capabilities.mjs, then this importer.",
            "modelCount": len(models),
        },
        "models": models,
    }
    args.output.write_text(
        json.dumps(snapshot, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
