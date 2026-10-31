"""Trace build input reads without retaining command secrets or environment values."""
from __future__ import annotations

from dataclasses import dataclass, replace
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading


class TraceError(Exception):
    """A trace cannot establish the declared input contract."""


@dataclass
class Process:
    cwd: str
    unit: str | None = None
    via_dx: bool = False


def string_at(text: str, start: int = 0) -> tuple[str, int]:
    start = text.index('"', start)
    index = start + 1
    value = bytearray()
    escapes = {"n": 10, "r": 13, "t": 9, "v": 11, "f": 12, "a": 7, "b": 8, "\\": 92, '"': 34}
    while index < len(text):
        character = text[index]
        index += 1
        if character == '"':
            return os.fsdecode(bytes(value)), index
        if character != "\\":
            value.extend(character.encode("utf-8", errors="surrogateescape"))
            continue
        character = text[index]
        index += 1
        if character in escapes:
            value.append(escapes[character])
        elif character in "01234567":
            digits = character
            while len(digits) < 3 and index < len(text) and text[index] in "01234567":
                digits += text[index]
                index += 1
            value.append(int(digits, 8))
        elif character == "x":
            value.append(int(text[index:index + 2], 16))
            index += 2
        else:
            raise TraceError("Unknown trace string escape")
    raise TraceError("Incomplete quoted trace string")


def array_at(text: str, start: int) -> tuple[list[str], int]:
    start = text.index("[", start)
    index = start + 1
    values: list[str] = []
    while index < len(text):
        if text[index] == '"':
            value, index = string_at(text, index)
            values.append(value)
        elif text.startswith("...", index):
            raise TraceError("An abbreviated argument cannot establish input coverage")
        elif text[index] == "]":
            return values, index + 1
        else:
            index += 1
    raise TraceError("Incomplete trace array")


def flag(arguments: list[str], name: str) -> str | None:
    for index, argument in enumerate(arguments):
        if argument == name:
            return arguments[index + 1]
        if argument.startswith(name + "="):
            return argument[len(name) + 1:]
    return None


def extra_filename(arguments: list[str]) -> str | None:
    for argument in arguments:
        if argument.startswith("extra-filename=-"):
            return argument.removeprefix("extra-filename=-")
        if argument.startswith("-Cextra-filename=-"):
            return argument.removeprefix("-Cextra-filename=-")
    return None


