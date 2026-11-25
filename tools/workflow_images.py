"""Check that workflow Rust images use explicit YAML pins.

Split the executable shell text of each step into words. If a word names
`docker`, reject every Rust image word in that step. This includes `rust`,
tagged or digested references, and registry paths that end in a Rust image.
Step names and shell comments do not enter the word scan. A Docker container
named `rust` also fails. The existing linked Render resolver keeps its
exception.
"""
from __future__ import annotations

import re
import shlex

from tools.workflow_pins import PinError, YamlMapping, _direct, _jobs, load_workflow


_IMAGE_REF = re.compile(r"[A-Za-z0-9\[][A-Za-z0-9_.:/@\[\]-]*")
_RUST_TAG = re.compile(r"([0-9]+\.[0-9]+\.[0-9]+)(?:-[A-Za-z0-9_.-]+)?")
_RUST_LITERAL = re.compile(r"(?<![\w.:-])rust(?=:(?!:)[^\s]|@)")
_RUST_IMAGE_WORD = re.compile(r"(?:[A-Za-z0-9_.:\[\]-]+/)*rust(?:[:@]\S*)?")
_QUOTED_SUBSTITUTION = re.compile(r"\$\(([^()]*)\)|`([^`]*)`")
_SHELL_QUOTING = str.maketrans("", "", "'\"\\")
_NEEDS_IMAGE = re.compile(r"\$\{\{\s*needs\.([\w-]+)\.outputs\.([\w-]+)\s*\}\}")
_STEP_IMAGE = re.compile(r"\$\{\{\s*steps\.([\w-]+)\.outputs\.image\s*\}\}")
_RESOLVER = "python3 -m tools.cargo_cache_identity resolve-image "
_RESOLVER_ENV = re.compile(r'"\$([A-Za-z_]\w*)"')
_EXPLICIT_PIN = "Use a Rust image pin in YAML under a job container or workflow or job env."


def image_version(reference: str) -> str | None:
    """Read a Rust version from one image tag."""
    if not _IMAGE_REF.fullmatch(reference):
        raise PinError(_EXPLICIT_PIN)
    tagged, separator, _ = reference.rsplit("/", 1)[-1].partition("@")
    repository, _, tag = tagged.partition(":")
    if repository != "rust":
        return None
    if separator:
        raise PinError("Use the image resolver for Rust image digests. " + _EXPLICIT_PIN)
    match = _RUST_TAG.fullmatch(tag)
    if match is None or len(tag) > 128:
        raise PinError(_EXPLICIT_PIN)
    return match[1]


def one_value(mapping: YamlMapping, key: str, default=None):
    values = _direct(mapping, key)
    if len(values) > 1:
        raise PinError(f"More than one workflow value for {key}.")
    return values[0] if values else default


def environment(mapping: YamlMapping) -> dict[str, object]:
    declared = one_value(mapping, "env", YamlMapping([]))
    if not isinstance(declared, YamlMapping):
        raise PinError("A workflow environment must be a mapping.")
    values: dict[str, object] = {}
    for name, value in declared.pairs:
        if not isinstance(name, str) or name in values:
            raise PinError("A workflow environment name must be unique.")
        values[name] = value
    return values


def has_rust_image(text: str) -> bool:
    return bool(_RUST_LITERAL.search(text) or text.rsplit("/", 1)[-1] == "rust")


def require_yaml_pin(value: object) -> None:
    """Reject Rust image spellings outside the permitted YAML declarations."""
    if isinstance(value, YamlMapping):
        for _, child in value.pairs:
            require_yaml_pin(child)
    elif isinstance(value, list):
        for child in value:
            require_yaml_pin(child)
    elif isinstance(value, str):
        if has_rust_image(value):
            raise PinError(_EXPLICIT_PIN)


