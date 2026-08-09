from __future__ import annotations

import gzip
import hashlib
import io
import json
import os
import stat
import tarfile
import tempfile
import unittest
import zipfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from tools import cli_release


VERSION = "0.1.0"
DISPATCH_SHA = "a" * 40


def executable_for(spec: cli_release.TargetSpec) -> bytes:
    if spec.executable_format == "elf":
        value = bytearray(64)
        value[:6] = b"\x7fELF\x02\x01"
        value[18:20] = spec.machine.to_bytes(2, "little")
        return bytes(value)
    if spec.executable_format == "macho":
        return b"\xcf\xfa\xed\xfe" + spec.machine.to_bytes(4, "little") + b"\0" * 56
    if spec.executable_format == "pe":
        value = bytearray(0x98)
        value[:2] = b"MZ"
        value[0x3C:0x40] = (0x80).to_bytes(4, "little")
        value[0x80:0x84] = b"PE\0\0"
        value[0x84:0x86] = spec.machine.to_bytes(2, "little")
        return bytes(value)
    raise AssertionError(f"unhandled executable format {spec.executable_format}")


class CliReleaseTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.repository_root = self.root / "repository"
        self.repository_root.mkdir()
        for name in cli_release.DOCUMENT_NAMES:
            (self.repository_root / name).write_bytes(f"contents of {name}\n".encode())
        self.staging_root = self.root / "staging"
        self.staging_root.mkdir()
        for spec in cli_release.TARGETS:
            self.write_stage(spec)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write_stage(
        self,
        spec: cli_release.TargetSpec,
        *,
        target: str | None = None,
        digest: str | None = None,
        version: str = VERSION,
        dispatch_sha: str = DISPATCH_SHA,
        version_output: str | None = None,
    ) -> None:
        stage = self.staging_root / spec.target
        stage.mkdir(exist_ok=True)
        binary = executable_for(spec)
        binary_path = stage / spec.binary_name
        binary_path.write_bytes(binary)
        os.chmod(binary_path, 0o644)
        metadata = cli_release.StageMetadata(
            target=target or spec.target,
            dispatch_sha=dispatch_sha,
            version=version,
            binary_name=spec.binary_name,
            binary_sha256=digest or hashlib.sha256(binary).hexdigest(),
            version_output=version_output or f"jbotci {version}",
        )
        (stage / cli_release.METADATA_NAME).write_bytes(metadata.as_json_bytes())

    def package(self, name: str = "release") -> Path:
        output = self.root / name
        cli_release.create_release_assets(
            staging_root=self.staging_root,
            repository_root=self.repository_root,
            output_dir=output,
            version=VERSION,
            dispatch_sha=DISPATCH_SHA,
        )
        return output

    def rewrite_checksums(self, release_dir: Path) -> None:
        text = "".join(
            f"{hashlib.sha256((release_dir / name).read_bytes()).hexdigest()}  {name}\n"
            for name in cli_release.archive_names(VERSION)
        )
        (release_dir / cli_release.CHECKSUMS_NAME).write_text(text, encoding="ascii")

    def test_asset_names_are_target_qualified_and_stable(self) -> None:
        self.assertEqual(
            cli_release.asset_names(VERSION),
            (
                "jbotci-0.1.0-aarch64-apple-darwin.tar.gz",
                "jbotci-0.1.0-aarch64-unknown-linux-musl.tar.gz",
                "jbotci-0.1.0-x86_64-apple-darwin.tar.gz",
                "jbotci-0.1.0-x86_64-pc-windows-msvc.zip",
                "jbotci-0.1.0-x86_64-unknown-linux-musl.tar.gz",
                "SHA256SUMS",
            ),
        )

    def test_expected_assets_cli_prints_the_exact_release_set(self) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            result = cli_release.main(["expected-assets", "--version", VERSION])

        self.assertEqual(result, 0)
        self.assertEqual(
            tuple(stdout.getvalue().splitlines()), cli_release.asset_names(VERSION)
        )

    def test_expected_assets_cli_reports_invalid_versions(self) -> None:
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = cli_release.main(["expected-assets", "--version", "../unsafe"])

        self.assertEqual(result, 1)
        self.assertIn("unsafe Cargo package version", stderr.getvalue())

    def test_stage_binary_writes_exact_metadata_and_restores_mode(self) -> None:
        spec = cli_release.TARGET_BY_NAME["aarch64-unknown-linux-musl"]
        source = self.root / "source-binary"
        source.write_bytes(executable_for(spec))
        os.chmod(source, 0o600)
        output = self.root / "single-stage"

        cli_release.stage_binary(
            target=spec.target,
            dispatch_sha=DISPATCH_SHA,
            version=VERSION,
            binary_path=source,
            version_output=f"jbotci {VERSION}",
            output_dir=output,
        )

        self.assertEqual({path.name for path in output.iterdir()}, {"jbotci", "metadata.json"})
        self.assertEqual(stat.S_IMODE((output / "jbotci").stat().st_mode), 0o755)
        metadata = cli_release.StageMetadata.from_path(output / "metadata.json")
        self.assertEqual(metadata.target, spec.target)
        self.assertEqual(metadata.dispatch_sha, DISPATCH_SHA)
        self.assertEqual(metadata.binary_sha256, hashlib.sha256(source.read_bytes()).hexdigest())
        self.assertEqual(metadata.version_output, f"jbotci {VERSION}")

    def test_stage_binary_rejects_a_wrong_version_smoke_string(self) -> None:
        spec = cli_release.TARGETS[0]
        source = self.root / "source-binary"
        source.write_bytes(executable_for(spec))
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "version smoke output"):
            cli_release.stage_binary(
                target=spec.target,
                dispatch_sha=DISPATCH_SHA,
                version=VERSION,
                binary_path=source,
                version_output="jbotci 9.9.9",
                output_dir=self.root / "single-stage",
            )

    def test_architecture_validation_binds_every_target(self) -> None:
        for spec in cli_release.TARGETS:
            with self.subTest(target=spec.target):
                cli_release.validate_binary_architecture(executable_for(spec), spec)

        with self.assertRaisesRegex(cli_release.ReleaseToolError, "requires elf machine"):
            cli_release.validate_binary_architecture(
                executable_for(cli_release.TARGETS[0]), cli_release.TARGETS[1]
            )
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "requires macho machine"):
            cli_release.validate_binary_architecture(
                executable_for(cli_release.TARGETS[2]), cli_release.TARGETS[3]
            )

    def test_architecture_validation_rejects_wrong_or_truncated_formats(self) -> None:
        for spec in cli_release.TARGETS:
            with self.subTest(target=spec.target):
                with self.assertRaises(cli_release.ReleaseToolError):
                    cli_release.validate_binary_architecture(b"not an executable", spec)

        pe_spec = cli_release.TARGET_BY_NAME["x86_64-pc-windows-msvc"]
        invalid_pe = bytearray(executable_for(pe_spec))
        invalid_pe[0x3C:0x40] = (len(invalid_pe) + 1).to_bytes(4, "little")
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "invalid PE header"):
            cli_release.validate_binary_architecture(bytes(invalid_pe), pe_spec)

        elf_spec = cli_release.TARGETS[0]
        big_endian_elf = bytearray(executable_for(elf_spec))
        big_endian_elf[5] = 2
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "little-endian"):
            cli_release.validate_binary_architecture(bytes(big_endian_elf), elf_spec)

        macho_spec = cli_release.TARGETS[2]
        big_endian_macho = (
            b"\xfe\xed\xfa\xcf" + macho_spec.machine.to_bytes(4, "big") + b"\0" * 56
        )
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "little-endian"):
            cli_release.validate_binary_architecture(big_endian_macho, macho_spec)

    def test_metadata_validation_rejects_target_digest_sha_and_version_mismatches(self) -> None:
        cases = (
            {"target": "aarch64-unknown-linux-musl"},
            {"digest": "0" * 64},
            {"dispatch_sha": "b" * 40},
            {"version": "2.0.0", "version_output": "jbotci 2.0.0"},
            {"version_output": "jbotci 2.0.0"},
        )
        spec = cli_release.TARGETS[0]
        for overrides in cases:
            with self.subTest(overrides=overrides):
                stage = self.staging_root / spec.target
                for path in stage.iterdir():
                    path.unlink()
                self.write_stage(spec, **overrides)
                with self.assertRaises(cli_release.ReleaseToolError):
                    cli_release.validate_staging_root(
                        self.staging_root, VERSION, DISPATCH_SHA
                    )
                for path in stage.iterdir():
                    path.unlink()
                self.write_stage(spec)

    def test_metadata_validation_rejects_unknown_fields(self) -> None:
        metadata_path = self.staging_root / cli_release.TARGETS[0].target / "metadata.json"
        value = json.loads(metadata_path.read_text())
        value["unexpected"] = "value"
        metadata_path.write_text(json.dumps(value), encoding="utf-8")
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "must contain exactly"):
            cli_release.validate_staging_root(self.staging_root, VERSION, DISPATCH_SHA)

    def test_staging_rejects_extra_files_and_symlinked_binaries(self) -> None:
        extra = self.staging_root / cli_release.TARGETS[0].target / "unexpected"
        extra.write_text("unexpected")
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "must contain exactly"):
            cli_release.validate_staging_root(self.staging_root, VERSION, DISPATCH_SHA)
        extra.unlink()

        spec = cli_release.TARGETS[0]
        binary = self.staging_root / spec.target / spec.binary_name
        replacement = self.root / "replacement"
        replacement.write_bytes(binary.read_bytes())
        binary.unlink()
        binary.symlink_to(replacement)
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "non-symlink"):
            cli_release.validate_staging_root(self.staging_root, VERSION, DISPATCH_SHA)

    def test_archives_have_exact_layout_contents_and_normalized_modes(self) -> None:
        release_dir = self.package()
        expected_root = f"jbotci-{VERSION}"
        for spec in cli_release.TARGETS:
            archive_path = release_dir / spec.archive_name(VERSION)
            expected_files = sorted((*cli_release.DOCUMENT_NAMES, spec.binary_name))
            if spec.archive_suffix == ".tar.gz":
                with tarfile.open(archive_path, "r:gz") as archive:
                    members = archive.getmembers()
                    self.assertEqual(
                        [member.name for member in members],
                        [expected_root, *[f"{expected_root}/{name}" for name in expected_files]],
                    )
                    self.assertEqual(members[0].mode, 0o755)
                    for member in members[1:]:
                        expected_mode = 0o755 if member.name.endswith("/jbotci") else 0o644
                        self.assertEqual(member.mode, expected_mode)
                        self.assertEqual((member.uid, member.gid, member.mtime), (0, 0, 0))
            else:
                with zipfile.ZipFile(archive_path) as archive:
                    members = archive.infolist()
                    self.assertEqual(
                        [member.filename for member in members],
                        [
                            f"{expected_root}/",
                            *[f"{expected_root}/{name}" for name in expected_files],
                        ],
                    )
                    self.assertEqual(stat.S_IMODE(members[0].external_attr >> 16), 0o755)
                    for member in members[1:]:
                        expected_mode = 0o755 if member.filename.endswith(".exe") else 0o644
                        self.assertEqual(
                            stat.S_IMODE(member.external_attr >> 16), expected_mode
                        )
                        self.assertEqual(member.date_time, cli_release.ZIP_TIMESTAMP)

    def test_actions_mode_loss_is_repaired_before_packaging(self) -> None:
        for spec in cli_release.TARGETS:
            binary = self.staging_root / spec.target / spec.binary_name
            self.assertEqual(stat.S_IMODE(binary.stat().st_mode), 0o644)
        self.package()
        for spec in cli_release.TARGETS:
            binary = self.staging_root / spec.target / spec.binary_name
            self.assertEqual(stat.S_IMODE(binary.stat().st_mode), 0o755)

    def test_identical_inputs_produce_byte_identical_assets(self) -> None:
        first = self.package("first")
        second = self.package("second")
        for name in cli_release.asset_names(VERSION):
            with self.subTest(asset=name):
                self.assertEqual((first / name).read_bytes(), (second / name).read_bytes())

    def test_checksums_cover_exactly_the_archives_in_sorted_order(self) -> None:
        release_dir = self.package()
        actual = (release_dir / cli_release.CHECKSUMS_NAME).read_text(encoding="ascii")
        expected = "".join(
            f"{hashlib.sha256((release_dir / name).read_bytes()).hexdigest()}  {name}\n"
            for name in cli_release.archive_names(VERSION)
        )
        self.assertEqual(actual, expected)
        self.assertNotIn(cli_release.CHECKSUMS_NAME, actual)

    def test_validation_detects_checksum_and_unexpected_asset_changes(self) -> None:
        release_dir = self.package()
        checksum_path = release_dir / cli_release.CHECKSUMS_NAME
        checksum_path.write_text("0" * 64 + "  wrong.tar.gz\n", encoding="ascii")
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "exactly cover"):
            cli_release.validate_release_assets(
                release_dir=release_dir,
                repository_root=self.repository_root,
                version=VERSION,
            )

        self.rewrite_checksums(release_dir)
        (release_dir / "unexpected").write_text("unexpected")
        with self.assertRaisesRegex(cli_release.ReleaseToolError, "contain exactly"):
            cli_release.validate_release_assets(
                release_dir=release_dir,
                repository_root=self.repository_root,
                version=VERSION,
            )

    def test_release_validation_requires_sha_and_staging_root_together(self) -> None:
        release_dir = self.package()
        for supplied in (
            {"dispatch_sha": DISPATCH_SHA},
            {"staging_root": self.staging_root},
        ):
            with self.subTest(supplied=tuple(supplied)):
                with self.assertRaisesRegex(
                    cli_release.ReleaseToolError, "must be supplied together"
                ):
                    cli_release.validate_release_assets(
                        release_dir=release_dir,
                        repository_root=self.repository_root,
                        version=VERSION,
                        **supplied,
                    )

        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = cli_release.main(
                [
                    "validate",
                    "--release-dir",
                    str(release_dir),
                    "--repository-root",
                    str(self.repository_root),
                    "--version",
                    VERSION,
                    "--sha",
                    DISPATCH_SHA,
                ]
            )
        self.assertEqual(result, 1)
        self.assertIn("must be supplied together", stderr.getvalue())

    def test_validation_rejects_symlink_archive_member_even_with_updated_checksum(self) -> None:
        release_dir = self.package()
        spec = cli_release.TARGETS[0]
        archive_path = release_dir / spec.archive_name(VERSION)
        root = f"jbotci-{VERSION}"
        with archive_path.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
                with tarfile.open(fileobj=compressed, mode="w") as archive:
                    root_info = tarfile.TarInfo(root)
                    root_info.type = tarfile.DIRTYPE
                    root_info.mode = 0o755
                    archive.addfile(root_info)
                    for name in sorted((*cli_release.DOCUMENT_NAMES, spec.binary_name)):
                        member = tarfile.TarInfo(f"{root}/{name}")
                        if name == spec.binary_name:
                            member.type = tarfile.SYMTYPE
                            member.linkname = "/outside"
                            member.mode = 0o755
                            archive.addfile(member)
                        else:
                            value = (self.repository_root / name).read_bytes()
                            member.size = len(value)
                            member.mode = 0o644
                            archive.addfile(member, io.BytesIO(value))
        self.rewrite_checksums(release_dir)

        with self.assertRaisesRegex(cli_release.ReleaseToolError, "regular mode-0755"):
            cli_release.validate_release_assets(
                release_dir=release_dir,
                repository_root=self.repository_root,
                version=VERSION,
            )

    def test_archive_path_validation_rejects_absolute_and_parent_paths(self) -> None:
        for name in ("/absolute/jbotci", "root/../jbotci", r"root\jbotci"):
            with self.subTest(name=name):
                with self.assertRaisesRegex(cli_release.ReleaseToolError, "unsafe"):
                    cli_release._validate_member_name(name)

    def test_validation_binds_archived_binary_to_stage(self) -> None:
        release_dir = self.package()
        spec = cli_release.TARGET_BY_NAME["x86_64-pc-windows-msvc"]
        archive_path = release_dir / spec.archive_name(VERSION)
        root = f"jbotci-{VERSION}"
        with zipfile.ZipFile(archive_path, "w") as archive:
            archive.writestr(cli_release._zip_info(f"{root}/", 0o755, directory=True), b"")
            for name in sorted((*cli_release.DOCUMENT_NAMES, spec.binary_name)):
                if name == spec.binary_name:
                    value = executable_for(spec) + b"changed"
                    mode = 0o755
                else:
                    value = (self.repository_root / name).read_bytes()
                    mode = 0o644
                archive.writestr(cli_release._zip_info(f"{root}/{name}", mode), value)
        self.rewrite_checksums(release_dir)

        with self.assertRaisesRegex(cli_release.ReleaseToolError, "differs from the validated"):
            cli_release.validate_release_assets(
                release_dir=release_dir,
                repository_root=self.repository_root,
                version=VERSION,
                dispatch_sha=DISPATCH_SHA,
                staging_root=self.staging_root,
            )


if __name__ == "__main__":
    unittest.main()
