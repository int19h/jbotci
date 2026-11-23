"""Read pinned keys from a GitHub workflow.

PyYAML parses the file. The Debian package `python3-yaml` provides that
parser. CI installs the package in each job that runs the trace. A local
machine uses the same package.
"""
from __future__ import annotations

import re

import yaml


class PinError(Exception):
    """A workflow pin is missing, repeated, or malformed."""


class YamlMapping:
    """A mapping that keeps every entry, including a duplicate key."""

    def __init__(self, pairs: list[tuple[object, object]]) -> None:
        self.pairs = pairs


class PinLoader(yaml.SafeLoader):
    """Load mappings without dropping a repeated key."""


def _construct_mapping(loader: PinLoader, node: yaml.Node) -> YamlMapping:
    loader.flatten_mapping(node)
    pairs: list[tuple[object, object]] = []
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=True)
        value = loader.construct_object(value_node, deep=True)
        pairs.append((key, value))
    return YamlMapping(pairs)


PinLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
    _construct_mapping,
)


def load_workflow(text: str) -> object:
    """Return the parsed workflow.

    A syntax error is a pin error.
    """
    try:
        return yaml.load(text, Loader=PinLoader)
    except yaml.YAMLError as error:
        raise PinError("The workflow pin is unavailable.") from error


def _walk(node: object, key: str, found: list[str]) -> None:
    if isinstance(node, YamlMapping):
        for name, value in node.pairs:
            if name == key:
                if not isinstance(value, str):
                    raise PinError("The workflow pin is unavailable.")
                found.append(value)
            _walk(value, key, found)
    elif isinstance(node, list):
        for item in node:
            _walk(item, key, found)


def mapping_values(text: str, key: str) -> list[str]:
    """Return every mapping value of `key`.

    A block entry, a flow entry, and a quoted key all count.
    An inline comment is not part of the value.
    A use of the name in a scalar is not a declaration.
    """
    found: list[str] = []
    _walk(load_workflow(text), key, found)
    return found


def exactly_one(text: str, key: str, pattern: re.Pattern[str]) -> str:
    """Return the one well-formed value of `key`.

    A missing declaration, a second declaration, or a malformed value is an error.
    """
    found = mapping_values(text, key)
    if len(found) != 1 or pattern.fullmatch(found[0]) is None:
        raise PinError("The workflow pin is unavailable.")
    return found[0]


def _jobs(text: str) -> list[tuple[str, YamlMapping]]:
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError("The workflow pin is unavailable.")
    jobs: list[tuple[str, YamlMapping]] = []
    for name, value in root.pairs:
        if name != "jobs":
            continue
        if not isinstance(value, YamlMapping):
            raise PinError("The workflow pin is unavailable.")
        for job_name, job in value.pairs:
            if not isinstance(job_name, str) or not isinstance(job, YamlMapping):
                raise PinError("The workflow pin is unavailable.")
            jobs.append((job_name, job))
    return jobs


def _direct(node: YamlMapping, key: str) -> list[object]:
    return [value for name, value in node.pairs if name == key]


def _string_values(node: object, key: str) -> list[str]:
    if not isinstance(node, YamlMapping):
        raise PinError("The workflow pin is unavailable.")
    found: list[str] = []
    for value in _direct(node, key):
        if not isinstance(value, str):
            raise PinError("The workflow pin is unavailable.")
        found.append(value)
    return found


def job_env_values(text: str, key: str) -> list[tuple[str, str]]:
    """Return `(job, value)` for each job-level `env` declaration of `key`."""
    found: list[tuple[str, str]] = []
    for job_name, job in _jobs(text):
        for env in _direct(job, "env"):
            if not isinstance(env, YamlMapping):
                raise PinError("The workflow pin is unavailable.")
            for value in _string_values(env, key):
                found.append((job_name, value))
    return found


def step_env_values(text: str, key: str) -> list[tuple[str, str]]:
    """Return `(job, value)` for each step-level `env` declaration of `key`."""
    found: list[tuple[str, str]] = []
    for job_name, job in _jobs(text):
        for steps in _direct(job, "steps"):
            if not isinstance(steps, list):
                raise PinError("The workflow pin is unavailable.")
            for step in steps:
                if not isinstance(step, YamlMapping):
                    raise PinError("The workflow pin is unavailable.")
                for env in _direct(step, "env"):
                    if not isinstance(env, YamlMapping):
                        raise PinError("The workflow pin is unavailable.")
                    for value in _string_values(env, key):
                        found.append((job_name, value))
    return found


def _strings(node: object) -> list[str]:
    found: list[str] = []

    def walk(current: object) -> None:
        if isinstance(current, str):
            found.append(current)
        elif isinstance(current, YamlMapping):
            for _name, value in current.pairs:
                walk(value)
        elif isinstance(current, list):
            for item in current:
                walk(item)

    walk(node)
    return found


def _unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def _assignments(pieces: list[str], key: str) -> list[str]:
    pattern = re.compile(rf"(?:^|(?<=\s))(?:export\s+)?{re.escape(key)}=(\S+)")
    found: list[str] = []
    for piece in pieces:
        for match in pattern.finditer(piece):
            found.append(_unquote(match.group(1)))
    return found


def shell_assignments(text: str, key: str) -> list[str]:
    """Return shell assignments of `key` in workflow scalars."""
    return _assignments(_strings(load_workflow(text)), key)


def job_shell_assignments(text: str, job_name: str, key: str) -> list[str]:
    """Return shell assignments of `key` under one job."""
    found: list[str] = []
    for name, job in _jobs(text):
        if name == job_name:
            found.extend(_assignments(_strings(job), key))
    return found


def traced_target_values(text: str) -> list[str]:
    """Return each `CARGO_TARGET_DIR` declaration on a traced step.

    A traced step runs `tools.cargo_inputs trace`.
    A flow mapping and a quoted key count.
    A shell assignment in that step counts.
    """
    found: list[str] = []
    for _name, job in _jobs(text):
        for steps in _direct(job, "steps"):
            if not isinstance(steps, list):
                raise PinError("The workflow pin is unavailable.")
            for step in steps:
                if not isinstance(step, YamlMapping):
                    raise PinError("The workflow pin is unavailable.")
                runs = [value for value in _direct(step, "run") if isinstance(value, str)]
                if not any("tools.cargo_inputs trace" in run for run in runs):
                    continue
                for env in _direct(step, "env"):
                    if not isinstance(env, YamlMapping):
                        raise PinError("The workflow pin is unavailable.")
                    found.extend(_string_values(env, "CARGO_TARGET_DIR"))
                found.extend(_assignments(runs, "CARGO_TARGET_DIR"))
    return found


_IMAGE_TOKEN = re.compile(r"(?<![A-Za-z0-9._/-])rust:[^\s\"'`]+")
_IMAGE_TAG = re.compile(r"rust:([0-9]+\.[0-9]+\.[0-9]+)-[A-Za-z0-9.]+")


def rust_image_versions(text: str) -> list[str]:
    """Return the version from each `rust:` image tag in `text`.

    A tag must be `rust:<version>-<name>`.
    """
    versions: list[str] = []
    for token in _IMAGE_TOKEN.findall(text):
        match = _IMAGE_TAG.fullmatch(token)
        if match is None:
            raise PinError("The workflow pin is unavailable.")
        versions.append(match.group(1))
    return versions


def workflow_image_versions(text: str) -> list[str]:
    """Return the version from each Rust image tag in a workflow."""
    versions: list[str] = []
    for piece in _strings(load_workflow(text)):
        versions.extend(rust_image_versions(piece))
    return versions
