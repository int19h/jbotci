#!/usr/bin/env python3
"""Capture each fixture with a probe built against one pinned revision."""
import argparse
import concurrent.futures
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import threading
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    assert args.jobs > 0
    args.output.mkdir(parents=True, exist_ok=True)
    local = threading.local()
    processes = []
    lock = threading.Lock()

    def capture(path):
        fixture = tomllib.loads(path.read_text())
        text = fixture.get("lojban")
        if text is None and "lojban-filename" in fixture:
            with (path.parent / fixture["lojban-filename"]).open(newline="") as handle:
                text = handle.read()
        if text is None:
            return False
        if not hasattr(local, "process"):
            local.process = subprocess.Popen(
                [str(args.probe.resolve())],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                text=True,
            )
            with lock:
                processes.append(local.process)
        process = local.process
        relative = "tests/fixtures/" + path.relative_to(args.root).as_posix()
        request = {
            "path": relative,
            "text": text,
            "dialect": fixture.get("dialect"),
            "max_errors": fixture.get("expectations", {}).get("syntax", {}).get("recovered", {}).get("max-errors"),
        }
        process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        line = process.stdout.readline()
        assert line, (relative, process.poll())
        record = json.loads(line)
        assert record["path"] == relative
        name = hashlib.sha256(relative.encode()).hexdigest() + ".json.gz"
        with gzip.open(args.output / name, "wt", compresslevel=1) as output:
            output.write(line)
        return True

    count = 0
    try:
        with concurrent.futures.ThreadPoolExecutor(args.jobs) as executor:
            for captured in executor.map(capture, sorted(args.root.rglob("*.toml"))):
                count += int(captured)
                if captured and count % 1000 == 0:
                    print(f"fixtures={count}", flush=True)
    finally:
        for process in processes:
            process.stdin.close()
            assert process.wait() == 0
    print(f"fixtures={count}", flush=True)


if __name__ == "__main__":
    main()