def shell_words(text: str) -> list[str]:
    lexer = shlex.shlex(text, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    return list(lexer)


def require_shell_pin(command: str) -> None:
    """Apply the step-wide literal image rule without reading Docker options."""
    if "docker" not in command.translate(_SHELL_QUOTING).replace("\n", ""):
        return
    try:
        words = shell_words(command)
        # shlex keeps a command substitution inside a quoted word together.
        substitutions = [
            match[1] if match[1] is not None else match[2]
            for word in words for match in _QUOTED_SUBSTITUTION.finditer(word)
        ]
        for substitution in substitutions:
            words.extend(shell_words(substitution))
    except ValueError as error:
        raise PinError("Cannot split words in a Docker step. " + _EXPLICIT_PIN) from error
    # shlex retains the newline from an escaped line break inside a word.
    words = [word.replace("\n", "") for word in words]
    if any(word.rsplit("/", 1)[-1] == "docker" for word in words) and any(
        _RUST_IMAGE_WORD.fullmatch(word) for word in words
    ):
        raise PinError(_EXPLICIT_PIN)


def container_image(job: YamlMapping) -> str | None:
    value = one_value(job, "container")
    if isinstance(value, YamlMapping):
        for key, child in value.pairs:
            if key != "image":
                require_yaml_pin(child)
        value = one_value(value, "image")
        if value is None:
            raise PinError(_EXPLICIT_PIN)
    if value is not None and not isinstance(value, str):
        raise PinError(_EXPLICIT_PIN)
    return value


def resolver_step(outputs: YamlMapping, key: str, steps: list[object]) -> tuple[int, YamlMapping]:
    expression = one_value(outputs, key)
    match = _STEP_IMAGE.fullmatch(expression) if isinstance(expression, str) else None
    if match is not None:
        resolved = [
            (index, step) for index, step in enumerate(steps)
            if isinstance(step, YamlMapping) and one_value(step, "id") == match[1]
        ]
        if len(resolved) == 1:
            return resolved[0]
    raise PinError(_EXPLICIT_PIN)


def workflow_image_versions(text: str, *, unchanged_render_resolver: bool = False) -> list[str]:
    """Read Rust pins and check the source of each digest from a job output."""
    root = load_workflow(text)
    if not isinstance(root, YamlMapping):
        raise PinError(_EXPLICIT_PIN)
    job_entries = _jobs(root)
    jobs = dict(job_entries)
    if len(jobs) != len(job_entries):
        raise PinError("A workflow job name must be unique.")
    root_env = environment(root)
    job_envs = {name: environment(job) for name, job in jobs.items()}
    steps = {name: one_value(job, "steps", []) for name, job in jobs.items()}
    if any(not isinstance(value, list) for value in steps.values()):
        raise PinError("Workflow steps must be a list.")

    versions = []
    for values in (root_env, *job_envs.values()):
        for value in values.values():
            if isinstance(value, str) and has_rust_image(value):
                version = image_version(value)
                if version is None:
                    raise PinError(_EXPLICIT_PIN)
                versions.append(version)
            else:
                require_yaml_pin(value)

    legacy_steps: set[tuple[str, int]] = set()
    for job in jobs.values():
        reference = container_image(job)
        if reference is None:
            continue
        source = _NEEDS_IMAGE.fullmatch(reference)
        if source is None:
            version = image_version(reference)
            if version is not None:
                versions.append(version)
            continue
        name, output = source.groups()
        needs = one_value(job, "needs")
        if name not in jobs or not (needs == name or isinstance(needs, list) and name in needs):
            raise PinError("The image output must come from a required resolver job. " + _EXPLICIT_PIN)
        outputs = one_value(jobs[name], "outputs")
        if not isinstance(outputs, YamlMapping):
            raise PinError(_EXPLICIT_PIN)
        index, step = resolver_step(outputs, output, steps[name])
        command = one_value(step, "run")
        if not isinstance(command, str) or not command.strip().startswith(_RESOLVER):
            raise PinError(_EXPLICIT_PIN)
        argument = command.strip().removeprefix(_RESOLVER)
        variable = _RESOLVER_ENV.fullmatch(argument)
        legacy = unchanged_render_resolver and (name, output, one_value(step, "id")) == ("build-image", "image", "image")
        if variable is not None:
            key = variable[1]
            if key in environment(step):
                raise PinError(_EXPLICIT_PIN)
            tag = job_envs[name].get(key, root_env.get(key))
        elif legacy:
            # Render still uses the resolver invocation from main.
            tag = argument
        else:
            raise PinError(_EXPLICIT_PIN)
        if not isinstance(tag, str) or not tag.startswith("rust:") or "/" in tag:
            raise PinError(_EXPLICIT_PIN)
        version = image_version(tag)
        if variable is None and (name, index) not in legacy_steps:
            legacy_steps.add((name, index))
            versions.append(version)

    for key, value in root.pairs:
        if key not in ("jobs", "env", "name", "run-name"):
            require_yaml_pin(value)
    for name, job in jobs.items():
        for key, value in job.pairs:
            if key not in ("container", "steps", "env", "name"):
                require_yaml_pin(value)
        for index, step in enumerate(steps[name]):
            if not isinstance(step, YamlMapping):
                raise PinError("A workflow step must be a mapping.")
            command = one_value(step, "run")
            if isinstance(command, str) and (name, index) not in legacy_steps:
                require_shell_pin(command)
            for key, child in step.pairs:
                if key not in ("name", "run"):
                    require_yaml_pin(child)
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
