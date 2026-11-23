"""Unit tests for the release compiler gate."""

from __future__ import annotations

import io
import os
from pathlib import Path
import tempfile
import unittest
from contextlib import redirect_stderr
from unittest.mock import patch

from tools.release_compiler import (
    CompilerPinError, main, require_verbose, toolchain_channel, verbose_field, version_line,
)


LF = '[toolchain]\nchannel = "1.100.0"\n'
CRLF = '[toolchain]\r\nchannel = "1.100.0"\r\n'
VERBOSE = (
    "rustc 1.100.0 (abc 2026-11-09)\n"
    "host: aarch64-unknown-linux-gnu\n"
    "release: 1.100.0\n"
)


class ReleaseCompilerTests(unittest.TestCase):
    def test_toolchain_channel_accepts_lf_crlf_comments_and_single_quotes(self) -> None:
        documents = [
            LF,
            CRLF,
            "[toolchain]\nchannel = '1.100.0'\n",
            '[toolchain]\nchannel = "1.100.0" # pin\n',
            "[toolchain]\r\nchannel = '1.100.0' # pin\r\n",
        ]
        for text in documents:
            with self.subTest(text=text):
                self.assertEqual(toolchain_channel(text), "1.100.0")

    def test_toolchain_channel_rejects_a_missing_or_blank_channel(self) -> None:
        documents = [
            "",
            "[toolchain]\n",
            '[toolchain]\nchannel = ""\n',
            '[toolchain]\nchannel = " 1.100.0"\n',
            '[toolchain]\nchannel = "1.100.0 "\n',
            "not toml {{\n",
            "[toolchain]\nchannel = 1\n",
            "[package]\nname = 'demo'\n",
        ]
        for text in documents:
            with self.subTest(text=text):
                with self.assertRaises(CompilerPinError) as caught:
                    toolchain_channel(text)
                self.assertEqual(str(caught.exception), "The toolchain file has no channel.")

    def test_require_verbose_matches_one_release_and_one_host(self) -> None:
        require_verbose("1.100.0", VERBOSE, None)
        require_verbose("1.100.0", VERBOSE, "aarch64-unknown-linux-gnu")
        require_verbose("1.100.0", VERBOSE.replace("\n", "\r\n"), "aarch64-unknown-linux-gnu")

    def test_require_verbose_rejects_a_wrong_or_repeated_field(self) -> None:
        cases = {
            "release": (VERBOSE.replace("release: 1.100.0", "release: 1.99.0"), "aarch64-unknown-linux-gnu"),
            "host": (VERBOSE, "x86_64-unknown-linux-musl"),
            "missing-release": ("host: aarch64-unknown-linux-gnu\n", None),
            "missing-host": ("release: 1.100.0\n", "aarch64-unknown-linux-gnu"),
            "repeated-release": ("release: 1.100.0\nrelease: 1.100.0\n", None),
            "blank-release": ("release: \n", None),
            "padded-release": ("release: 1.100.0 \n", None),
        }
        for name, (text, host) in cases.items():
            with self.subTest(name=name):
                with self.assertRaises(CompilerPinError):
                    require_verbose("1.100.0", text, host)
        self.assertEqual(verbose_field(VERBOSE, "release"), "1.100.0")

    def test_version_line_returns_the_first_line(self) -> None:
        self.assertEqual(version_line("rustc 1.100.0 (abc)\nextra\n"), "rustc 1.100.0 (abc)")
        self.assertEqual(version_line("rustc 1.100.0 (abc)\r\n"), "rustc 1.100.0 (abc)")
        for text in ("", "\n", " rustc 1.100.0\n", "rustc 1.100.0 \n"):
            with self.subTest(text=text):
                with self.assertRaises(CompilerPinError):
                    version_line(text)

    def test_main_gates_a_crlf_file_and_the_version_line(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write_rustc(root, VERBOSE, "rustc 1.100.0 (abc 2026-11-09)\n")
            lf_file = root / "lf.toml"
            crlf_file = root / "crlf.toml"
            lf_file.write_text(LF, encoding="utf-8", newline="\n")
            crlf_file.write_bytes(CRLF.encode("utf-8"))
            status, errors = self._run(root, ["gate", "--file", str(lf_file)])
            self.assertEqual((status, errors), (0, ""))
            status, errors = self._run(
                root,
                ["gate", "--file", str(crlf_file), "--host", "aarch64-unknown-linux-gnu"],
            )
            self.assertEqual((status, errors), (0, ""))
            status, errors = self._run(
                root,
                ["gate", "--file", str(crlf_file), "--host", "x86_64-unknown-linux-musl"],
            )
            self.assertEqual(status, 1)
            self.assertIn("The rustc host does not match the expected host.", errors)
            status, errors = self._run(
                root,
                ["expect-version", "--expect", "rustc 1.100.0 (abc 2026-11-09)"],
            )
            self.assertEqual((status, errors), (0, ""))
            status, errors = self._run(root, ["expect-version", "--expect", "rustc 1.99.0"])
            self.assertEqual(status, 1)
            self.assertIn("The rustc version line does not match the pin.", errors)

    def test_main_reports_an_unavailable_compiler_and_a_bad_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            missing = root / "missing.toml"
            status, _errors = self._run(root, ["gate", "--file", str(missing)], prepend=False)
            self.assertEqual(status, 1)
            bad = root / "bad.toml"
            bad.write_text("[toolchain]\n", encoding="utf-8")
            self._write_rustc(root, VERBOSE, "rustc 1.100.0 (abc 2026-11-09)\n")
            status, errors = self._run(root, ["gate", "--file", str(bad)])
            self.assertEqual(status, 1)
            self.assertIn("The toolchain file has no channel.", errors)
            good = root / "good.toml"
            good.write_text(LF, encoding="utf-8")
            empty = root / "empty-bin"
            empty.mkdir()
            status, errors = self._run(empty, ["gate", "--file", str(good)], prepend=False)
            self.assertEqual(status, 1)
            self.assertIn("The pinned compiler is unavailable.", errors)

    def _write_rustc(self, root: Path, verbose: str, version: str) -> None:
        script = root / "rustc"
        script.write_text(
            "#!/usr/bin/env python3\n"
            "import sys\n"
            f"verbose = {verbose!r}\n"
            f"version = {version!r}\n"
            "argument = sys.argv[1]\n"
            "if argument == '-vV':\n"
            "    sys.stdout.write(verbose)\n"
            "elif argument == '--version':\n"
            "    sys.stdout.write(version)\n"
            "else:\n"
            "    sys.exit(1)\n",
            encoding="utf-8",
        )
        script.chmod(0o755)

    def _run(self, root: Path, argv: list[str], prepend: bool = True) -> tuple[int, str]:
        buffer = io.StringIO()
        path = f"{root}{os.pathsep}{os.environ.get('PATH', '')}" if prepend else str(root)
        with patch.dict(os.environ, {"PATH": path}):
            with redirect_stderr(buffer):
                status = main(argv)
        return status, buffer.getvalue()


if __name__ == "__main__":
    unittest.main()
