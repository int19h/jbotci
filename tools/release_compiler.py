#!/usr/bin/env python3
"""Compare the installed rustc with the channel in rust-toolchain.toml.

The release workflow calls this tool.
Pull-request CI runs the unit tests for this tool.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path


class CompilerPinError(Exception):
    """The installed rustc does not match the pin."""


def toolchain_channel(text: str) -> str:
    """Return the `channel` value from a `rust-toolchain.toml` document.

    `tomllib` accepts LF and CRLF.
    A missing channel, a blank channel, or a syntax error is a pin error.
    """
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise CompilerPinError("The toolchain file has no channel.") from error
    toolchain = data.get("toolchain")
    channel = toolchain.get("channel") if isinstance(toolchain, dict) else None
    if not isinstance(channel, str) or not channel or channel != channel.strip():
        raise CompilerPinError("The toolchain file has no channel.")
    return channel


def verbose_field(text: str, name: str) -> str:
    """Return the one `name` field from `rustc -vV` text.

    A missing field or a second field is a pin error.
    """
    prefix = f"{name}: "
    found: list[str] = []
    for line in text.splitlines():
        if line.startswith(prefix):
            found.append(line[len(prefix):])
    if len(found) != 1 or not found[0] or found[0] != found[0].strip():
        raise CompilerPinError(f"rustc -vV has no {name} field.")
    return found[0]


def require_verbose(channel: str, verbose: str, host: str | None) -> None:
    """Stop when `verbose` does not report `channel`.

    When `host` is set, the host field must equal that value.
    """
    release = verbose_field(verbose, "release")
    if release != channel:
        raise CompilerPinError("The rustc release does not match the toolchain channel.")
    if host is not None:
        actual = verbose_field(verbose, "host")
        if actual != host:
            raise CompilerPinError("The rustc host does not match the expected host.")


def version_line(text: str) -> str:
    """Return the first line of `rustc --version` text."""
    lines = text.splitlines()
    if not lines or not lines[0] or lines[0] != lines[0].strip():
        raise CompilerPinError("The rustc version line does not match the pin.")
    return lines[0]


def _run_rustc(argument: str) -> str:
    try:
        result = subprocess.run(
            ["rustc", argument],
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        raise CompilerPinError("The pinned compiler is unavailable.") from error
    if result.returncode != 0 or not result.stdout:
        raise CompilerPinError("The pinned compiler is unavailable.")
    return result.stdout


def main(argv: list[str] | None = None) -> int:
    """Run the release compiler gate.

    `gate` reads the toolchain file and `rustc -vV`.
    `expect-version` compares `rustc --version` with one expected line.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    gate = commands.add_parser("gate")
    gate.add_argument("--file", type=Path, required=True)
    gate.add_argument("--host")
    expect = commands.add_parser("expect-version")
    expect.add_argument("--expect", required=True)
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "gate":
            text = arguments.file.read_text(encoding="utf-8")
            require_verbose(toolchain_channel(text), _run_rustc("-vV"), arguments.host)
        else:
            if version_line(_run_rustc("--version")) != arguments.expect:
                raise CompilerPinError("The rustc version line does not match the pin.")
    except (OSError, UnicodeError, CompilerPinError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
