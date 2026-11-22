"""Trace build input reads without retaining command secrets or environment values."""
from __future__ import annotations

from dataclasses import dataclass, replace
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import threading
import tomllib
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
    unit_kind: str | None = None
    via_dx: bool = False
    from_pinned_dx: bool = False
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


UNIT_ID = re.compile(r"[0-9a-f]{16}")
CLASSIC_DIR = re.compile(r"-[0-9a-f]{16}$")


def cargo_rustc_metadata(arguments: list[str]) -> bool:
    """Return true when rustc arguments carry Cargo's unit metadata hash."""
    return any(re.fullmatch(r"(?:-C)?metadata=[0-9a-f]{16}", argument) for argument in arguments)


def metadata_free_probe(arguments: list[str]) -> bool:
    """Return true for a `-vV` or `--print` rustc probe with no Cargo metadata."""
    if cargo_rustc_metadata(arguments):
        return False
    return "-vV" in arguments or has_flag(arguments, "--print")


RUSTC_NAMES = ("rustc", "rustc.exe")
COMPILER_WRAPPER_ENV = ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER")
COMPILER_CONFIG_KEYS = {"rustc", "rustc-wrapper", "rustc-workspace-wrapper"}
UNSUPPORTED_COMPILER = "The compiler configuration is not supported."
EXACT_TEST_MARKERS = (
    "test-lib-",
    "test-bin-",
    "test-integration-test-",
    "test-example-",
    "test-bench-",
)


def has_flag(arguments: list[str], name: str) -> bool:
    """Return true when `name` is present and has a value."""
    for index, argument in enumerate(arguments):
        if argument == name and index + 1 < len(arguments):
            return True
        if argument.startswith(name + "="):
            return True
    return False


def cargo_compiler_arguments(arguments: list[str]) -> bool:
    """Return true when arguments carry `--crate-name` and Cargo's metadata hash."""
    return has_flag(arguments, "--crate-name") and cargo_rustc_metadata(arguments)


def is_compiler_exec(executable: Path, real_executable: Path, arguments: list[str]) -> bool:
    """Return true when this exec runs a compiler.

    A `rustc` name, a real path named `rustc`, or Cargo compiler arguments is that exec.
    """
    if executable.name in RUSTC_NAMES or real_executable.name in RUSTC_NAMES:
        return True
    return cargo_compiler_arguments(arguments)


def real_path(value: str, cwd: str) -> Path:
    """Return the real path of one trace path."""
    return Path(os.path.realpath(normalized_path(value, cwd)))


def search_path(name: str, path_value: str, cwd: str) -> Path | None:
    """Return the first executable PATH entry for a bare command name."""
    for entry in path_value.split(os.pathsep):
        directory = Path(cwd if entry == "" else entry)
        if not directory.is_absolute():
            directory = normalized_path(str(directory), cwd)
        candidate = directory / name
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


def executed_file(executable: str, cwd: str, env: dict[str, str]) -> Path:
    """Return the real file an execve loads.

    A name with no slash is resolved through PATH, as execve does.
    A path with a slash is relative to the process directory.
    """
    if "/" not in executable:
        # execve searches PATH before it installs the new environment.
        # Cargo and Command::new copy the caller PATH into that environment.
        found = search_path(executable, env.get("PATH", ""), cwd)
        if found is None:
            return normalized_path(executable, cwd)
        return Path(os.path.realpath(found))
    return real_path(executable, cwd)


def rustup_binary() -> Path | None:
    """Return the real rustup binary, or None when rustup is absent."""
    found = shutil.which("rustup")
    if found is None:
        return None
    path = Path(os.path.realpath(found))
    return path if path.is_file() else None


def query_rustup_rustc(
    rustup: Path, cwd: str, toolchain: str | None, home: str | None,
) -> Path | None:
    """Return the real rustc that this rustup proxy selects from `cwd`."""
    if not Path(cwd).is_dir():
        return None
    clean = os.environ.copy()
    if toolchain is None:
        clean.pop("RUSTUP_TOOLCHAIN", None)
    else:
        clean["RUSTUP_TOOLCHAIN"] = toolchain
    if home is None:
        clean.pop("RUSTUP_HOME", None)
    else:
        clean["RUSTUP_HOME"] = home
    result = subprocess.run(
        [str(rustup), "which", "rustc"],
        cwd=cwd,
        env=clean,
        check=False,
        capture_output=True,
        text=True,
    )
    raw = result.stdout.strip()
    if result.returncode != 0 or not raw:
        return None
    path = Path(os.path.realpath(raw))
    return path if path.is_file() else None


