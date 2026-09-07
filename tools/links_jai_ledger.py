"""Shared epoch-10 stage rules over actual observations, not expectation-file digests.

The Rust collector records what happened. This module derives the nine ledger surfaces
and checks permitted transitions. Neither a manual label nor a successful capture makes
an otherwise prohibited transition acceptable.
"""

from __future__ import annotations

from dataclasses import dataclass
import argparse
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import sys
import tomllib
from typing import Any, Mapping


STAGES = ("base", "c-a", "c-b")
SURFACES = (
    "status", "owner", "syntax_raw_full_tree_sha256", "syntax_diagnostics_sha256",
    "semantics_refs_status", "semantics_refs_raw_sha256", "semantics_refs_error_sha256",
    "gentufa_tree_sha256", "gentufa_json_sha256",
)
INAPPLICABLE = "INAPPLICABLE"
UNPROBED = "UNPROBED"
ALICE_ID = "corpus.alis.full-alice"
OWNERS = {
    "full-linked-term": "FullLinkedTerm",
    "connected-linked-term": "ConnectedLinkedTerm",
    "bound-linked-term-connection": "BoundLinkedTermConnection",
    "place-tagged-linked-sumti": "PlaceTaggedLinkedSumti",
    "tense-tagged-linked-sumti": "TenseTaggedLinkedSumti",
    "plain-linked-sumti": "PlainLinkedSumti",
    "empty-linked-sumti": "EmptyLinkedSumti",
}