class Parser:
    def __init__(self, workspace: Path, target: Path, output):
        self.workspace = workspace
        self.target = target
        self.output = output
        self.processes: dict[int, Process] = {}
        self.pending: dict[int, str] = {}
        self.fork_states: dict[int, Process] = {}
        self.waiting: dict[int, list[str]] = {}
        self.errors = 0

    def emit(self, event: dict) -> None:
        self.output.write(json.dumps(event, ensure_ascii=True) + "\n")

    def consume(self, line: str) -> None:
        match = re.match(r"\s*(\d+)\s+(.*)", line)
        if not match:
            return
        pid, body = int(match[1]), match[2]
        if pid not in self.processes:
            if not self.processes:
                self.processes[pid] = Process(str(self.workspace))
            else:
                self.waiting.setdefault(pid, []).append(line)
                return
        if "<unfinished ...>" in body:
            self.pending[pid] = body.split("<unfinished ...>", 1)[0]
            if re.match(r"(?:clone3?|fork|vfork)\(", body):
                self.fork_states[pid] = replace(self.processes[pid])
            return
        if body.startswith("<... "):
            resumed = re.match(r"<\.\.\. \w+ resumed>(.*)", body)
            if resumed:
                body = self.pending.pop(pid, "") + resumed[1]
            else:
                return
        process = self.processes.setdefault(pid, Process(str(self.workspace)))
        if re.match(r"(?:clone3?|fork|vfork)\(", body):
            child = re.search(r"= (\d+)\s*$", body)
            if child:
                child_pid = int(child[1])
                self.processes[child_pid] = self.fork_states.pop(pid, replace(process))
                for buffered in self.waiting.pop(child_pid, []):
                    self.consume(buffered)
            return
        if body.startswith("chdir(") and body.endswith("= 0"):
            directory, _ = string_at(body)
            process.cwd = str(Path(process.cwd) / directory)
            return
        if body.startswith("execve(") and body.endswith("= 0"):
            executable, end = string_at(body)
            arguments, end = array_at(body, end)
            environment, _ = array_at(body, end)
            # Never retain the raw environment or arguments in the journal.
            env = dict(item.split("=", 1) for item in environment if "=" in item)
            if Path(executable).name == "dx":
                process.via_dx = True
            package = env.get("CARGO_PKG_NAME")
            manifest = env.get("CARGO_MANIFEST_DIR")
            out = env.get("OUT_DIR")
            unit = None
            kind = None
            dep_info = None
            if package and manifest and out and Path(executable).name == "build-script-build":
                directory = Path(out).parent
                if directory.parent.name == "build" and directory.is_relative_to(self.target):
                    unit = directory.parent.parent / ".fingerprint" / directory.name
                    kind = "run"
            if package and manifest and Path(executable).name in ("rustc", "rustc.exe"):
                destination = flag(arguments, "--out-dir")
                suffix = extra_filename(arguments)
                crate = flag(arguments, "--crate-name")
                if destination and suffix and crate and re.fullmatch(r"[0-9a-f]{16}", suffix):
                    directory = Path(process.cwd) / destination
                    if directory.name in ("deps", "examples"):
                        profile = directory.parent
                    elif directory.parent.name == "build":
                        profile = directory.parent.parent
                    else:
                        profile = None
                    if profile is not None and profile.is_relative_to(self.target):
                        unit = profile / ".fingerprint" / f"{package}-{suffix}"
                        kind = "compile"
                        dep_info = str(directory / f"{crate}-{suffix}.d")
            if unit is not None and unit.is_dir():
                process.unit = str(unit.relative_to(self.target))
                self.emit({"event": "unit", "unit": process.unit, "kind": kind,
                           "package": manifest, "cwd": process.cwd, "dep_info": dep_info,
                           "via_dx": process.via_dx})
            return
        if not body.startswith(("open(", "openat(", "openat2(")):
            return
        result = re.search(r"= (\d+)<(.*)>\s*$", body)
        if not result:
            return
        _, filename_end = string_at(body)
        flags = body[filename_end:result.start()]
        if re.search(r"\bO_DIRECTORY\b", flags):
            return
        access = re.search(r"\bO_(RDONLY|WRONLY|RDWR)\b", flags)
        if access is None:
            raise TraceError("Unknown file access mode in an input trace")
        opened = result[2]
        if opened.endswith(" (deleted)"):
            opened = opened[:-10]
        if not opened.startswith("/"):
            return
        if Path(opened).is_relative_to(self.target):
            return
        writing = access[1] in ("WRONLY", "RDWR")
        if process.unit is None and not writing:
            return
        self.emit({"event": "open", "unit": process.unit, "path": opened,
                   "write": writing, "read": access[1] != "WRONLY"})

    def feed(self, stream) -> None:
        for raw in stream:
            try:
                self.consume(raw.decode("utf-8", errors="surrogateescape").rstrip("\n"))
            except (TraceError, ValueError, SyntaxError, IndexError, KeyError, OSError) as error:
                # A raw line can contain credentials. Record no part of it.
                self.errors += 1
                match = re.match(rb"\s*(\d+)\s+(\w+)\(", raw)
                self.emit({"event": "trace-error", "pid": int(match[1]) if match else None,
                           "operation": match[2].decode("ascii") if match else "resumed",
                           "unit": self.processes.get(int(match[1]), Process(str(self.workspace))).unit if match else None,
                           "reason": str(error) if isinstance(error, TraceError) else type(error).__name__})
        if self.pending or self.waiting:
            self.errors += len(self.pending) + len(self.waiting)
            self.emit({"event": "incomplete-processes",
                       "pending": {pid: {"unit": self.processes[pid].unit,
                                         "operation": body.split("(", 1)[0]} for pid, body in self.pending.items()},
                       "waiting": list(self.waiting)})
        self.emit({"event": "complete", "errors": self.errors})


def trace(workspace: Path, target: Path, journal: Path, command: list[str]) -> int:
    read_fd, write_fd = os.pipe()
    journal.parent.mkdir(parents=True, exist_ok=True)
    with journal.open("w") as output, os.fdopen(read_fd, "rb") as stream:
        parser = Parser(workspace, target, output)
        reader = threading.Thread(target=parser.feed, args=(stream,))
        reader.start()
        try:
            process = subprocess.Popen([
                "strace", "-f", "--seccomp-bpf", "-qq", "-v", "-s", "1048576", "-yy",
                "-e", "trace=execve,clone,clone3,fork,vfork,chdir,open,openat,openat2",
                "-o", f"/dev/fd/{write_fd}", "--", *command,
            ], cwd=workspace, pass_fds=(write_fd,))
        finally:
            os.close(write_fd)
        status = process.wait()
        reader.join()
    if parser.errors:
        print(f"Input trace rejected {parser.errors} operations. Read the error events in {journal}.", file=sys.stderr)
    return status if status else (1 if parser.errors else 0)