def pinned_channel(workspace: Path) -> str:
    """Return the toolchain channel from `rust-toolchain.toml` in `workspace`."""
    path = workspace / "rust-toolchain.toml"
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise TraceError("The pinned compiler is unavailable.") from error
    toolchain = data.get("toolchain")
    channel = toolchain.get("channel") if isinstance(toolchain, dict) else None
    if not isinstance(channel, str) or not channel or channel != channel.strip():
        raise TraceError("The pinned compiler is unavailable.")
    return channel


DX_VERSION = "0.7.9"


def dx_version_line_matches(line: str) -> bool:
    """Return true when `line` is `dioxus 0.7.9` with no longer version token.

    A parenthetical hash can follow the version.
    A suffix on the version does not match.
    """
    prefix = f"dioxus {DX_VERSION}"
    if line == prefix:
        return True
    return line.startswith(prefix) and line[len(prefix)].isspace()


def pinned_dx(env: dict[str, str], cwd: str) -> Path | None:
    """Return the real path of `dx` 0.7.9 on `PATH`, or None when it is absent.

    The Render image job installs this version and puts that file on `PATH`.
    A different version is not the workspace-wrapper exception.
    """
    found = search_path("dx", env.get("PATH", ""), cwd)
    if found is None:
        return None
    path = Path(os.path.realpath(found))
    if not path.is_file():
        return None
    result = subprocess.run(
        [str(path), "--version"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        return None
    if not dx_version_line_matches(result.stdout.splitlines()[0]):
        return None
    return path


def version_line_matches(line: str, channel: str) -> bool:
    """Return true when `line` is `rustc <channel>` with no longer version token.

    A parenthetical hash can follow the channel.
    A suffix on the channel does not match.
    """
    prefix = f"rustc {channel}"
    if line == prefix:
        return True
    return line.startswith(prefix) and line[len(prefix)].isspace()


def compiler_version_line(compiler: Path) -> str:
    """Return the first line of `compiler --version`."""
    result = subprocess.run(
        [str(compiler), "--version"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0 or not result.stdout:
        raise TraceError("The pinned compiler is unavailable.")
    return result.stdout.splitlines()[0]


def accept_pinned_compiler(path: Path, workspace: Path) -> Path:
    """Return `path` when it is the workspace pin and not the rustup proxy."""
    proxy = rustup_binary()
    if proxy is not None:
        try:
            same = os.path.samefile(path, proxy)
        except OSError as error:
            raise TraceError("The pinned compiler is unavailable.") from error
        if same:
            raise TraceError("The pinned compiler is the rustup proxy.")
    if not version_line_matches(compiler_version_line(path), pinned_channel(workspace)):
        raise TraceError("The compiler version is not the pinned toolchain.")
    return path


def supported_compiler(workspace: Path) -> Path:
    """Return the real path of the rustc for this workspace.

    `rustup which rustc` selects the toolchain file in `workspace`.
    `RUSTUP_TOOLCHAIN` is cleared for that one call, so the workspace pin decides.
    The version line must equal `rustc` plus the channel in `rust-toolchain.toml`.
    The resolved file must not be the rustup proxy.
    When `rustup` is absent, `rustc` on `PATH` is that compiler.
    """
    rustup = shutil.which("rustup")
    if rustup is not None:
        # The workspace file selects the toolchain. An ambient override must not.
        env = os.environ.copy()
        env.pop("RUSTUP_TOOLCHAIN", None)
        result = subprocess.run(
            [rustup, "which", "rustc"],
            cwd=workspace,
            env=env,
            check=False,
            capture_output=True,
            text=True,
        )
        raw = result.stdout.strip()
        if result.returncode != 0 or not raw:
            raise TraceError("The pinned compiler is unavailable.")
    else:
        found = shutil.which("rustc")
        if found is None:
            raise TraceError("The pinned compiler is unavailable.")
        raw = found
    path = Path(os.path.realpath(raw))
    if not path.is_file():
        raise TraceError("The pinned compiler is unavailable.")
    return accept_pinned_compiler(path, workspace)


def build_unit_fingerprint(out_dir: Path) -> Path | None:
    """Return the fingerprint directory for `<profile>/build/<package>/<16 hex>/out`."""
    if out_dir.name != "out" or not UNIT_ID.fullmatch(out_dir.parent.name):
        return None
    if len(out_dir.parents) < 3 or out_dir.parents[2].name != "build":
        return None
    return out_dir.parent / "fingerprint"


def classic_fingerprint(target: Path, out_dir: Path, package: str, suffix: str) -> Path | None:
    if out_dir.name in ("deps", "examples"):
        profile = out_dir.parent
    elif out_dir.parent.name == "build":
        profile = out_dir.parent.parent
        named = classic_package_dir(out_dir.name)
        if named is not None and named[0] != package:
            raise TraceError(f"The target package {named[0]} does not match CARGO_PKG_NAME {package}")
    else:
        return None
    if not profile_inside_target(target, profile):
        return None
    return profile / ".fingerprint" / f"{package}-{suffix}"


def normalized_path(value: str, cwd: str) -> Path:
    """Join a trace path to the process directory without following a symlink."""
    path = Path(value)
    if not path.is_absolute():
        path = Path(cwd) / path
    return Path(os.path.normpath(path))


def classic_package_dir(name: str) -> tuple[str, str] | None:
    match = CLASSIC_DIR.search(name)
    if match is None:
        return None
    package = name[:match.start()]
    if not package:
        return None
    return package, name[match.start() + 1:]


def profile_inside_target(target: Path, profile: Path) -> bool:
    return profile.is_relative_to(target) and profile != target


def under_profile_build(target: Path, executable: Path) -> bool:
    """Return true when `executable` is under a profile `build` directory."""
    if not executable.is_relative_to(target) or executable == target:
        return False
    current = executable.parent
    while current.is_relative_to(target) and current != target:
        if current.name == "build" and profile_inside_target(target, current.parent):
            return True
        parent = current.parent
        if parent == current:
            break
        current = parent
    return False


def profiles_match(host_profile: Path, out_profile: Path, requested_target: str | None) -> bool:
    """Return true when both directories are one Cargo build's profiles.

    Without `--target`, the build script and `OUT_DIR` share one profile.
    With `--target`, Cargo writes the host script under `<root>/<profile>`
    and `OUT_DIR` under `<root>/<TARGET>/<profile>`.
    `TARGET` is the build-script environment value.
    `<root>` is the traced target, or a nested target directory inside it.
    A split layout with no `TARGET`, or a different directory name, is not a match.
    """
    if out_profile == host_profile:
        return True
    if out_profile.name != host_profile.name:
        return False
    inserted = out_profile.parent
    if inserted.parent != host_profile.parent:
        return False
    return requested_target is not None and inserted.name == requested_target


def classic_build_script_fingerprint(target: Path, executable: Path, out_dir: Path,
                                      cargo_package: str, requested_target: str | None) -> Path | None:
    """Return the classic run fingerprint for one build-script executable.

    Cargo 1.99 runs `<profile>/build/<package>-<16 hex>/build-script-<stem>`.
    `OUT_DIR` is `<profile>/build/<package>-<16 hex>/out` for the run unit.
    Those two suffixes are different directories of one package.
    With `--target`, the script profile is `<root>/<profile>` and `OUT_DIR`
    is under `<root>/<TARGET>/<profile>`. The run fingerprint follows `OUT_DIR`.
    The package name in that path must equal CARGO_PKG_NAME.
    Return None when `executable` is not in that place.
    """
    if not executable.name.startswith("build-script-") or executable.name == "build-script-":
        return None
    script_dir = executable.parent
    parsed = classic_package_dir(script_dir.name)
    if parsed is None or script_dir.parent.name != "build":
        return None
    package, _compile_hash = parsed
    profile = script_dir.parent.parent
    if not profile_inside_target(target, profile):
        raise TraceError("Unknown build script unit layout")
    out_package = classic_package_dir(out_dir.parent.name)
    out_build = out_dir.parent.parent
    out_profile = out_build.parent
    if (
        out_dir.name != "out"
        or out_package is None
        or out_package[0] != package
        or out_build.name != "build"
        or not profiles_match(profile, out_profile, requested_target)
        or not profile_inside_target(target, out_profile)
        or not out_dir.is_relative_to(target)
    ):
        raise TraceError("Unknown build script unit layout")
    if package != cargo_package:
        raise TraceError(f"The target package {package} does not match CARGO_PKG_NAME {cargo_package}")
    return out_profile / ".fingerprint" / out_dir.parent.name


def fingerprint_names(fingerprint: Path, prefix: str) -> bool:
    if not fingerprint.is_dir():
        return False
    return any(entry.name.startswith(prefix) for entry in fingerprint.iterdir())


def build_unit_executable_fingerprint(target: Path, executable: Path) -> Path | None:
    """Return `<id>/fingerprint` for one Cargo 1.100 unit executable.

    The place is `<profile>/build/<package>/<16 hex>/out/<name>`.
    Return None when `executable` is not in that place.
    """
    if executable.parent.name != "out":
        return None
    compile_dir = executable.parent.parent
    if not UNIT_ID.fullmatch(compile_dir.name):
        return None
    package_dir = compile_dir.parent
    if package_dir.parent.name != "build":
        return None
    profile = package_dir.parent.parent
    if not profile_inside_target(target, profile):
        return None
    return compile_dir / "fingerprint"


def regular_file(entry: Path) -> bool:
    """Return true when `entry` is a regular file and not a link."""
    try:
        return stat.S_ISREG(entry.lstat().st_mode)
    except OSError:
        return False


def test_marker_kind(fingerprint: Path) -> str:
    """Return `test`, `unknown`, or `absent` for one compile fingerprint.

    Cargo 1.100 writes `test-lib-`, `test-bin-`, `test-integration-test-`,
    `test-example-`, and `test-bench-`.
    `test-bench-` is the bench marker. Cargo runs a bench as a test.
    An exact marker must be a regular file.
    A directory, a link, or another entry with a marker name is an unknown shape.
    Another `test-` or `bench-` name is an unknown shape.
    A `.json` sidecar is not a marker.
    """
    if not fingerprint.is_dir():
        return "absent"
    exact = False
    unknown = False
    for entry in fingerprint.iterdir():
        name = entry.name
        if name.endswith(".json"):
            continue
        if name.startswith(EXACT_TEST_MARKERS):
            if regular_file(entry):
                exact = True
            else:
                unknown = True
        elif name.startswith(("test-", "bench-")):
            unknown = True
    if unknown:
        return "unknown"
    if exact:
        return "test"
    return "absent"


def has_unit_id_child(path: Path) -> bool:
    """If a direct child is a 16-hex unit directory, return true."""
    return any(child.is_dir() and UNIT_ID.fullmatch(child.name) for child in path.iterdir())


def is_unit_out_directory(path: Path) -> bool:
    """Return true for one Cargo unit `out` directory.

    A package name can end in 16 hex characters.
    A 16-hex child makes that directory a 1.100 package, not a 1.99 output directory.
    """
    if path.name != "out":
        return False
    parent = path.parent
    if parent.parent.name == "build" and classic_package_dir(parent.name) is not None:
        return not has_unit_id_child(parent)
    return bool(UNIT_ID.fullmatch(parent.name) and parent.parent.parent.name == "build")


def inside_unit_out(root: Path, path: Path) -> bool:
    """If `path` is strictly inside a unit `out` directory under `root`, return true."""
    if not path.is_relative_to(root) or path == root:
        return False
    current = path.parent
    while current.is_relative_to(root) and current != root:
        if is_unit_out_directory(current):
            return True
        if current.parent == current:
            break
        current = current.parent
    return False


def fingerprint_ready(path: Path, run: bool) -> None:
    """If `path` cannot be the claimed unit, raise TraceError.

    Cargo creates the fingerprint directory before it starts rustc or the script.
    Cargo writes the hash file and the `run-build-script` file after that process exits.
    A missing directory is an error.
    A directory that holds a marker of the other kind is an error.
    """
    reason = "Unknown build script unit layout" if run else "Unknown rustc unit layout"
    if not path.is_dir():
        raise TraceError(reason)
    for entry in path.iterdir():
        name = entry.name
        if name in ("invoked.timestamp", "jbotci-inputs-v1.json") or name.startswith(("dep-", "output-")):
            continue
        if run and not name.startswith("run-build-script-"):
            raise TraceError(reason)
        if not run and name.startswith("run-build-script-"):
            raise TraceError(reason)


def build_unit_build_script_fingerprint(target: Path, executable: Path, out_dir: Path,
                                         cargo_package: str, requested_target: str | None) -> Path | None:
    """Return the build-unit run fingerprint for one build-script executable.

    Cargo 1.100 runs `<profile>/build/<package>/<compile id>/out/build_script_<stem>`.
    That compile unit's fingerprint holds a `build-script-*` file.
    `OUT_DIR` is `<profile>/build/<package>/<run id>/out` for a different id.
    With `--target`, the script profile is `<root>/<profile>` and `OUT_DIR`
    is under `<root>/<TARGET>/<profile>`. The run fingerprint follows `OUT_DIR`.
    The package name in that path must equal CARGO_PKG_NAME.
    Return None when `executable` is not in that place.
    Once that place matches, a missing compile fingerprint is an error.
    """
    if not executable.name.startswith("build_script_") or executable.name == "build_script_":
        return None
    if executable.parent.name != "out":
        return None
    compile_dir = executable.parent.parent
    if not UNIT_ID.fullmatch(compile_dir.name):
        return None
    package_dir = compile_dir.parent
    build_dir = package_dir.parent
    # The package name can end in 16 hex characters. The `out/<id>` shape is the layout.
    if build_dir.name != "build":
        return None
    profile = build_dir.parent
    if not profile_inside_target(target, profile):
        raise TraceError("Unknown build script unit layout")
    compile_fingerprint = compile_dir / "fingerprint"
    if not fingerprint_names(compile_fingerprint, "build-script-"):
        raise TraceError("Unknown build script unit layout")
    run_id = out_dir.parent.name
    out_package = out_dir.parent.parent
    out_build = out_package.parent
    out_profile = out_build.parent
    if (
        out_dir.name != "out"
        or not UNIT_ID.fullmatch(run_id)
        or run_id == compile_dir.name
        or out_build.name != "build"
        or out_package.name != package_dir.name
        or not profiles_match(profile, out_profile, requested_target)
        or not profile_inside_target(target, out_profile)
        or not out_dir.is_relative_to(target)
    ):
        raise TraceError("Unknown build script unit layout")
    if package_dir.name != cargo_package:
        raise TraceError(
            f"The target package {package_dir.name} does not match CARGO_PKG_NAME {cargo_package}")
    return out_dir.parent / "fingerprint"


class Parser:
    def __init__(self, workspace: Path, target: Path, output,
                 inspect_process: Callable[[int], dict] | None = None,
                 compiler: Path = Path("/usr/bin/rustc"),
                 rustup: Path | None = None,
                 dx: Path | None = None):
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
        self.root_pid: int | None = None
        self.cargo_home: Path | None = None
        # Unit traces use `/usr/bin/rustc` unless the caller passes the resolved compiler.
        self.compiler = Path(os.path.realpath(compiler))
        self.rustup = rustup
        # None means this trace has no dx 0.7.9 exception.
        self.dx = None if dx is None else Path(os.path.realpath(dx))
        self.proxy_cache: dict[tuple[str, str | None, str | None], Path | None] = {}

    def resolves_compiler(self, value: str, cwd: str, env: dict[str, str]) -> bool:
        """Return true when `value` loads the supported compiler."""
        return self.resolved_compiler(executed_file(value, cwd, env), cwd, env) == self.compiler

    def resolved_compiler(self, loaded: Path, cwd: str, env: dict[str, str]) -> Path:
        """Return the compiler a loaded file will run.

        The rustup proxy is not the compiler file. `rustup which rustc` from the
        process directory selects the toolchain that the proxy will exec.
        """
        if self.rustup is None:
            return loaded
        try:
            proxy = os.path.samefile(loaded, self.rustup)
        except OSError:
            return loaded
        if not proxy:
            return loaded
        toolchain = env.get("RUSTUP_TOOLCHAIN")
        home = env.get("RUSTUP_HOME")
        key = (cwd, toolchain, home)
        if key not in self.proxy_cache:
            self.proxy_cache[key] = query_rustup_rustc(self.rustup, cwd, toolchain, home)
        selected = self.proxy_cache[key]
        return selected if selected is not None else loaded

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
                self.root_pid = pid
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
            if pid == self.root_pid:
                cargo_home = env.get("CARGO_HOME")
                if cargo_home:
                    self.cargo_home = normalized_path(cargo_home, process.cwd)
            # An empty value clears a wrapper. Cargo uses that form to keep the default compiler.
            # RUSTC_WRAPPER has no dx exception.
            # RUSTC_WORKSPACE_WRAPPER can name only the recorded dx 0.7.9 file.
            rustc_wrapper_name, workspace_wrapper_name = COMPILER_WRAPPER_ENV
            if env.get(rustc_wrapper_name):
                raise TraceError(UNSUPPORTED_COMPILER)
            workspace_wrapper = env.get(workspace_wrapper_name) or ""
            wrapper_file = None
            if workspace_wrapper:
                wrapper_file = executed_file(workspace_wrapper, process.cwd, env)
                if self.dx is None or wrapper_file != self.dx:
                    raise TraceError(UNSUPPORTED_COMPILER)
            # Cargo sets RUSTC on a build script to the compiler it resolved.
            # A bare name or the rustup proxy must resolve to the supported compiler.
            rustc_value = env.get("RUSTC")
            if rustc_value and not self.resolves_compiler(rustc_value, process.cwd, env):
                raise TraceError(UNSUPPORTED_COMPILER)
            executable_name = Path(executable).name
            resolved = normalized_path(executable, process.cwd)
            # A bare name is resolved through PATH. The loaded file can be the rustup proxy.
            real_exec = executed_file(executable, process.cwd, env)
            if self.dx is not None and real_exec == self.dx:
                process.from_pinned_dx = True
            if executable_name == "dx" or process.from_pinned_dx:
                process.via_dx = True
            compiler_file = self.resolved_compiler(real_exec, process.cwd, env)
            # Cargo runs the pinned dx file as `dx <compiler> <arguments>`.
            # The child rustc records the unit. This exec must not.
            if wrapper_file is not None and real_exec == self.dx:
                if len(arguments) < 2 or not self.resolves_compiler(arguments[1], process.cwd, env):
                    raise TraceError(UNSUPPORTED_COMPILER)
                return
            # A compiler exec must use the supported compiler path.
            # A rustc name, a real path named rustc, or Cargo compiler arguments selects that exec.
            # A metadata-free nightly probe under the pinned dx is not a compile.
            if is_compiler_exec(Path(executable), real_exec, arguments) and compiler_file != self.compiler:
                if not (process.from_pinned_dx and metadata_free_probe(arguments)):
                    raise TraceError(UNSUPPORTED_COMPILER)
            package = env.get("CARGO_PKG_NAME")
            manifest = env.get("CARGO_MANIFEST_DIR")
            out = env.get("OUT_DIR")
            requested_target = env.get("TARGET") or None
            unit = None
            kind = None
            dep_info = None
            # A candidate comes from the operation position in the traced target.
            # When its name starts with `build-script-` or `build_script_`, an executable
            # under a profile `build` directory is a build-script candidate.
            # When that executable is in the Cargo 1.100 unit place, read its fingerprint first.
            # An exact marker is `test-lib-`, `test-bin-`, `test-integration-test-`,
            # `test-example-`, or `test-bench-`. That execution is not a unit.
            # Another `test-` or `bench-` name is an unknown shape.
            # A missing fingerprint is an error.
            # A rustc call with Cargo metadata is a unit candidate.
            # The environment does not make the candidate decision.
            # When a candidate differs from a known Cargo layout, raise TraceError.
            # A missing environment value, a malformed id, or an unknown shape is that difference.
            # An operation outside those positions is not a unit.
            script_name = resolved.name.startswith(("build-script-", "build_script_"))
            if under_profile_build(self.target, resolved) and script_name:
                compile_fingerprint = build_unit_executable_fingerprint(self.target, resolved)
                marker = "absent" if compile_fingerprint is None else test_marker_kind(compile_fingerprint)
                # Another test or bench name is an unknown shape. It is not test execution.
                if marker == "unknown":
                    raise TraceError("Unknown build script unit layout")
                # An exact test or bench marker is test execution. It is not a build-script unit.
                if marker != "test":
                    if not package or not manifest or not out:
                        raise TraceError("Unknown build script unit layout")
                    out_dir = normalized_path(out, process.cwd)
                    # A nested Cargo target lives inside the outer run unit's OUT_DIR.
                    # The 1.100 build-script executable also sits in a compile unit `out`
                    # directory, so the executable path alone is not the nested check.
                    if inside_unit_out(self.target, out_dir):
                        if process.unit is None:
                            raise TraceError("A nested build has no outer unit")
                    else:
                        fingerprint = classic_build_script_fingerprint(
                            self.target, resolved, out_dir, package, requested_target)
                        if fingerprint is None:
                            fingerprint = build_unit_build_script_fingerprint(
                                self.target, resolved, out_dir, package, requested_target)
                        if fingerprint is None:
                            raise TraceError("Unknown build script unit layout")
                        if inside_unit_out(self.target, fingerprint):
                            if process.unit is None:
                                raise TraceError("A nested build has no outer unit")
                        elif not fingerprint.is_relative_to(self.target):
                            raise TraceError("Unknown build script unit layout")
                        else:
                            fingerprint_ready(fingerprint, run=True)
                            unit = fingerprint
                            kind = "run"
            # The rustup proxy is not the compiler file. The resolved file is the unit.
            if compiler_file == self.compiler and cargo_rustc_metadata(arguments):
                if not package or not manifest:
                    raise TraceError("Unknown rustc unit layout")
                destination = flag(arguments, "--out-dir")
                suffix = extra_filename(arguments)
                crate = flag(arguments, "--crate-name")
                hashed_suffix = bool(suffix and UNIT_ID.fullmatch(suffix))
                if not destination or not crate:
                    raise TraceError("Unknown rustc unit layout")
                directory = normalized_path(destination, process.cwd)
                if inside_unit_out(self.target, directory):
                    if process.unit is None:
                        raise TraceError("A nested build has no outer unit")
                else:
                    fingerprint = build_unit_fingerprint(directory)
                    if fingerprint is not None:
                        path_package = directory.parents[1].name
                        if path_package != package:
                            raise TraceError(
                                f"The target package {path_package} does not match CARGO_PKG_NAME {package}")
                        if not profile_inside_target(self.target, fingerprint.parents[3]):
                            raise TraceError("Unknown rustc unit layout")
                        relative = fingerprint.relative_to(self.target).as_posix() if fingerprint.is_relative_to(self.target) else None
                        # dx execs the rustup proxy. The proxy execs the real compiler in this process.
                        # Both execs are one compile. Keep the record from the first.
                        if relative is not None and process.unit == relative and process.unit_kind == "compile":
                            return
                        if relative is None or (suffix is not None and not hashed_suffix) or process.unit == relative:
                            raise TraceError("Unknown rustc unit layout")
                        fingerprint_ready(fingerprint, run=False)
                        unit = fingerprint
                        kind = "compile"
                        dep_name = f"{crate}-{suffix}.d" if hashed_suffix else f"{crate}.d"
                        dep_info = str(directory / dep_name)
                    elif hashed_suffix:
                        fingerprint = classic_fingerprint(self.target, directory, package, suffix)
                        if fingerprint is not None and inside_unit_out(self.target, fingerprint):
                            if process.unit is None:
                                raise TraceError("A nested build has no outer unit")
                        elif fingerprint is None or not fingerprint.is_relative_to(self.target):
                            raise TraceError("Unknown rustc unit layout")
                        else:
                            relative = fingerprint.relative_to(self.target).as_posix()
                            # The rustup proxy and the real compiler share this process.
                            if process.unit == relative and process.unit_kind == "compile":
                                return
                            fingerprint_ready(fingerprint, run=False)
                            unit = fingerprint
                            kind = "compile"
                            dep_info = str(directory / f"{crate}-{suffix}.d")
                    else:
                        raise TraceError("Unknown rustc unit layout")
            if unit is not None:
                process.unit = unit.relative_to(self.target).as_posix()
                process.unit_kind = kind
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
        self.reject_compiler_config(Path(os.path.normpath(opened)))
        if Path(opened).is_relative_to(self.target):
            return
        writing = access[1] in ("WRONLY", "RDWR")
        if process.unit is None and not writing:
            return
        self.emit({"event": "open", "unit": process.unit, "path": opened,
                   "write": writing, "read": access[1] != "WRONLY"})

    def is_cargo_config(self, path: Path) -> bool:
        """Return true for a Cargo config path the build can read.

        Cargo reads `.cargo/config.toml` and `.cargo/config` from the project parents.
        It also reads `config.toml` or `config` in `CARGO_HOME`.
        """
        if path.name not in ("config.toml", "config"):
            return False
        if path.parent.name == ".cargo":
            return True
        if self.cargo_home is None:
            return False
        if path.parent == self.cargo_home:
            return True
        try:
            return path.parent.resolve() == self.cargo_home.resolve()
        except OSError:
            return False

    def reject_compiler_config(self, path: Path) -> None:
        """Raise when a Cargo config selects another compiler or a wrapper."""
        if not self.is_cargo_config(path):
            return
        try:
            parsed = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
            raise TraceError("A Cargo config is unreadable.") from error
        build = parsed.get("build")
        if isinstance(build, dict) and (COMPILER_CONFIG_KEYS & set(build)):
            raise TraceError(UNSUPPORTED_COMPILER)

    def executable_in_target(self, raw: bytes) -> str | None:
        """Return the execve path relative to the target, or None.

        The raw line can hold credentials in its arguments and environment.
        Only an absolute executable inside the traced target is safe to keep.
        """
        if b"execve(" not in raw:
            return None
        try:
            executable, _ = string_at(raw.decode("utf-8", errors="surrogateescape"))
        except (TraceError, ValueError):
            return None
        path = Path(os.path.normpath(executable))
        if not path.is_absolute() or not path.is_relative_to(self.target):
            return None
        return path.relative_to(self.target).as_posix()

    def feed(self, stream) -> None:
        for raw in stream:
            try:
                self.consume(raw.decode("utf-8", errors="surrogateescape").rstrip("\n"))
            except (TraceError, ValueError, SyntaxError, IndexError, KeyError, OSError) as error:
                # A raw line can contain credentials. Record no argument and no environment value.
                # An executable under the traced target has no credential, so keep that relative path.
                self.errors += 1
                match = re.match(rb"\s*(\d+)(?:<[^>]*>)?\s+(\w+)\(", raw)
                event = {"event": "trace-error", "pid": int(match[1]) if match else None,
                         "operation": match[2].decode("ascii") if match else "resumed",
                         "unit": self.processes.get(int(match[1]), Process(str(self.workspace))).unit if match else None,
                         "reason": str(error) if isinstance(error, TraceError) else type(error).__name__}
                relative = self.executable_in_target(raw)
                if relative is not None:
                    event["path"] = relative
                self.emit(event)
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
    # A trace must not download a toolchain. dx probes nightly before it builds.
    env = os.environ.copy()
    env["RUSTUP_AUTO_INSTALL"] = "0"
    with journal.open("w") as output, os.fdopen(read_fd, "rb") as stream:
        parser = Parser(
            workspace, target, output, inspect_process=process_snapshot,
            compiler=supported_compiler(workspace),
            rustup=rustup_binary(),
            dx=pinned_dx(env, str(workspace)),
        )
        reader = threading.Thread(target=parser.feed, args=(stream,))
        reader.start()
        try:
            process = subprocess.Popen([
                "strace", "-f", "--seccomp-bpf", "-q", "-v", "-s", "1048576", "-yy", "-Y",
                "-e", "trace=execve,clone,clone3,fork,vfork,chdir,open,openat,openat2",
                "-o", f"/dev/fd/{write_fd}", "--", *command,
            ], cwd=workspace, env=env, pass_fds=(write_fd,))
        finally:
            os.close(write_fd)
        status = process.wait()
        reader.join()
    if parser.errors:
        print(f"Input trace rejected {parser.errors} operations. Read the error events in {journal}.", file=sys.stderr)
    return status if status else (1 if parser.errors else 0)
