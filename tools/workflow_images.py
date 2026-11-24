"""Read image pins from workflow declarations and shell commands."""
from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import PurePosixPath
import re
import shlex
import subprocess
from typing import Iterator

from tools.workflow_pins import PinError, YamlMapping, _direct, _jobs, load_workflow


_EXPRESSION = re.compile(r"\$\{\{.*?\}\}", re.DOTALL)
_IMAGE_REF = re.compile(r"[A-Za-z0-9\[][A-Za-z0-9_.:/@\[\]-]*")
_RUST_TAG = re.compile(r"([0-9]+\.[0-9]+\.[0-9]+)(?:-[A-Za-z0-9_.-]+)?")
_PROJECT_IMAGE = re.compile(r"ghcr\.io/\$\{\{\s*github\.repository\s*\}\}:sha-\$\{\{\s*github\.sha\s*\}\}")
_NEEDS_IMAGE = re.compile(r"\$\{\{\s*needs\.([\w-]+)\.outputs\.([\w-]+)\s*\}\}")
_STEP_IMAGE = re.compile(r"\$\{\{\s*steps\.([\w-]+)\.outputs\.image\s*\}\}")
_RESOLVER = ["python3", "-m", "tools.cargo_cache_identity", "resolve-image"]
_EXPRESSION_PARAMETER = "__WORKFLOW_EXPRESSION__"
_EXPLICIT_PIN = "Use an explicit image pin in the workflow."


def image_version(reference: str) -> str | None:
    """Return the Rust version from one literal image reference."""
    if _PROJECT_IMAGE.fullmatch(reference):
        return None
    reference = reference.removeprefix("docker://")
    if not _IMAGE_REF.fullmatch(reference):
        raise PinError(_EXPLICIT_PIN)
    leaf = reference.rsplit("/", 1)[-1]
    tagged, separator, digest = leaf.partition("@")
    repository, _, tag = tagged.partition(":")
    if repository != "rust":
        return None
    match = _RUST_TAG.fullmatch(tag)
    if match is None or len(tag) > 128 or (separator and not re.fullmatch(r"sha256:[0-9a-f]{64}", digest)):
        raise PinError(_EXPLICIT_PIN)
    return match[1]


def one_value(mapping: YamlMapping, key: str, default=None):
    values = _direct(mapping, key)
    if len(values) > 1:
        raise PinError(f"More than one workflow value for {key}.")
    return values[0] if values else default


def inherited_env(mapping: YamlMapping, parent: dict[str, object]) -> dict[str, object]:
    declared = one_value(mapping, "env", YamlMapping([]))
    if not isinstance(declared, YamlMapping):
        raise PinError(_EXPLICIT_PIN)
    environment: dict[str, object] = {}
    for name, value in declared.pairs:
        if not isinstance(name, str) or name in environment:
            raise PinError("A workflow environment name must be unique.")
        environment[name] = value
    return parent | environment


def mappings(items: object) -> Iterator[YamlMapping]:
    if not isinstance(items, list) or not all(isinstance(item, YamlMapping) for item in items):
        raise PinError("A workflow step must be a mapping.")
    yield from items


def shell_nodes(node: object) -> Iterator[dict]:
    if isinstance(node, dict):
        yield node
        for child in node.values():
            yield from shell_nodes(child)
    elif isinstance(node, list):
        for child in node:
            yield from shell_nodes(child)


def shell_parts(parts: list[dict], quoted: bool = False) -> str | None:
    """Join literal shell word parts without executing expansions."""
    pieces = []
    for part in parts:
        match part["Type"]:
            case "Lit":
                value = part["Value"]
                if not quoted and re.search(r"(?<!\\)[*?\[{}~]", value):
                    return None
                escape = r"\\([$\x60\"\\\n])" if quoted else r"\\(.)"
                pieces.append(re.sub(escape, lambda match: match[1].removesuffix("\n"), value, flags=re.DOTALL))
            case "SglQuoted" if not part.get("Dollar"):
                pieces.append(part["Value"])
            case "DblQuoted" if not part.get("Dollar"):
                value = shell_parts(part.get("Parts", []), quoted=True)
                if value is None:
                    return None
                pieces.append(value)
            case _:
                return None
    return "".join(pieces)