class ValidationError(ValueError):
    """An absent observation or an unauthorized/ill-formed transition."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def json_text(value: Any) -> str:
    # Keep the collector's field order, matching the fixture diagnostic serializer.
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def exact_keys(value: Mapping[str, Any], keys: set[str], label: str) -> None:
    require(isinstance(value, Mapping), f"{label}: expected an object")
    require(set(value) == keys, f"{label}: fields differ: {set(value) ^ keys}")


def nonempty_text(value: Any, label: str) -> str:
    require(isinstance(value, str) and bool(value), f"{label}: missing text")
    return value


@dataclass(frozen=True)
class Identity:
    """Source/dialect values loaded from the base fixture, excluding its expectations."""

    id: str
    path: str
    source: str
    dialect: str | None
    target: Mapping[str, Any]
    refs: bool
    gentufa_tree: bool
    gentufa_json: bool

    def check(self, observation: Mapping[str, Any]) -> None:
        require(isinstance(observation, Mapping), f"{self.id}: expected an observation object")
        for name in ("id", "path", "source", "dialect", "target"):
            require(observation.get(name) == getattr(self, name), f"{self.id}: changed {name}")

    def check_fixture(self, root: Path) -> None:
        """Expectation edits do not change identity; source/ID/dialect edits do."""
        fixture = read_fixture(root, self.path)
        require(fixture.get("id") == self.id, f"{self.id}: changed fixture ID")
        require(fixture.get("lojban") == self.source, f"{self.id}: changed fixture source")
        require(fixture.get("dialect") == self.dialect, f"{self.id}: changed fixture dialect")


@dataclass(frozen=True)
class Snapshot:
    stage: str
    values: Mapping[str, str]
    # The retained base has compact measured cells, not reconstructed diagnostic arrays.
    # Every new parser observation carries its complete array. Never interpret None as [].
    diagnostics: tuple[Mapping[str, Any], ...] | None

    @property
    def error_count(self) -> int:
        require(self.diagnostics is not None, "diagnostic counts need the actual array")
        return sum(item["severity"] == "error" for item in self.diagnostics)

    @property
    def owner(self) -> str | None:
        raw = self.values["owner"]
        return None if raw == INAPPLICABLE else json.loads(raw)["variant"]


@dataclass(frozen=True)
class Surface:
    status: str
    raw: str
    error: str


def surface(value: Mapping[str, Any], applicable: bool, strict_success: bool,
            label: str, *, semantic: bool = False) -> Surface:
    """Return status/raw/error cells without inventing unavailable downstream output."""
    if not applicable:
        exact_keys(value, {"status"}, label)
        require(value["status"] == "inapplicable", f"{label}: changed applicability")
        return Surface(INAPPLICABLE, INAPPLICABLE, INAPPLICABLE)
    if not strict_success:
        exact_keys(value, {"status"}, label)
        require(value["status"] == "blocked-by-syntax-failure", f"{label}: failure must block output")
        return Surface("blocked by syntax failure", INAPPLICABLE, INAPPLICABLE)
    if value.get("status") == "success":
        exact_keys(value, {"status", "value"}, label)
        return Surface("success", digest(nonempty_text(value["value"], label)), INAPPLICABLE)
    require(semantic and value.get("status") == "failure", f"{label}: missing successful output")
    exact_keys(value, {"status", "error"}, label)
    return Surface("failure", INAPPLICABLE, digest(nonempty_text(value["error"], label)))


def validate_diagnostics(diagnostics: Any, source: str, label: str) -> None:
    require(isinstance(diagnostics, list), f"{label}: diagnostics were not captured")
    source_bytes = source.encode("utf-8")
    for item in diagnostics:
        require(isinstance(item, dict), f"{label}: malformed diagnostic")
        require(isinstance(item.get("severity"), str) and item["severity"] in {"error", "warning", "advice"},
                f"{label}: unknown severity")
        nonempty_text(item.get("code"), "diagnostic code")
        span = item.get("byte-span")
        require(isinstance(span, list) and len(span) == 2 and
                all(isinstance(x, int) and not isinstance(x, bool) for x in span) and
                0 <= span[0] <= span[1] <= len(source_bytes), f"{label}: invalid diagnostic extent")
        require(isinstance(item.get("source-text"), str), f"{label}: missing diagnostic source text")
        try:
            actual_text = source_bytes[span[0]:span[1]].decode("utf-8")
        except UnicodeDecodeError as error:
            raise ValidationError(f"{label}: diagnostic splits a UTF-8 character") from error
        require(item["source-text"] == actual_text, f"{label}: wrong diagnostic source text")


def snapshot(identity: Identity, observation: Mapping[str, Any], stage: str) -> Snapshot:
    require(stage in STAGES, f"unknown stage {stage}")
    identity.check(observation)
    require(observation.get("stage") == stage, f"{identity.id}: wrong observation stage")
    for name in ("syntax", "refs", "gentufa_tree", "gentufa_json"):
        require(isinstance(observation.get(name), Mapping), f"{identity.id}: missing {name} observation")
    syntax = observation["syntax"]
    diagnostics = syntax.get("diagnostics")
    validate_diagnostics(diagnostics, identity.source, identity.id)
    error_count = sum(item["severity"] == "error" for item in diagnostics)
    success = syntax.get("status") == "success"
    if success:
        exact_keys(syntax, {"status", "raw", "target", "diagnostics"}, "strict success")
        require(error_count == 0, f"{identity.id}: successful syntax has an error")
        raw = digest(nonempty_text(syntax["raw"], "strict tree"))
        target = syntax["target"]
        exact_keys(target, {"status", "link"}, "successful target")
        require(target["status"] == "found", f"{identity.id}: missing or ambiguous target")
        exact_keys(target["link"], {"anchor", "owner"}, "target link")
        require(target["link"]["anchor"] == identity.target, f"{identity.id}: wrong successor anchor")
        owner_name = nonempty_text(target["link"]["owner"], "target owner")
        variant = OWNERS.get(owner_name)
        require(variant is not None, f"{identity.id}: unknown owner")
        owner = json_text({"kind": "syntax-node", "variant": variant})
    else:
        require(syntax.get("status") == "failure", f"{identity.id}: unexpected upstream blockage")
        exact_keys(syntax, {"status", "error", "diagnostics"}, "strict failure")
        nonempty_text(syntax["error"], "strict error")
        require(error_count > 0, f"{identity.id}: failure without an error diagnostic")
        raw = owner = INAPPLICABLE
    refs = surface(observation["refs"], identity.refs, success, "refs", semantic=True)
    tree = surface(observation["gentufa_tree"], identity.gentufa_tree, success, "Gentufa tree")
    rendered_json = surface(observation["gentufa_json"], identity.gentufa_json, success, "Gentufa JSON")
    values = dict(zip(SURFACES, (
        "success" if success else "failure", owner, raw, digest(json_text(diagnostics)),
        refs.status, refs.raw, refs.error, tree.raw, rendered_json.raw,
    ), strict=True))
    return Snapshot(stage, values, tuple(diagnostics))


def changed_surfaces(before: Snapshot, after: Snapshot) -> frozenset[str]:
    return frozenset(key for key in SURFACES if before.values[key] != after.values[key])


def transition(before: Snapshot, after: Snapshot, stage: str,
               disposition: Mapping[str, Any]) -> frozenset[str]:
    require(stage in {"c-a", "c-b"}, f"no transition rules for {stage}")
    predecessor = "base" if stage == "c-a" else "c-a"
    require(before.stage == predecessor and after.stage == stage,
            f"{stage} requires a measured {predecessor} predecessor and its own successor")
    changed = changed_surfaces(before, after)
    old_status, new_status = before.values["status"], after.values["status"]
    exact_keys(disposition, {"kind", "reviewed_surfaces", "reason"}, "disposition")
    reviewed = disposition["reviewed_surfaces"]
    require(isinstance(reviewed, list) and all(isinstance(item, str) for item in reviewed) and
            len(reviewed) == len(set(reviewed)), "invalid or duplicate reviewed surface")
    require(set(reviewed) == changed, f"reviewed surfaces do not match actual delta {sorted(changed)}")
    if stage == "c-a":
        require(old_status == "success" and before.owner == "EmptyLinkedSumti", "base is not the frozen Empty predecessor")
        require(new_status == "success", "C-a rejection is not authorized")
        if after.owner == "EmptyLinkedSumti":
            require(not changed and disposition["kind"] == "unchanged", "Empty must preserve all nine surfaces")
        else:
            require(after.owner == "FullLinkedTerm", "C-a has an unapproved owner")
            require("syntax_raw_full_tree_sha256" in changed, "owner changed without a changed strict tree")
            require(disposition["kind"] == "d1-reowner-preserved", "Full reownership requires a D1 manual disposition")
            nonempty_text(disposition["reason"], "manual reownership reason")
    elif stage == "c-b":
        require(old_status == "success", "C-b needs a successful measured C-a predecessor")
        if before.owner == "FullLinkedTerm":
            require(not changed and disposition["kind"] == "reowner-preserved", "C-b changed a preserved Full payload")
        else:
            require(before.owner == "EmptyLinkedSumti", "C-b predecessor has an unapproved owner")
            require(new_status == "failure" and after.error_count > 0, "Empty removal must produce actual strict failure")
            require("syntax_diagnostics_sha256" in changed, "success/failure diagnostics did not change")
            require(disposition["kind"] == "empty-route-removed", "removal needs an explicit manual disposition")
            nonempty_text(disposition["reason"], "manual removal reason")
    else:
        raise ValidationError(f"no transition rules for {stage}")
    return changed


def population(identities: Mapping[str, Identity], observations: list[Mapping[str, Any]],
               stage: str) -> dict[str, Snapshot]:
    result = {}
    for observation in observations:
        require(isinstance(observation, Mapping), "expected an observation object")
        case_id = observation.get("id")
        require(isinstance(case_id, str) and case_id in identities, f"unknown observation ID {case_id}")
        require(case_id not in result, f"duplicate observation ID {case_id}")
        result[case_id] = snapshot(identities[case_id], observation, stage)
    require(result.keys() == identities.keys(), f"missing observations: {identities.keys() - result.keys()}")
    return result


def read_fixture(root: Path, path: str) -> Mapping[str, Any]:
    nonempty_text(path, "fixture path")
    relative = Path(path)
    require(not relative.is_absolute() and ".." not in relative.parts,
            f"fixture path must be repository-relative: {path}")
    try:
        fixture_path = root / relative
        fixture = tomllib.loads(fixture_path.read_text(encoding="utf-8"))
        filenames = [key for key in ("lojban-filename", "lojban_filename") if key in fixture]
        require(int("lojban" in fixture) + len(filenames) == 1,
                f"{path}: declare exactly one inline or file-backed source")
        if filenames:
            filename = Path(nonempty_text(fixture[filenames[0]], "source filename"))
            require(not filename.is_absolute() and ".." not in filename.parts and filename.parts,
                    f"{path}: source filename must be a relative child path")
            fixture["lojban"] = (fixture_path.parent / filename).read_text(encoding="utf-8")
        require(isinstance(fixture["lojban"], str), f"{path}: source must be text")
        require(fixture.get("dialect") is None or isinstance(fixture["dialect"], str),
                f"{path}: dialect must be text")
        return fixture
    except (OSError, ValueError) as error:
        raise ValidationError(f"cannot read fixture {path}: {error}") from error


def load_base(data: Mapping[str, Any], baseline_root: Path) -> tuple[dict[str, Identity], dict[str, Snapshot]]:
    """Read ordinary committed base observations and source values from a Git archive.

    The retained base contains measured output cells, not diagnostic arrays. No array or
    derived count is reconstructed from a digest. Successor arrays come from the collector.
    """
    exact_keys(data, {"semantic_base", "rows"}, "base data")
    require(isinstance(data["semantic_base"], str) and
            re.fullmatch(r"[0-9a-f]{40}", data["semantic_base"]) is not None, "invalid semantic base commit")
    require(isinstance(data["rows"], list) and bool(data["rows"]), "missing base rows")
    identities = {}
    snapshots = {}
    paths = set()
    for row in data["rows"]:
        exact_keys(row, {"id", "path", "dialect", "target", "coverage", "base"}, "base row")
        case_id = nonempty_text(row["id"], "fixture ID")
        require(case_id not in identities, f"duplicate base ID {case_id}")
        path = nonempty_text(row["path"], "fixture path")
        require(path not in paths, f"duplicate base path {path}")
        paths.add(path)
        fixture = read_fixture(baseline_root, path)
        source = nonempty_text(fixture.get("lojban"), f"{case_id}: source")
        target = row["target"]
        exact_keys(target, {"marker", "byte_start", "byte_end"}, "base anchor")
        require(target["marker"] in {"be", "bei"}, "invalid base marker")
        start, end = target["byte_start"], target["byte_end"]
        require(all(isinstance(value, int) and not isinstance(value, bool) for value in (start, end)) and
                0 <= start < end <= len(source.encode("utf-8")), "invalid base marker extent")
        coverage = row["coverage"]
        exact_keys(coverage, {"refs", "gentufa_tree", "gentufa_json"}, "base coverage")
        require(all(isinstance(value, bool) for value in coverage.values()), "coverage must be boolean")
        identity = Identity(case_id, path, source, row["dialect"], target, **coverage)
        identity.check_fixture(baseline_root)
        values = row["base"]
        exact_keys(values, set(SURFACES), "base surfaces")
        require(values["status"] == "success", "frozen base must have strict success")
        require(values["owner"] == json_text({"kind": "syntax-node", "variant": "EmptyLinkedSumti"}),
                "frozen base must have an Empty owner")
        required_hashes = {"syntax_raw_full_tree_sha256", "syntax_diagnostics_sha256"}
        for surface_name in ("gentufa_tree", "gentufa_json"):
            key = f"{surface_name}_sha256"
            if coverage[surface_name]:
                required_hashes.add(key)
            else:
                require(values[key] == INAPPLICABLE, f"{case_id}: changed base {surface_name} coverage")
        if coverage["refs"]:
            require(values["semantics_refs_status"] in {"success", "failure"}, "missing base refs status")
            success = values["semantics_refs_status"] == "success"
            required_hashes.add("semantics_refs_raw_sha256" if success else "semantics_refs_error_sha256")
            require(values["semantics_refs_error_sha256" if success else "semantics_refs_raw_sha256"] == INAPPLICABLE,
                    "base refs has contradictory raw/error surfaces")
        else:
            require(all(values[key] == INAPPLICABLE for key in SURFACES if key.startswith("semantics_refs_")),
                    "changed base refs coverage")
        for key in required_hashes:
            require(isinstance(values[key], str) and re.fullmatch(r"[0-9a-f]{64}", values[key]) is not None,
                    f"{case_id}: missing measured {key}")
        identities[case_id] = identity
        snapshots[case_id] = Snapshot("base", values, None)
    return identities, snapshots


def stage_rows(identities: Mapping[str, Identity], base: Mapping[str, Snapshot],
               observations: Mapping[str, list[Mapping[str, Any]]],
               dispositions: Mapping[str, Mapping[str, Mapping[str, Any]]],
               stage: str) -> list[list[str]]:
    """Derive the 32-column report from collector output and reviewed transitions.

    The TSV is a report, not another canonical observation input. An unmeasured future stage
    is visibly UNPROBED; missing observations at or before the requested stage are errors.
    """
    require(stage in {"c-a", "c-b"}, "ledger stage must be c-a or c-b")
    required = STAGES[1:STAGES.index(stage) + 1]
    exact_keys(observations, set(required), "measured stages")
    exact_keys(dispositions, set(required), "transition stages")
    require(base.keys() == identities.keys(), "base population differs from identities")
    measured = {"base": base}
    for successor in required:
        measured[successor] = population(identities, observations[successor], successor)
        if successor == "c-b" and ALICE_ID in identities:
            alice = next(row for row in observations[successor] if row["id"] == ALICE_ID)
            require_recovered_capture(alice, identities[ALICE_ID])
        exact_keys(dispositions[successor], set(identities), f"{successor} dispositions")
        predecessor = STAGES[STAGES.index(successor) - 1]
        for case_id in identities:
            if case_id == ALICE_ID and successor == "c-a":
                require(measured[successor][case_id].owner == "EmptyLinkedSumti",
                        "Alice C-a reownership requires a new ruling; the generic Full transition is not a waiver")
            transition(measured[predecessor][case_id], measured[successor][case_id], successor,
                       dispositions[successor][case_id])
    rows = []
    for case_id, identity in identities.items():
        row = [case_id, identity.path, json_text(identity.dialect)]
        for measured_stage in STAGES:
            row.extend(measured[measured_stage][case_id].values[key] if measured_stage in measured else UNPROBED
                       for key in SURFACES)
        row.extend(json_text(dispositions[successor][case_id]) if successor in dispositions else UNPROBED
                   for successor in STAGES[1:])
        rows.append(row)
    return rows


def require_recovered_capture(observation: Mapping[str, Any], identity: Identity) -> None:
    """Alice's narrow C-b ruling requires a real recovery oracle as well as strict failure.

