"""Paths shared by the Cargo 1.100 inventory and trace."""
from __future__ import annotations

from pathlib import Path
import re


RECORD = "jbotci-inputs-v1.json"
UNIT_ID = re.compile(r"[0-9a-f]{16}")

# Cargo 1.100.0 writes these names next to `<profile>/build`.
# Restore keeps them and removes every other profile entry, including `deps`.
PROFILE_KEEP = frozenset({
    "build",
    ".cargo-lock",
    ".cargo-artifact-lock",
    ".cargo-build-lock",
})

# Cargo adds an optional mode prefix to the target kind and name.
_TARGET_KINDS = ("lib", "bin", "integration-test", "example", "bench")
EXACT_TEST_MARKERS = tuple(f"test-{kind}-" for kind in _TARGET_KINDS)
_FINGERPRINT_STEM = re.compile(rf"(?:(?:test-|doc-)?(?:{'|'.join(_TARGET_KINDS)})|(?:run-)?build-script)-.+")


def is_fingerprint_hash_name(name: str) -> bool:
    return _FINGERPRINT_STEM.fullmatch(name) is not None


def is_auxiliary_fingerprint_entry(name: str) -> bool:
    """Return true for a fingerprint entry that is not a Cargo hash file.

    Cargo writes `invoked.timestamp`, `dep-*`, and `output-*`.
    The cache tool writes the input record.
    """
    return name in ("invoked.timestamp", RECORD) or name.startswith(("dep-", "output-"))


def fingerprint_from_out(out_dir: Path) -> Path | None:
    """Locate the fingerprint for `<profile>/build/<package>/<16 hex>/out`."""
    if out_dir.name != "out" or not UNIT_ID.fullmatch(out_dir.parent.name):
        return None
    if len(out_dir.parents) < 3 or out_dir.parents[2].name != "build":
        return None
    return out_dir.parent / "fingerprint"


def fingerprint_profile(fingerprint: Path) -> Path:
    """Return the profile for `<profile>/build/<package>/<id>/fingerprint`."""
    return fingerprint.parents[3]


def profile_inside_target(target: Path, profile: Path) -> bool:
    return profile.is_relative_to(target) and profile != target


def inside_unit_out(root: Path, path: Path) -> bool:
    """Return true if the path is below a unit output directory under the root."""
    if not path.is_relative_to(root) or path == root:
        return False
    for parent in path.parents:
        if parent == root:
            break
        if fingerprint_from_out(parent) is not None:
            return True
    return False
