"""Read Rust image and version pins from selected workflow YAML fields.

Read job container images and workflow or job environment values. Read
literal Rust version pins from RUST_VERSION, RUSTC_VERSION, RUST_TOOLCHAIN,
and RUSTUP_TOOLCHAIN. A Rust image spelled in a shell command is not checked.
"""
from __future__ import annotations

import re

from tools.workflow_pins import PinError, YamlMapping, _direct, _jobs, load_workflow


_RUST_IMAGE = re.compile(r"(?:[A-Za-z0-9_.:\[\]-]+/)*rust(?=$|[:@])")
_RUST_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
_RUST_TAG = re.compile(rf"({_RUST_VERSION.pattern})(?:-[A-Za-z0-9_.-]+)?")
_VERSION_VARIABLES = {"RUST_VERSION", "RUSTC_VERSION", "RUST_TOOLCHAIN", "RUSTUP_TOOLCHAIN"}
_EXPLICIT_PIN = "Use a Rust image pin in YAML under a job container or workflow or job env."


def image_version(reference: str) -> str | None:
    """Read the version of a literal Rust image reference."""
    image = _RUST_IMAGE.match(reference)
    if image is None:
        return None
    suffix = reference[image.end():]
    if "@" in suffix:
        raise PinError("Use the image resolver for Rust image digests. " + _EXPLICIT_PIN)
    tag = suffix.removeprefix(":")
    match = _RUST_TAG.fullmatch(tag)
    if match is None or len(tag) > 128:
        raise PinError(_EXPLICIT_PIN)
    return match[1]


def one_value(mapping: YamlMapping, key: str, default: object = None) -> object:
    values = _direct(mapping, key)
    if len(values) > 1:
        raise PinError(f"More than one workflow value for {key}.")
    return values[0] if values else default


def env_versions(mapping: YamlMapping) -> list[str]:
    declared = one_value(mapping, "env", YamlMapping([]))
    if not isinstance(declared, YamlMapping):
        raise PinError("A workflow environment must be a mapping.")
    names = set()
    versions = []
    for name, value in declared.pairs:
        if not isinstance(name, str) or name in names:
            raise PinError("A workflow environment name must be unique.")
        names.add(name)
        if name in _VERSION_VARIABLES:
            if not isinstance(value, str) or not _RUST_VERSION.fullmatch(value):
                raise PinError(f"Declare a Rust version in {name}.")
            versions.append(value)
        elif isinstance(value, str):
            version = image_version(value)
            if version is not None:
                versions.append(version)
    return versions


def workflow_rust_versions(text: str) -> list[str]:
    """Read Rust versions from job containers and workflow or job env."""
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError("A workflow must be a mapping.")
    jobs = _jobs(root)
    if len({name for name, _ in jobs}) != len(jobs):
        raise PinError("A workflow job name must be unique.")
    versions = env_versions(root)
    for _, job in jobs:
        versions.extend(env_versions(job))
        container = one_value(job, "container")
        if isinstance(container, YamlMapping):
            container = one_value(container, "image")
        if isinstance(container, str):
            version = image_version(container)
            if version is not None:
                versions.append(version)
    return versions
