"""Paths shared by the Cargo 1.100 inventory and trace."""
from __future__ import annotations

from pathlib import Path
import re


RECORD = "jbotci-inputs-v1.json"
UNIT_ID = re.compile(r"[0-9a-f]{16}")


def fingerprint_from_out(out_dir: Path) -> Path | None:
    """Locate the fingerprint for `<profile>/build/<package>/<16 hex>/out`."""
    if out_dir.name != "out" or not UNIT_ID.fullmatch(out_dir.parent.name):
        return None
    if len(out_dir.parents) < 3 or out_dir.parents[2].name != "build":
        return None
    return out_dir.parent / "fingerprint"


def profile_inside_target(target: Path, profile: Path) -> bool:
    return profile.is_relative_to(target) and profile != target


def inside_unit_out(root: Path, path: Path) -> bool:
    """Return true if the path is below a unit output directory under the root."""
    if not path.is_relative_to(root):
        return False
    for parent in path.parents:
        if parent == root:
            break
        if fingerprint_from_out(parent) is not None:
            return True
    return False