This checks capture completeness, not approval of the recovery or the later four
paired JAI extents. Those changes still need the prescribed manual review.
"""
    recovered = observation.get("recovered")
    exact_keys(recovered, {"status", "value"}, "Alice recovery capture")
    require(recovered["status"] == "success", "Alice requires a captured recovered result")
    value = recovered["value"]
    exact_keys(value, {"parser_status", "raw", "errors", "diagnostics", "tree", "target"},
               "Alice recovered result")
    require(isinstance(value["parser_status"], str) and value["parser_status"] in {"success", "failure"},
            "Alice recovered status is not measured")
    nonempty_text(value["raw"], "Alice recovered full tree")
    require(isinstance(value["errors"], list) and all(isinstance(item, str) for item in value["errors"]),
            "Alice recovered errors were not captured")
    validate_diagnostics(value["diagnostics"], identity.source, "Alice recovered diagnostics")
    exact_keys(value["tree"], {"valid-tokens", "recovery-items"}, "Alice recovery projection")
    require(all(isinstance(item, list) for item in value["tree"].values()), "Alice incomplete recovery projection")
    require(isinstance(value["target"], Mapping) and value["target"].get("status") in {"found", "missing"},
            "Alice recovery target must be observed, not omitted or ambiguous")


def render_tsv(rows: list[list[str]]) -> str:
    header = ["id", "path", "dialect"]
    header.extend(f"{stage}.{surface}" for stage in STAGES for surface in SURFACES)
    header.extend(f"{stage}.disposition" for stage in STAGES[1:])
    require(all(len(row) == len(header) == 32 for row in rows), "ledger report row width differs")
    stream = io.StringIO(newline="")
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow(header)
    writer.writerows(rows)
    return stream.getvalue()


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValidationError(f"cannot read JSON {path}: {error}") from error


def read_observations(path: Path) -> list[Mapping[str, Any]]:
    try:
        with path.open(encoding="utf-8") as stream:
            return [json.loads(line) for line in stream if line.strip()]
    except (OSError, ValueError) as error:
        raise ValidationError(f"cannot read collector output {path}: {error}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", choices=STAGES[1:], required=True)
    parser.add_argument("--base-data", type=Path, default=Path(__file__).with_name("data") / "links-jai-empty-base.json")
    parser.add_argument("--baseline-root", type=Path, required=True,
                        help="Repository-root Git archive at the semantic base, including file-backed sources")
    parser.add_argument("--candidate-root", type=Path, default=Path("."))
    parser.add_argument("--c-a-observations", type=Path, required=True)
    parser.add_argument("--c-b-observations", type=Path)
    parser.add_argument("--dispositions", type=Path, required=True,
                        help="Stage-to-ID map of individually reviewed transition dispositions")
    args = parser.parse_args()
    try:
        identities, base = load_base(read_json(args.base_data), args.baseline_root)
        for identity in identities.values():
            identity.check_fixture(args.candidate_root)
        observations = {"c-a": read_observations(args.c_a_observations)}
        if args.c_b_observations is not None:
            observations["c-b"] = read_observations(args.c_b_observations)
        rows = stage_rows(identities, base, observations, read_json(args.dispositions), args.stage)
        sys.stdout.write(render_tsv(rows))
    except ValidationError as error:
        print(f"links-jai ledger: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
