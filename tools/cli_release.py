#!/usr/bin/env python3
"""Build and validate deterministic jbotci CLI release archives."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import stat
import sys
import tarfile
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Mapping, Sequence


DOCUMENT_NAMES = ("LICENSE.md", "README.md", "THIRD-PARTY-NOTICES.md")
METADATA_NAME = "metadata.json"
CHECKSUMS_NAME = "SHA256SUMS"
REPRODUCIBLE_TIMESTAMP = 0
ZIP_TIMESTAMP = (1980, 1, 1, 0, 0, 0)
SHA_RE = re.compile(r"[0-9a-f]{40}")
VERSION_RE = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+-]*")


class ReleaseToolError(Exception):
    """An input cannot safely produce or validate a release artifact."""


@dataclass(frozen=True)
class TargetSpec:
    target: str
    binary_name: str
    archive_suffix: str
    executable_format: str
    machine: int

    def archive_name(self, version: str) -> str:
        return f"jbotci-{version}-{self.target}{self.archive_suffix}"


TARGETS = (
    TargetSpec(
        target="x86_64-unknown-linux-musl",
        binary_name="jbotci",
        archive_suffix=".tar.gz",
        executable_format="elf",
        machine=62,
    ),
    TargetSpec(
        target="aarch64-unknown-linux-musl",
        binary_name="jbotci",
        archive_suffix=".tar.gz",
        executable_format="elf",
        machine=183,
    ),
    TargetSpec(
        target="x86_64-apple-darwin",
        binary_name="jbotci",
        archive_suffix=".tar.gz",
        executable_format="macho",
        machine=0x01000007,
    ),
    TargetSpec(
        target="aarch64-apple-darwin",
        binary_name="jbotci",
        archive_suffix=".tar.gz",
        executable_format="macho",
        machine=0x0100000C,
    ),
    TargetSpec(
        target="x86_64-pc-windows-msvc",
        binary_name="jbotci.exe",
        archive_suffix=".zip",
        executable_format="pe",
        machine=0x8664,
    ),
)
TARGET_BY_NAME = {spec.target: spec for spec in TARGETS}


@dataclass(frozen=True)
class StageMetadata:
    target: str
    dispatch_sha: str
    version: str
    binary_name: str
    binary_sha256: str
    version_output: str

    @classmethod
    def from_path(cls, path: Path) -> StageMetadata:
        _require_regular_file(path, "stage metadata")
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (UnicodeError, json.JSONDecodeError) as error:
            raise ReleaseToolError(f"invalid JSON in {path}: {error}") from error
        expected_keys = {
            "target",
            "dispatch_sha",
            "version",
            "binary_name",
            "binary_sha256",
            "version_output",
        }
        if not isinstance(value, dict) or set(value) != expected_keys:
            actual_keys = sorted(value) if isinstance(value, dict) else type(value).__name__
            raise ReleaseToolError(
                f"{path} must contain exactly {sorted(expected_keys)}, found {actual_keys}"
            )
        if not all(isinstance(value[key], str) for key in expected_keys):
            raise ReleaseToolError(f"every value in {path} must be a string")
        return cls(**value)

    def as_json_bytes(self) -> bytes:
        value = {
            "binary_name": self.binary_name,
            "binary_sha256": self.binary_sha256,
            "dispatch_sha": self.dispatch_sha,
            "target": self.target,
            "version": self.version,
            "version_output": self.version_output,
        }
        return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


@dataclass(frozen=True)
class ValidatedStage:
    spec: TargetSpec
    metadata: StageMetadata
    binary_path: Path


def _validate_version(version: str) -> None:
    if VERSION_RE.fullmatch(version) is None:
        raise ReleaseToolError(f"unsafe Cargo package version: {version!r}")


def _validate_sha(dispatch_sha: str) -> None:
    if SHA_RE.fullmatch(dispatch_sha) is None:
        raise ReleaseToolError(
            "dispatch SHA must be exactly 40 lowercase hexadecimal characters"
        )


def _require_regular_file(path: Path, description: str) -> None:
    if path.is_symlink() or not path.is_file():
        raise ReleaseToolError(f"{description} must be a regular, non-symlink file: {path}")


def _require_empty_output_directory(path: Path) -> None:
    if path.is_symlink():
        raise ReleaseToolError(f"output directory must not be a symlink: {path}")
    if path.exists() and not path.is_dir():
        raise ReleaseToolError(f"output path must be a directory: {path}")
    path.mkdir(parents=True, exist_ok=True)
    entries = list(path.iterdir())
    if entries:
        raise ReleaseToolError(f"output directory must be empty: {path}")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _machine_from_elf(binary: bytes) -> int:
    if len(binary) < 20 or binary[:4] != b"\x7fELF":
        raise ReleaseToolError("binary is not an ELF executable")
    if binary[4] != 2:
        raise ReleaseToolError("release ELF executable must use the 64-bit class")
    if binary[5] != 1:
        raise ReleaseToolError("release ELF executable must be little-endian")
    return int.from_bytes(binary[18:20], "little")


def _machine_from_macho(binary: bytes) -> int:
    if len(binary) < 8:
        raise ReleaseToolError("binary is too short to be a Mach-O executable")
    magic = binary[:4]
    if magic == b"\xcf\xfa\xed\xfe":
        return int.from_bytes(binary[4:8], "little")
    elif magic == b"\xfe\xed\xfa\xcf":
        raise ReleaseToolError("release Mach-O executable must be little-endian")
    else:
        raise ReleaseToolError("binary is not a thin 64-bit Mach-O executable")


def _machine_from_pe(binary: bytes) -> int:
    if len(binary) < 0x40 or binary[:2] != b"MZ":
        raise ReleaseToolError("binary is not a PE executable")
    pe_offset = int.from_bytes(binary[0x3C:0x40], "little")
    if pe_offset > len(binary) - 6 or binary[pe_offset : pe_offset + 4] != b"PE\0\0":
        raise ReleaseToolError("PE executable has an invalid PE header offset or signature")
    return int.from_bytes(binary[pe_offset + 4 : pe_offset + 6], "little")


def validate_binary_architecture(binary: bytes, spec: TargetSpec) -> None:
    parsers = {
        "elf": _machine_from_elf,
        "macho": _machine_from_macho,
        "pe": _machine_from_pe,
    }
    machine = parsers[spec.executable_format](binary)
    if machine != spec.machine:
        raise ReleaseToolError(
            f"{spec.target} requires {spec.executable_format} machine "
            f"0x{spec.machine:x}, found 0x{machine:x}"
        )


def stage_binary(
    *,
    target: str,
    dispatch_sha: str,
    version: str,
    binary_path: Path,
    version_output: str,
    output_dir: Path,
) -> None:
    _validate_version(version)
    _validate_sha(dispatch_sha)
    spec = TARGET_BY_NAME.get(target)
    if spec is None:
        raise ReleaseToolError(f"unsupported release target: {target}")
    if version_output != f"jbotci {version}":
        raise ReleaseToolError(
            f"version smoke output must be exactly 'jbotci {version}', found {version_output!r}"
        )
    _require_regular_file(binary_path, "release binary")
    binary = binary_path.read_bytes()
    validate_binary_architecture(binary, spec)
    _require_empty_output_directory(output_dir)

    staged_binary = output_dir / spec.binary_name
    shutil.copyfile(binary_path, staged_binary)
    os.chmod(staged_binary, 0o755)
    metadata = StageMetadata(
        target=target,
        dispatch_sha=dispatch_sha,
        version=version,
        binary_name=spec.binary_name,
        binary_sha256=_sha256_bytes(binary),
        version_output=version_output,
    )
    metadata_path = output_dir / METADATA_NAME
    metadata_path.write_bytes(metadata.as_json_bytes())
    os.chmod(metadata_path, 0o644)


def _validate_stage_directory(
    stage_dir: Path,
    spec: TargetSpec,
    version: str,
    dispatch_sha: str,
) -> ValidatedStage:
    if stage_dir.is_symlink() or not stage_dir.is_dir():
        raise ReleaseToolError(f"stage must be a non-symlink directory: {stage_dir}")
    entries = {entry.name for entry in stage_dir.iterdir()}
    expected_entries = {spec.binary_name, METADATA_NAME}
    if entries != expected_entries:
        raise ReleaseToolError(
            f"{stage_dir} must contain exactly {sorted(expected_entries)}, found {sorted(entries)}"
        )

    metadata = StageMetadata.from_path(stage_dir / METADATA_NAME)
    expected_metadata = {
        "target": spec.target,
        "dispatch_sha": dispatch_sha,
        "version": version,
        "binary_name": spec.binary_name,
        "version_output": f"jbotci {version}",
    }
    for field_name, expected in expected_metadata.items():
        actual = getattr(metadata, field_name)
        if actual != expected:
            raise ReleaseToolError(
                f"{stage_dir / METADATA_NAME}: {field_name} must be {expected!r}, "
                f"found {actual!r}"
            )
    if re.fullmatch(r"[0-9a-f]{64}", metadata.binary_sha256) is None:
        raise ReleaseToolError(f"invalid binary digest in {stage_dir / METADATA_NAME}")

    binary_path = stage_dir / spec.binary_name
    _require_regular_file(binary_path, "staged release binary")
    # Actions artifacts deliberately do not preserve executable modes.
    os.chmod(binary_path, 0o755)
    actual_digest = _sha256_path(binary_path)
    if actual_digest != metadata.binary_sha256:
        raise ReleaseToolError(
            f"digest mismatch for {binary_path}: expected {metadata.binary_sha256}, "
            f"found {actual_digest}"
        )
    validate_binary_architecture(binary_path.read_bytes(), spec)
    return ValidatedStage(spec, metadata, binary_path)


def validate_staging_root(
    staging_root: Path, version: str, dispatch_sha: str
) -> Mapping[str, ValidatedStage]:
    _validate_version(version)
    _validate_sha(dispatch_sha)
    if staging_root.is_symlink() or not staging_root.is_dir():
        raise ReleaseToolError(
            f"staging root must be a non-symlink directory: {staging_root}"
        )
    entries = {entry.name for entry in staging_root.iterdir()}
    expected_entries = set(TARGET_BY_NAME)
    if entries != expected_entries:
        raise ReleaseToolError(
            f"staging root must contain exactly {sorted(expected_entries)}, "
            f"found {sorted(entries)}"
        )
    return {
        spec.target: _validate_stage_directory(
            staging_root / spec.target, spec, version, dispatch_sha
        )
        for spec in TARGETS
    }


def _read_documents(repository_root: Path) -> Mapping[str, bytes]:
    documents: dict[str, bytes] = {}
    for name in DOCUMENT_NAMES:
        path = repository_root / name
        _require_regular_file(path, f"release document {name}")
        documents[name] = path.read_bytes()
    return documents


def _archive_members(
    version: str, spec: TargetSpec, binary: bytes, documents: Mapping[str, bytes]
) -> list[tuple[str, bytes, int]]:
    root = f"jbotci-{version}"
    files = dict(documents)
    files[spec.binary_name] = binary
    return [
        (f"{root}/{name}", files[name], 0o755 if name == spec.binary_name else 0o644)
        for name in sorted(files)
    ]


def _write_tar_gz(
    path: Path,
    version: str,
    spec: TargetSpec,
    binary: bytes,
    documents: Mapping[str, bytes],
) -> None:
    root = f"jbotci-{version}"
    with path.open("wb") as raw_stream:
        with gzip.GzipFile(
            filename="", mode="wb", fileobj=raw_stream, compresslevel=9, mtime=0
        ) as gzip_stream:
            with tarfile.open(
                fileobj=gzip_stream, mode="w", format=tarfile.USTAR_FORMAT
            ) as archive:
                root_info = tarfile.TarInfo(root)
                root_info.type = tarfile.DIRTYPE
                root_info.mode = 0o755
                root_info.uid = 0
                root_info.gid = 0
                root_info.uname = ""
                root_info.gname = ""
                root_info.mtime = REPRODUCIBLE_TIMESTAMP
                archive.addfile(root_info)
                for member_name, member_data, mode in _archive_members(
                    version, spec, binary, documents
                ):
                    info = tarfile.TarInfo(member_name)
                    info.size = len(member_data)
                    info.mode = mode
                    info.uid = 0
                    info.gid = 0
                    info.uname = ""
                    info.gname = ""
                    info.mtime = REPRODUCIBLE_TIMESTAMP
                    archive.addfile(info, fileobj=io.BytesIO(member_data))


def _zip_info(name: str, mode: int, *, directory: bool = False) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, date_time=ZIP_TIMESTAMP)
    info.create_system = 3
    info.compress_type = zipfile.ZIP_STORED if directory else zipfile.ZIP_DEFLATED
    file_type = stat.S_IFDIR if directory else stat.S_IFREG
    info.external_attr = (file_type | mode) << 16
    if directory:
        info.external_attr |= 0x10
    return info


def _write_zip(
    path: Path,
    version: str,
    spec: TargetSpec,
    binary: bytes,
    documents: Mapping[str, bytes],
) -> None:
    root = f"jbotci-{version}"
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        archive.writestr(_zip_info(f"{root}/", 0o755, directory=True), b"")
        for member_name, member_data, mode in _archive_members(
            version, spec, binary, documents
        ):
            archive.writestr(_zip_info(member_name, mode), member_data)


def archive_names(version: str) -> tuple[str, ...]:
    _validate_version(version)
    return tuple(sorted(spec.archive_name(version) for spec in TARGETS))


def asset_names(version: str) -> tuple[str, ...]:
    return (*archive_names(version), CHECKSUMS_NAME)


def _checksum_text(release_dir: Path, version: str) -> str:
    return "".join(
        f"{_sha256_path(release_dir / name)}  {name}\n" for name in archive_names(version)
    )


def create_release_assets(
    *,
    staging_root: Path,
    repository_root: Path,
    output_dir: Path,
    version: str,
    dispatch_sha: str,
) -> None:
    stages = validate_staging_root(staging_root, version, dispatch_sha)
    documents = _read_documents(repository_root)
    _require_empty_output_directory(output_dir)
    for spec in TARGETS:
        stage = stages[spec.target]
        binary = stage.binary_path.read_bytes()
        archive_path = output_dir / spec.archive_name(version)
        if spec.archive_suffix == ".tar.gz":
            _write_tar_gz(archive_path, version, spec, binary, documents)
        else:
            _write_zip(archive_path, version, spec, binary, documents)
        os.chmod(archive_path, 0o644)
    checksums_path = output_dir / CHECKSUMS_NAME
    checksums_path.write_text(_checksum_text(output_dir, version), encoding="utf-8")
    os.chmod(checksums_path, 0o644)
    validate_release_assets(
        release_dir=output_dir,
        repository_root=repository_root,
        version=version,
        dispatch_sha=dispatch_sha,
        staging_root=staging_root,
    )


def _validate_member_name(name: str) -> None:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or "\\" in name:
        raise ReleaseToolError(f"unsafe archive member path: {name!r}")


def _expected_file_member_names(version: str, spec: TargetSpec) -> list[str]:
    root = f"jbotci-{version}"
    return [
        f"{root}/{name}" for name in sorted((*DOCUMENT_NAMES, spec.binary_name))
    ]


def _validate_tar_archive(path: Path, version: str, spec: TargetSpec) -> Mapping[str, bytes]:
    raw = path.read_bytes()
    if len(raw) < 10 or raw[:3] != b"\x1f\x8b\x08" or raw[4:8] != b"\0\0\0\0":
        raise ReleaseToolError(f"{path} does not have a deterministic gzip header")
    try:
        with tarfile.open(path, "r:gz") as archive:
            members = archive.getmembers()
            names = [member.name for member in members]
            for name in names:
                _validate_member_name(name)
            root = f"jbotci-{version}"
            expected_names = [root, *_expected_file_member_names(version, spec)]
            if names != expected_names:
                raise ReleaseToolError(
                    f"{path} members must be {expected_names}, found {names}"
                )
            root_member = members[0]
            if not root_member.isdir() or root_member.mode != 0o755:
                raise ReleaseToolError(f"{path} archive root must be a mode-0755 directory")
            result: dict[str, bytes] = {}
            for member in members:
                if (
                    member.uid != 0
                    or member.gid != 0
                    or member.uname != ""
                    or member.gname != ""
                    or member.mtime != REPRODUCIBLE_TIMESTAMP
                ):
                    raise ReleaseToolError(f"{path} has non-reproducible metadata on {member.name}")
                if member is root_member:
                    continue
                leaf_name = PurePosixPath(member.name).name
                expected_mode = 0o755 if leaf_name == spec.binary_name else 0o644
                if not member.isfile() or member.mode != expected_mode:
                    raise ReleaseToolError(
                        f"{path} member {member.name} must be a regular "
                        f"mode-{expected_mode:04o} file"
                    )
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise ReleaseToolError(f"cannot read {member.name} from {path}")
                result[leaf_name] = extracted.read()
            return result
    except (tarfile.TarError, EOFError) as error:
        raise ReleaseToolError(f"invalid tar archive {path}: {error}") from error


def _validate_zip_archive(path: Path, version: str, spec: TargetSpec) -> Mapping[str, bytes]:
    try:
        with zipfile.ZipFile(path, "r") as archive:
            members = archive.infolist()
            names = [member.filename for member in members]
            for name in names:
                _validate_member_name(name)
            root = f"jbotci-{version}"
            expected_names = [f"{root}/", *_expected_file_member_names(version, spec)]
            if names != expected_names:
                raise ReleaseToolError(
                    f"{path} members must be {expected_names}, found {names}"
                )
            result: dict[str, bytes] = {}
            for index, member in enumerate(members):
                unix_mode = member.external_attr >> 16
                if member.create_system != 3 or member.date_time != ZIP_TIMESTAMP:
                    raise ReleaseToolError(
                        f"{path} has non-reproducible metadata on {member.filename}"
                    )
                if index == 0:
                    if not member.is_dir() or stat.S_IMODE(unix_mode) != 0o755:
                        raise ReleaseToolError(
                            f"{path} archive root must be a mode-0755 directory"
                        )
                    continue
                leaf_name = PurePosixPath(member.filename).name
                expected_mode = 0o755 if leaf_name == spec.binary_name else 0o644
                if (
                    member.is_dir()
                    or stat.S_IFMT(unix_mode) != stat.S_IFREG
                    or stat.S_IMODE(unix_mode) != expected_mode
                    or member.compress_type != zipfile.ZIP_DEFLATED
                ):
                    raise ReleaseToolError(
                        f"{path} member {member.filename} must be a deflated regular "
                        f"mode-{expected_mode:04o} file"
                    )
                result[leaf_name] = archive.read(member)
            return result
    except (zipfile.BadZipFile, EOFError) as error:
        raise ReleaseToolError(f"invalid zip archive {path}: {error}") from error


def validate_release_assets(
    *,
    release_dir: Path,
    repository_root: Path,
    version: str,
    dispatch_sha: str | None = None,
    staging_root: Path | None = None,
) -> None:
    _validate_version(version)
    if (dispatch_sha is None) != (staging_root is None):
        raise ReleaseToolError("dispatch SHA and staging root must be supplied together")
    if release_dir.is_symlink() or not release_dir.is_dir():
        raise ReleaseToolError(f"release path must be a non-symlink directory: {release_dir}")
    entries = {entry.name for entry in release_dir.iterdir()}
    expected_entries = set(asset_names(version))
    if entries != expected_entries:
        raise ReleaseToolError(
            f"release directory must contain exactly {sorted(expected_entries)}, "
            f"found {sorted(entries)}"
        )
    for name in expected_entries:
        _require_regular_file(release_dir / name, "release asset")

    checksums_path = release_dir / CHECKSUMS_NAME
    try:
        checksum_text = checksums_path.read_text(encoding="ascii")
    except UnicodeError as error:
        raise ReleaseToolError(f"{checksums_path} must be ASCII") from error
    expected_checksum_text = _checksum_text(release_dir, version)
    if checksum_text != expected_checksum_text:
        raise ReleaseToolError(f"{checksums_path} does not exactly cover the five archives")

    documents = _read_documents(repository_root)
    stages = (
        validate_staging_root(staging_root, version, dispatch_sha)
        if staging_root is not None and dispatch_sha is not None
        else None
    )
    for spec in TARGETS:
        archive_path = release_dir / spec.archive_name(version)
        if spec.archive_suffix == ".tar.gz":
            members = _validate_tar_archive(archive_path, version, spec)
        else:
            members = _validate_zip_archive(archive_path, version, spec)
        for name, expected in documents.items():
            if members.get(name) != expected:
                raise ReleaseToolError(f"{archive_path} contains the wrong {name}")
        binary = members.get(spec.binary_name)
        if binary is None:
            raise ReleaseToolError(f"{archive_path} does not contain {spec.binary_name}")
        validate_binary_architecture(binary, spec)
        if (
            stages is not None
            and _sha256_bytes(binary) != stages[spec.target].metadata.binary_sha256
        ):
            raise ReleaseToolError(
                f"{archive_path} binary differs from the validated {spec.target} stage"
            )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    stage = subparsers.add_parser("stage", help="validate and stage one native binary")
    stage.add_argument("--target", required=True, choices=tuple(TARGET_BY_NAME))
    stage.add_argument("--sha", required=True)
    stage.add_argument("--version", required=True)
    stage.add_argument("--binary", required=True, type=Path)
    stage.add_argument("--version-output", required=True)
    stage.add_argument("--output", required=True, type=Path)

    package = subparsers.add_parser("package", help="create all five release archives")
    package.add_argument("--staging-root", required=True, type=Path)
    package.add_argument("--repository-root", required=True, type=Path)
    package.add_argument("--output", required=True, type=Path)
    package.add_argument("--version", required=True)
    package.add_argument("--sha", required=True)

    validate = subparsers.add_parser("validate", help="validate an assembled release directory")
    validate.add_argument("--release-dir", required=True, type=Path)
    validate.add_argument("--repository-root", required=True, type=Path)
    validate.add_argument("--version", required=True)
    validate.add_argument("--staging-root", type=Path)
    validate.add_argument("--sha")

    expected = subparsers.add_parser("expected-assets", help="print canonical asset names")
    expected.add_argument("--version", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.command == "stage":
            stage_binary(
                target=args.target,
                dispatch_sha=args.sha,
                version=args.version,
                binary_path=args.binary,
                version_output=args.version_output,
                output_dir=args.output,
            )
        elif args.command == "package":
            create_release_assets(
                staging_root=args.staging_root,
                repository_root=args.repository_root,
                output_dir=args.output,
                version=args.version,
                dispatch_sha=args.sha,
            )
        elif args.command == "validate":
            validate_release_assets(
                release_dir=args.release_dir,
                repository_root=args.repository_root,
                version=args.version,
                dispatch_sha=args.sha,
                staging_root=args.staging_root,
            )
        elif args.command == "expected-assets":
            print("\n".join(asset_names(args.version)))
        else:
            raise AssertionError(f"unhandled command: {args.command}")
        return 0
    except (OSError, ReleaseToolError) as error:
        print(f"cli_release.py: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
