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
from typing import Callable


LINE = re.compile(r"\s*(\d+)(?:<([^>]*)>)?\s+(.*)")
EXIT = re.compile(r"\+\+\+ (?:exited with \d+|killed by SIG[A-Z0-9]+(?: \(core dumped\))?) \+\+\+")
TEARDOWN = re.compile(r"(?:\?\?\?\(\s*<unfinished \.\.\.>|<\.\.\. \?\?\? resumed>\)\s*=\s*\?|\?\?\?\(\)\s*=\s*\?)")


class TraceError(Exception):
    """A trace cannot establish the declared input contract."""


@dataclass
class Process:
    cwd: str
    unit: str | None = None
    via_dx: bool = False
    parent: int | None = None
    exe: str | None = None
    argument_count: int | None = None

    def identity(self) -> dict:
        return {"parent": self.parent, "exe": self.exe,
                "cmdline": {"arguments": "<redacted>", "argument_count": self.argument_count}}


def process_snapshot(pid: int) -> dict:
    """Capture diagnostic evidence while the tracee exists, without retaining arguments."""
    root = Path(f"/proc/{pid}")
    result: dict = {"source": "proc-snapshot"}
    try:
        status = dict(line.split(":", 1) for line in (root / "status").read_text().splitlines())
        result.update(parent=int(status["PPid"]), tgid=int(status["Tgid"]))
        result["exe"] = os.readlink(root / "exe")
        command = (root / "cmdline").read_bytes().rstrip(b"\0")
        result["cmdline"] = {"arguments": "<redacted>",
                             "argument_count": max(0, len(command.split(b"\0")) - 1) if command else None}
    except (OSError, KeyError, ValueError):
        pass
    return result


def operation_name(body: str) -> str:
    match = re.match(r"(?:<\.\.\. )?([a-zA-Z0-9_]+|\?\?\?)(?:\(| resumed>)", body)
    return match[1] if match else "unknown"


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
    def __init__(self, workspace: Path, target: Path, output,
                 inspect_process: Callable[[int], dict] | None = None):
        self.workspace = workspace
        self.target = target
        self.output = output
        self.processes: dict[int, Process] = {}
        self.pending: dict[int, str] = {}
        self.fork_states: dict[int, Process] = {}
        self.waiting: dict[int, list[str]] = {}
        self.identities: dict[int, dict] = {}
        self.interrupted_forks: list[dict] = []
        self.inspect_process = inspect_process
        self.errors = 0

    def emit(self, event: dict) -> None:
        encoded = json.dumps(event, ensure_ascii=True)
        self.output.write(encoded + "\n")
        if event["event"] in ("trace-error", "incomplete-processes"):
            print(f"Input trace error: {encoded}", file=sys.stderr)

    def consume(self, line: str) -> None:
        match = LINE.fullmatch(line)
        if not match:
            return
        pid, name, body = int(match[1]), match[2], match[3]
        identity = self.identities.setdefault(pid, {"exe": None, "cmdline": None, "parent": None})
        if name:
            identity["comm"] = name
        if pid not in self.processes:
            if not self.processes:
                self.processes[pid] = Process(str(self.workspace))
            else:
                if pid not in self.waiting and self.inspect_process is not None:
                    identity.update(self.inspect_process(pid))
                    self.emit({"event": "process", "pid": pid, **identity})
                if body.startswith("execve(") and body.endswith("= 0"):
                    executable, end = string_at(body)
                    arguments, _ = array_at(body, end)
                    identity.update(exe=executable, cmdline={"arguments": "<redacted>",
                                                           "argument_count": max(0, len(arguments) - 1)})
                    self.emit({"event": "process", "pid": pid, **identity})
                if EXIT.fullmatch(body) and all(
                    TEARDOWN.fullmatch(LINE.fullmatch(buffered)[3])
                    for buffered in self.waiting.get(pid, [])
                ):
                    # A dying thread can have no reported clone result. Retain the
                    # proof that every observed event is teardown, with no exec,
                    # directory change, or file open. Any other event stays queued.
                    self.emit({"event": "unattributed-exit", "pid": pid, **identity,
                               "teardown_events": len(self.waiting.get(pid, [])),
                               "observed_teardown": [LINE.fullmatch(buffered)[3]
                                                     for buffered in self.waiting.get(pid, [])],
                               "termination": body})
                    self.waiting.pop(pid, None)
                    return
                self.waiting.setdefault(pid, []).append(line)
                return
        if EXIT.fullmatch(body):
            # strace can show an unfinished syscall when a thread exits.
            # An explicit exit proves that the operation cannot return or read.
            self.pending.pop(pid, None)
            fork = self.fork_states.pop(pid, None)
            if fork is not None:
                event = {"event": "interrupted-fork", "pid": pid, **fork.identity(),
                         "comm": identity.get("comm")}
                self.interrupted_forks.append(event)
                self.emit(event)
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
            child = re.search(r"= (\d+)(?:<[^>]*>)?\s*$", body)
            if child:
                child_pid = int(child[1])
                self.processes[child_pid] = replace(self.fork_states.pop(pid, process), parent=pid)
                self.identities.setdefault(child_pid, {}).update(self.processes[child_pid].identity(), source="journal")
                self.emit({"event": "process", "pid": child_pid, **self.identities[child_pid]})
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
            process.exe = executable
            process.argument_count = max(0, len(arguments) - 1)
            identity.update(process.identity(), source="journal")
            self.emit({"event": "process", "pid": pid, **identity})
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
                match = re.match(rb"\s*(\d+)(?:<[^>]*>)?\s+(\w+)\(", raw)
                self.emit({"event": "trace-error", "pid": int(match[1]) if match else None,
                           "operation": match[2].decode("ascii") if match else "resumed",
                           "unit": self.processes.get(int(match[1]), Process(str(self.workspace))).unit if match else None,
                           "reason": str(error) if isinstance(error, TraceError) else type(error).__name__})
        if self.pending or self.waiting:
            self.errors += len(self.pending) + len(self.waiting)
            self.emit({"event": "incomplete-processes",
                       "pending": {pid: {"unit": self.processes[pid].unit,
                                         "operation": operation_name(body)} for pid, body in self.pending.items()},
                       "waiting": list(self.waiting),
                       "waiting_processes": {pid: self.identities[pid] for pid in self.waiting},
                       "interrupted_forks": self.interrupted_forks})
        self.emit({"event": "complete", "errors": self.errors})


def trace(workspace: Path, target: Path, journal: Path, command: list[str]) -> int:
    read_fd, write_fd = os.pipe()
    journal.parent.mkdir(parents=True, exist_ok=True)
    with journal.open("w") as output, os.fdopen(read_fd, "rb") as stream:
        parser = Parser(workspace, target, output, inspect_process=process_snapshot)
        reader = threading.Thread(target=parser.feed, args=(stream,))
        reader.start()
        try:
            process = subprocess.Popen([
                "strace", "-f", "--seccomp-bpf", "-q", "-v", "-s", "1048576", "-yy", "-Y",
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