def env_name(word: dict) -> str | None:
    parts = word["Parts"]
    if len(parts) == 1 and parts[0]["Type"] == "DblQuoted":
        parts = parts[0].get("Parts", [])
    if len(parts) != 1:
        return None
    part = parts[0]
    if part["Type"] != "ParamExp":
        return None
    if part.keys() - {"Type", "Param", "Short", "Pos", "End", "Dollar", "Rbrace"}:
        return None
    return part["Param"]["Value"]


def parse_shell(source: str) -> Iterator[dict]:
    # A GitHub expression is a dynamic value. Its syntax is outside Bash.
    source = _EXPRESSION.sub("${" + _EXPRESSION_PARAMETER + "}", source)
    try:
        result = subprocess.run(
            ["shfmt", "-ln=bash", "--to-json"], input=source, capture_output=True, text=True, check=False,
        )
    except FileNotFoundError as error:
        raise PinError("Install shfmt to read workflow shell commands.") from error
    if result.returncode:
        raise PinError(f"Cannot read a Bash workflow step. {_EXPLICIT_PIN} {result.stderr.strip()}")
    yield from shell_nodes(json.loads(result.stdout))


_DOCKER_FLAGS = {"--rm", "--read-only", "--detach", "--interactive", "--tty", "-d", "-i", "-t", "-it"}
_DOCKER_VALUES = {
    "--platform", "--env", "--env-file", "--volume", "--mount", "--workdir", "--network",
    "--publish", "--name", "--entrypoint", "--user", "-e", "-v", "-w", "-p", "-u",
}


def docker_image_index(words: list[str | None]) -> int | None:
    if not words or words[0] is None or PurePosixPath(words[0]).name != "docker":
        return None
    index = 1
    if words[index:index + 1] in (["image"], ["container"]):
        index += 1
    if index >= len(words) or words[index] is None or words[index].startswith("-"):
        raise PinError(_EXPLICIT_PIN)
    if words[index] not in ("pull", "run", "create"):
        return None
    index += 1
    while index < len(words):
        option = words[index]
        if option == "--":
            return index + 1
        if option is None or not option.startswith("-"):
            return index
        name, equals, _ = option.partition("=")
        if name in _DOCKER_VALUES:
            index += 1 if equals else 2
        elif name in _DOCKER_FLAGS:
            index += 1
        else:
            raise PinError(f"Unsupported Docker option {name}. {_EXPLICIT_PIN}")
    raise PinError(_EXPLICIT_PIN)


@dataclass(frozen=True)
class CommandImage:
    reference: str
    resolves_digest: bool = False


def shell_image_references(source: str, shell: str, env: dict[str, object]) -> Iterator[CommandImage]:
    if shell not in ("bash", "sh"):
        if re.search(r"\b(?:docker\b|resolve-image\b|rust:)", source):
            raise PinError(f"Use a Bash step for image commands. {_EXPLICIT_PIN}")
        return
    nodes = list(parse_shell(source))
    assigned = {
        node["Name"]["Value"] for node in nodes
        if isinstance(node.get("Name"), dict) and "Value" in node["Name"] and "Value" in node
    }
    env = {name: value for name, value in env.items() if name not in assigned}
    for node in nodes:
        if node.get("Type") != "CallExpr" or not node.get("Args"):
            continue
        args = node["Args"]
        words = [shell_parts(word["Parts"]) for word in args]
        while words and words[0] in ("command", "exec", "sudo"):
            words = words[1:]
            args = args[1:]
        if not words:
            continue
        if words[0] in ("bash", "sh"):
            for index, option in enumerate(words[1:], start=1):
                if option is None or not option.startswith("-"):
                    break
                if re.fullmatch(r"-[a-zA-Z]*c[a-zA-Z]*", option):
                    if index + 1 >= len(words) or words[index + 1] is None:
                        raise PinError(_EXPLICIT_PIN)
                    yield from shell_image_references(words[index + 1], words[0], env)
                    break
            continue
        resolves_digest = words[:4] == _RESOLVER
        if resolves_digest:
            index = len(_RESOLVER)
        elif words[0] == "resolve-image":
            index = 1
        else:
            index = docker_image_index(words)
        if index is None:
            continue
        if index >= len(words):
            raise PinError(_EXPLICIT_PIN)
        reference = words[index]
        if reference is None:
            name = env_name(args[index])
            if name and name != _EXPRESSION_PARAMETER:
                reference = env.get(name)
        if not isinstance(reference, str):
            raise PinError(_EXPLICIT_PIN)
        yield CommandImage(reference, resolves_digest)


