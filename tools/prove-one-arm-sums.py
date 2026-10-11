#!/usr/bin/env python3
"""Prove the approved changes in every changed fixture expectation.

Supply the old fixture directory, the new fixture directory, and both probe
output directories. Each probe record uses its fixture path as its key.
Supply the tree captures for hashed expectations with source spans.
This tool reads the fixtures. It does not write them.
"""
import argparse
import collections
import gzip
import hashlib
import json
from pathlib import Path
import tomllib

from one_arm_sums import RULE_NAMES, compare, debug_tokens, rust_string


def rule_changes(text):
    result = collections.Counter()

    def walk(items):
        for index, item in enumerate(items):
            if not isinstance(item, tuple):
                continue
            if index and items[index - 1] == "MissingRequiredField" and item[0] == "{":
                for field_index, field in enumerate(item[1]):
                    if field == "expected" and item[1][field_index + 1] == ":":
                        name = rust_string(item[1][field_index + 2])
                        if name in RULE_NAMES and RULE_NAMES[name] != name:
                            result[name] += 1
            walk(item[1])

    walk(debug_tokens(text))
    return dict(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--before-values", type=Path, required=True)
    parser.add_argument("--after-values", type=Path, required=True)
    parser.add_argument("--tree-values", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    modes = {
        ("expectations", "syntax", "raw"): "raw",
        ("expectations", "syntax", "recovered", "raw"): "recovered_raw",
        ("expectations", "output", "gentufa", "json"): "json",
        ("expectations", "output", "gentufa", "tree"): "tree",
        ("expectations", "output", "gentufa", "show-elided", "json"): "json",
        ("expectations", "output", "gentufa", "show-elided", "tree"): "tree",
    }
    before_files = set(args.before.rglob("*.toml"))
    relative_files = {path.relative_to(args.before) for path in before_files}
    assert relative_files == {path.relative_to(args.after) for path in args.after.rglob("*.toml")}, "Fixture file list changed"
    counts = collections.Counter()
    changed_files = 0
    mapped = []
    for relative in sorted(relative_files):
        old = tomllib.loads((args.before / relative).read_text())
        new = tomllib.loads((args.after / relative).read_text())
        if old == new:
            continue
        changed_files += 1
        fixture_path = "tests/fixtures/" + relative.as_posix()
        record_name = hashlib.sha256(fixture_path.encode()).hexdigest() + ".json.gz"
        mapping_fields = []

        def walk(before, after, path=()):
            if before == after:
                return
            if path in modes:
                mode = modes[path]
                if isinstance(before, dict) and isinstance(after, dict):
                    assert {key: value for key, value in before.items() if key != "sha256"} == {key: value for key, value in after.items() if key != "sha256"}, (fixture_path, path, "Hash metadata changed")
                    assert "sha256" in before and "sha256" in after
                    if mode == "tree":
                        values = json.loads((args.tree_values / (record_name + ".json")).read_text())
                        old_text, new_text = values["before"], values["after"]
                    else:
                        with gzip.open(args.before_values / record_name, "rt") as handle:
                            old_text = json.load(handle)[mode]
                        with gzip.open(args.after_values / record_name, "rt") as handle:
                            new_text = json.load(handle)[mode]
                        if mode == "json":
                            old_text = json.dumps(json.loads(old_text), ensure_ascii=False, separators=(",", ":"))
                            new_text = json.dumps(json.loads(new_text), ensure_ascii=False, separators=(",", ":"))
                    assert hashlib.sha256(old_text.encode()).hexdigest() == before["sha256"], (fixture_path, path, "Old hash differs from the capture")
                    assert hashlib.sha256(new_text.encode()).hexdigest() == after["sha256"], (fixture_path, path, "New hash differs from the capture")
                else:
                    assert isinstance(before, str) and isinstance(after, str), (fixture_path, path, "Expectation type changed")
                    old_text, new_text = before, after
                assert compare(mode, old_text, new_text) == "wrapper-only", (fixture_path, path, "Unapproved expectation change")
                counts[".".join(path[1:])] += 1
                if mode in ("raw", "recovered_raw"):
                    rules = rule_changes(old_text)
                    if rules:
                        mapping_fields.append({"field": ".".join(path[1:]), "rules": rules})
                return
            assert isinstance(before, dict) and isinstance(after, dict), (fixture_path, path, "Unapproved value change")
            assert before.keys() == after.keys(), (fixture_path, path, "Field list changed")
            for key in before:
                walk(before[key], after[key], path + (key,))

        walk(old, new)
        if mapping_fields:
            mapped.append({"path": fixture_path, "fields": mapping_fields})
        if changed_files % 1000 == 0:
            print(f"fixtures={changed_files}", flush=True)
    report = {
        "fixtures": changed_files,
        "expectations": dict(counts),
        "rule_mapping_expectations": sum(len(row["fields"]) for row in mapped),
        "rule_mapping_fixtures": mapped,
    }
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
