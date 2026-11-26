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


def jobs(root: object) -> list[tuple[str, YamlMapping]]:
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


def direct(node: YamlMapping, key: str) -> list[object]:
    return [value for name, value in node.pairs if name == key]


def _string_values(node: object, key: str) -> list[str]:
    if not isinstance(node, YamlMapping):
        raise PinError("The workflow pin is unavailable.")
    found: list[str] = []
    for value in direct(node, key):
        if not isinstance(value, str):
            raise PinError("The workflow pin is unavailable.")
        found.append(value)
    return found


def job_env_values(text: str, key: str) -> list[tuple[str, str]]:
    """Return `(job, value)` for each job-level `env` declaration of `key`."""
    found: list[tuple[str, str]] = []
    for job_name, job in jobs(load_workflow(text)):
        for env in direct(job, "env"):
            if not isinstance(env, YamlMapping):
                raise PinError("The workflow pin is unavailable.")
            for value in _string_values(env, key):
                found.append((job_name, value))
    return found