def resolver_outputs(jobs: list[tuple[str, YamlMapping]]) -> set[tuple[str, str]]:
    """Find image outputs backed by a literal image resolver command."""
    resolved = set()
    for name, job in jobs:
        outputs = one_value(job, "outputs")
        if not isinstance(outputs, YamlMapping):
            continue
        for key, expression in outputs.pairs:
            match = _STEP_IMAGE.fullmatch(expression) if isinstance(expression, str) else None
            if match is None:
                continue
            for step in mappings(one_value(job, "steps", [])):
                if one_value(step, "id") != match[1]:
                    continue
                command = one_value(step, "run", "")
                calls = list(shell_image_references(command, one_value(step, "shell", "bash"), {}))
                if len(calls) == 1 and calls[0].resolves_digest and image_version(calls[0].reference) is not None:
                    resolved.add((name, key))
    return resolved


def container_image(value: object) -> str:
    if isinstance(value, YamlMapping):
        value = one_value(value, "image")
    if not isinstance(value, str):
        raise PinError(_EXPLICIT_PIN)
    return value


def workflow_image_versions(text: str) -> list[str]:
    """Read pins from containers, services, container actions, and image commands."""
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError(_EXPLICIT_PIN)
    jobs = _jobs(root)
    workflow_env = inherited_env(root, {})
    versions = []
    resolved = resolver_outputs(jobs)

    def add(reference: str) -> None:
        match = _NEEDS_IMAGE.fullmatch(reference)
        if match and (match[1], match[2]) in resolved:
            return
        version = image_version(reference)
        if version is not None:
            versions.append(version)

    for _, job in jobs:
        job_env = inherited_env(job, workflow_env)
        for container in _direct(job, "container"):
            add(container_image(container))
        services = one_value(job, "services", YamlMapping([]))
        if not isinstance(services, YamlMapping):
            raise PinError(_EXPLICIT_PIN)
        for _, service in services.pairs:
            add(container_image(service))
        for step in mappings(one_value(job, "steps", [])):
            action = one_value(step, "uses", "")
            if isinstance(action, str) and action.startswith("docker://"):
                add(action)
            command = one_value(step, "run")
            if command is None:
                continue
            env = inherited_env(step, job_env)
            shell = one_value(step, "shell", "bash")
            if not isinstance(command, str) or not isinstance(shell, str):
                raise PinError(_EXPLICIT_PIN)
            for image in shell_image_references(command, shell, env):
                add(image.reference)
    return versions


def dockerfile_image_versions(text: str) -> list[str]:
    """Read literal Dockerfile base image pins."""
    versions = []
    for line in text.replace("\\\n", "").splitlines():
        fields = line.split(maxsplit=1)
        if not fields or fields[0].upper() != "FROM":
            continue
        words = shlex.split(fields[1]) if len(fields) == 2 else []
        if words and words[0].startswith("--platform="):
            words.pop(0)
        if not words:
            raise PinError(_EXPLICIT_PIN)
        version = image_version(words[0])
        if version is not None:
            versions.append(version)
    return versions
