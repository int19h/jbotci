"""Read Rust image and version pins from selected build files.

Read job container images and workflow or job environment values.
Read literal Rust version pins from RUST_VERSION, RUSTC_VERSION,
RUST_TOOLCHAIN, and RUSTUP_TOOLCHAIN.
Read FROM lines in Dockerfiles that the build configuration names.
`render.yaml` names one Dockerfile through `dockerfilePath`.
A workflow names a Dockerfile through the `file` input of `docker/build-push-action`.
A Rust image spelled in a shell command is not checked.
The repository name is the last path component before the tag or the digest.
"""
from __future__ import annotations

import re
from pathlib import Path

from tools.workflow_pins import (
    PinError, YamlMapping, direct, jobs, load_workflow, mapping_values,
)


_RUST_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
_RUST_TAG = re.compile(rf"({_RUST_VERSION.pattern})(?:-[A-Za-z0-9_.-]+)?")
_VERSION_VARIABLES = {"RUST_VERSION", "RUSTC_VERSION", "RUST_TOOLCHAIN", "RUSTUP_TOOLCHAIN"}
_BUILD_PUSH = "docker/build-push-action@"


def _image_parts(reference: str) -> tuple[str, str | None, bool]:
    """Return the repository, the tag, and whether a digest is present.

    A colon before a slash is a registry port, not a tag.
    """
    name, separator, _digest = reference.partition("@")
    last = name[name.rfind("/") + 1:]
    colon = last.find(":")
    if colon < 0:
        return last, None, separator == "@"
    return last[:colon], last[colon + 1:], separator == "@"


def image_version(reference: str, place: str) -> str | None:
    """Return the Rust version from one image reference.

    Return None when the repository name is not `rust`.
    """
    repository, tag, has_digest = _image_parts(reference)
    if repository != "rust":
        return None
    if has_digest:
        raise PinError(
            f"{place} value {reference!r} uses a digest. "
            "Use the image resolver for Rust image digests."
        )
    match = None if tag is None else _RUST_TAG.fullmatch(tag)
    if tag is None or match is None or len(tag) > 128:
        raise PinError(
            f"{place} value {reference!r} is not a full Rust version. "
            "The tag must start with a full Rust version."
        )
    return match[1]


def one_value(mapping: YamlMapping, key: str, default: object = None) -> object:
    values = direct(mapping, key)
    if len(values) > 1:
        raise PinError(f"More than one workflow value for {key}.")
    return values[0] if values else default


def env_versions(mapping: YamlMapping, place: str) -> list[str]:
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
                shown = value if isinstance(value, str) else type(value).__name__
                raise PinError(
                    f"Declare a Rust version in {name}. "
                    f"The value {shown!r} is not a full Rust version."
                )
            versions.append(value)
        elif isinstance(value, str):
            version = image_version(value, place=f"{place} env {name}")
            if version is not None:
                versions.append(version)
    return versions


def workflow_rust_versions(text: str) -> list[str]:
    """Read Rust versions from job containers and workflow or job env."""
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError("A workflow must be a mapping.")
    found = jobs(root)
    if len({name for name, _ in found}) != len(found):
        raise PinError("A workflow job name must be unique.")
    versions = env_versions(root, "workflow")
    for job_name, job in found:
        versions.extend(env_versions(job, f"job {job_name}"))
        container = one_value(job, "container")
        place = f"job {job_name} container"
        if isinstance(container, YamlMapping):
            container = one_value(container, "image")
            place = f"job {job_name} container image"
        if isinstance(container, str):
            version = image_version(container, place=place)
            if version is not None:
                versions.append(version)
    return versions


def from_images(text: str) -> list[str]:
    """Return each image token on a Dockerfile FROM line.

    A `#` comment is not part of the line.
    A `--platform=...` flag is not the image.
    """
    images = []
    for line in text.splitlines():
        parts = line.split("#", 1)[0].split()
        if not parts or parts[0].upper() != "FROM":
            continue
        index = 1
        while index < len(parts) and parts[index].startswith("--"):
            index += 1
        if index >= len(parts):
            raise PinError("A Dockerfile FROM line has no image.")
        images.append(parts[index])
    return images


def dockerfile_rust_versions(text: str, source: str) -> list[str]:
    """Return Rust versions from FROM lines in one Dockerfile."""
    versions = []
    for image in from_images(text):
        version = image_version(image, place=f"{source} FROM")
        if version is not None:
            versions.append(version)
    return versions


def require_channel(versions: list[str], channel: str, source: str) -> None:
    """Reject a Rust version that differs from the toolchain channel."""
    for version in versions:
        if version != channel:
            raise PinError(f"{source} pins Rust {version}. The channel is {channel}.")


def _repo_file(repo: Path, value: str, place: str) -> Path:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        raise PinError(f"{place} path {value!r} leaves the repository.")
    return repo / path


def render_dockerfiles(repo: Path) -> list[Path]:
    """Return Dockerfiles named by `dockerfilePath` in `render.yaml`."""
    manifest = repo / "render.yaml"
    if not manifest.is_file():
        return []
    return [
        _repo_file(repo, value, "render.yaml dockerfilePath")
        for value in mapping_values(manifest.read_text(encoding="utf-8"), "dockerfilePath")
    ]


def workflow_dockerfiles(repo: Path, text: str) -> list[Path]:
    """Return Dockerfiles named by a `docker/build-push-action` `file` input."""
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError("A workflow must be a mapping.")
    found: list[Path] = []
    for job_name, job in jobs(root):
        for steps in direct(job, "steps"):
            if not isinstance(steps, list):
                raise PinError(f"Job {job_name} steps must be a list.")
            for step in steps:
                if not isinstance(step, YamlMapping):
                    continue
                uses = direct(step, "uses")
                if len(uses) != 1 or not isinstance(uses[0], str) or not uses[0].startswith(_BUILD_PUSH):
                    continue
                with_values = direct(step, "with")
                if len(with_values) != 1 or not isinstance(with_values[0], YamlMapping):
                    continue
                files = direct(with_values[0], "file")
                if not files:
                    continue
                if len(files) != 1 or not isinstance(files[0], str):
                    raise PinError(f"Job {job_name} build action file input must be one path.")
                found.append(_repo_file(repo, files[0], f"job {job_name} file"))
    return found


def named_dockerfiles(repo: Path) -> list[Path]:
    """Return every Dockerfile that the repository build configuration names."""
    found = render_dockerfiles(repo)
    workflows = repo / ".github" / "workflows"
    if workflows.is_dir():
        for path in sorted(workflows.iterdir()):
            if path.is_file() and path.suffix in (".yml", ".yaml"):
                found.extend(workflow_dockerfiles(repo, path.read_text(encoding="utf-8")))
    return found
