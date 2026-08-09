from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from dataclasses import dataclass
from pathlib import Path

from tools import cli_release


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SCRIPT = REPOSITORY_ROOT / ".github" / "scripts" / "prepare_release_draft.sh"
VERSION = "1.2.3"
TAG = f"v{VERSION}"
SHA = "a" * 40
REPOSITORY = "example/jbotci"
RELEASE_ID = 41
GENERATED_NAME = "Generated release title"
GENERATED_BODY = "Generated release body"
UNSET = object()


GH_STUB = r'''#!/usr/bin/env python3
import hashlib
import json
import os
import sys
from pathlib import Path
from urllib.parse import parse_qs, unquote, urlparse


def argument_value(arguments, flag):
    try:
        return arguments[arguments.index(flag) + 1]
    except (ValueError, IndexError):
        return None


arguments = sys.argv[1:]
if not arguments or arguments[0] != "api":
    print("stub gh accepts only api", file=sys.stderr)
    raise SystemExit(2)

method = (argument_value(arguments, "--method") or "GET").upper()
input_path = argument_value(arguments, "--input")
endpoint = None
skip_next = False
for value in arguments[1:]:
    if skip_next:
        skip_next = False
        continue
    if value in {"--method", "--input", "--header"}:
        skip_next = True
        continue
    if value.startswith("-"):
        continue
    endpoint = value
    break
if endpoint is None:
    print(f"missing gh endpoint: {arguments!r}", file=sys.stderr)
    raise SystemExit(2)

if endpoint.endswith("/releases/generate-notes"):
    operation = "generate_notes"
elif endpoint.endswith("/git/refs") and method == "POST":
    operation = "create_tag"
elif endpoint.endswith("/releases") and method == "POST":
    operation = "create_release"
elif "/git/matching-refs/tags/" in endpoint:
    operation = "list_tags"
elif endpoint.endswith("/releases?per_page=100"):
    operation = "list_releases"
elif endpoint.endswith("/assets?per_page=100"):
    operation = "list_assets"
elif "/releases/assets/" in endpoint and method == "DELETE":
    operation = "delete_asset"
elif endpoint.startswith("https://uploads.github.com/") and method == "POST":
    operation = "upload_asset"
elif method == "PATCH":
    operation = "patch"
else:
    print(f"unhandled gh request: {arguments!r}", file=sys.stderr)
    raise SystemExit(2)

scenario_path = Path(os.environ["GH_STUB_SCENARIO"])
scenario = json.loads(scenario_path.read_text(encoding="utf-8"))
request_json = None
if input_path and operation != "upload_asset":
    request_json = json.loads(Path(input_path).read_text(encoding="utf-8"))

call = {
    "operation": operation,
    "method": method,
    "endpoint": endpoint,
    "arguments": arguments,
    "request_json": request_json,
}
with Path(os.environ["GH_STUB_LOG"]).open("a", encoding="utf-8") as stream:
    stream.write(json.dumps(call, sort_keys=True) + "\n")

counts = scenario.setdefault("counts", {})
index = counts.get(operation, 0)
counts[operation] = index + 1
responses = scenario.get("responses", {}).get(operation)
response = None
if responses:
    response = responses[min(index, len(responses) - 1)]

if response is not None and "__exit__" in response:
    scenario_path.write_text(json.dumps(scenario), encoding="utf-8")
    print(response.get("__stderr__", "stub API failure"), file=sys.stderr)
    raise SystemExit(response["__exit__"])

if response is not None:
    payload = response["__payload__"]
elif operation == "generate_notes":
    payload = {"name": "Generated release title", "body": "Generated release body"}
elif operation == "create_tag":
    payload = {
        "ref": request_json["ref"],
        "object": {"type": "commit", "sha": request_json["sha"]},
    }
elif operation == "create_release":
    payload = {
        **request_json,
        "id": scenario.get("release_id", 41),
        "assets": [],
        "html_url": "https://github.com/example/jbotci/releases/tag/untagged-draft",
    }
elif operation == "delete_asset":
    payload = None
elif operation == "upload_asset":
    parsed = urlparse(endpoint)
    name = unquote(parse_qs(parsed.query)["name"][0])
    contents = Path(input_path).read_bytes()
    asset_id = scenario.setdefault("next_asset_id", 1000)
    scenario["next_asset_id"] = asset_id + 1
    payload = {
        "id": asset_id,
        "name": name,
        "state": "uploaded",
        "size": len(contents),
        "digest": f"sha256:{hashlib.sha256(contents).hexdigest()}",
    }
else:
    print(f"no response configured for {operation}", file=sys.stderr)
    raise SystemExit(2)

scenario_path.write_text(json.dumps(scenario), encoding="utf-8")
if payload is not None:
    json.dump(payload, sys.stdout)
    sys.stdout.write("\n")
'''


DATE_STUB = r'''#!/usr/bin/env python3
import os
from pathlib import Path

clock = Path(os.environ["POLL_STUB_CLOCK"])
print(clock.read_text(encoding="ascii").strip())
'''


SLEEP_STUB = r'''#!/usr/bin/env python3
import os
import sys
from pathlib import Path

seconds = int(sys.argv[1])
clock = Path(os.environ["POLL_STUB_CLOCK"])
now = int(clock.read_text(encoding="ascii").strip())
advance = int(os.environ.get("POLL_STUB_ADVANCE_SECONDS", str(seconds)))
clock.write_text(str(now + advance), encoding="ascii")
with Path(os.environ["POLL_STUB_SLEEP_LOG"]).open("a", encoding="ascii") as stream:
    stream.write(f"{seconds}\n")
'''


def queued(payload: object) -> dict[str, object]:
    return {"__payload__": payload}


def failed(message: str = "HTTP 500") -> dict[str, object]:
    return {"__exit__": 1, "__stderr__": message}


def paginated(*pages: list[dict[str, object]]) -> list[list[dict[str, object]]]:
    return list(pages)


def release(
    *,
    release_id: int = RELEASE_ID,
    name: str = GENERATED_NAME,
    body: str = GENERATED_BODY,
    sha: str = SHA,
    draft: bool = True,
    prerelease: bool = False,
) -> dict[str, object]:
    return {
        "id": release_id,
        "tag_name": TAG,
        "target_commitish": sha,
        "draft": draft,
        "prerelease": prerelease,
        "name": name,
        "body": body,
        "html_url": "https://github.com/example/jbotci/releases/tag/untagged-draft",
        "published_at": None,
    }


def initial_notes_base() -> dict[str, object]:
    return {
        "id": 7,
        "tag_name": "pr-artifacts",
        "target_commitish": "b" * 40,
        "draft": False,
        "prerelease": True,
        "name": "PR artifacts",
        "body": "Rolling artifacts",
        "html_url": "https://github.com/example/jbotci/releases/tag/pr-artifacts",
        "published_at": "2026-01-01T00:00:00Z",
    }


def exact_tag() -> dict[str, object]:
    return {
        "ref": f"refs/tags/{TAG}",
        "object": {"type": "commit", "sha": SHA},
    }


@dataclass(frozen=True)
class RunResult:
    process: subprocess.CompletedProcess[str]
    calls: list[dict[str, object]]
    sleeps: list[int]
    state_dir: Path
    summary: Path


class ReleaseDraftScriptTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.bin_dir = self.root / "bin"
        self.bin_dir.mkdir()
        self._write_executable("gh", GH_STUB)
        self._write_executable("date", DATE_STUB)
        self._write_executable("sleep", SLEEP_STUB)

        self.release_dir = self.root / "release"
        self.release_dir.mkdir()
        for index, name in enumerate(cli_release.asset_names(VERSION)):
            (self.release_dir / name).write_bytes(
                f"asset {index}: {name}\n".encode("utf-8")
            )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def _write_executable(self, name: str, contents: str) -> None:
        path = self.bin_dir / name
        path.write_text(textwrap.dedent(contents), encoding="utf-8")
        path.chmod(0o755)

    def uploaded_assets(
        self,
        *,
        first_state: str = "uploaded",
        first_digest: str | None | object = UNSET,
    ) -> list[dict[str, object]]:
        assets = []
        for index, name in enumerate(cli_release.asset_names(VERSION)):
            contents = (self.release_dir / name).read_bytes()
            digest: str | None = f"sha256:{hashlib.sha256(contents).hexdigest()}"
            if index == 0:
                state = first_state
                if first_digest is not UNSET:
                    digest = first_digest  # type: ignore[assignment]
            else:
                state = "uploaded"
            assets.append(
                {
                    "id": 1000 + index,
                    "name": name,
                    "state": state,
                    "size": len(contents),
                    "digest": digest,
                }
            )
        return assets

    def successful_create_responses(
        self, *, delayed_release_visibility: bool = False
    ) -> dict[str, list[dict[str, object]]]:
        base = initial_notes_base()
        final_assets = self.uploaded_assets()
        release_pages = [queued(paginated([base]))]
        if delayed_release_visibility:
            release_pages.append(queued(paginated([base])))
        release_pages.extend(
            [
                queued(paginated([base, release()])),
                queued(paginated([base, release()])),
            ]
        )
        return {
            "list_releases": release_pages,
            "list_tags": [
                queued(paginated([])),
                queued(paginated([exact_tag()])),
                queued(paginated([exact_tag()])),
            ],
            "list_assets": [
                queued(paginated([])),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ],
        }

    def successful_resume_responses(
        self,
        *,
        release_value: dict[str, object] | None = None,
        asset_pages: list[dict[str, object]] | None = None,
    ) -> dict[str, list[dict[str, object]]]:
        release_value = release_value or release()
        final_assets = self.uploaded_assets()
        return {
            "list_releases": [
                queued(paginated([initial_notes_base(), release_value])),
                queued(paginated([initial_notes_base(), release_value])),
            ],
            "list_tags": [queued(paginated([exact_tag()]))],
            "list_assets": asset_pages
            or [
                queued(paginated([])),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ],
        }

    def run_script(
        self,
        responses: dict[str, list[dict[str, object]]],
        *,
        case: str,
        clock_advance_seconds: int = 20,
    ) -> RunResult:
        case_root = self.root / case
        case_root.mkdir()
        scenario_path = case_root / "scenario.json"
        log_path = case_root / "gh.jsonl"
        sleep_log = case_root / "sleeps.txt"
        clock_path = case_root / "clock.txt"
        state_dir = case_root / "state"
        summary = case_root / "summary.md"
        scenario_path.write_text(
            json.dumps(
                {
                    "responses": responses,
                    "counts": {},
                    "release_id": RELEASE_ID,
                    "next_asset_id": 1000,
                }
            ),
            encoding="utf-8",
        )
        log_path.write_text("", encoding="utf-8")
        sleep_log.write_text("", encoding="ascii")
        clock_path.write_text("1000", encoding="ascii")

        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{self.bin_dir}:{environment['PATH']}",
                "GH_STUB_SCENARIO": str(scenario_path),
                "GH_STUB_LOG": str(log_path),
                "POLL_STUB_CLOCK": str(clock_path),
                "POLL_STUB_SLEEP_LOG": str(sleep_log),
                "POLL_STUB_ADVANCE_SECONDS": str(clock_advance_seconds),
            }
        )
        process = subprocess.run(
            [
                str(SCRIPT),
                "--repository",
                REPOSITORY,
                "--sha",
                SHA,
                "--tag",
                TAG,
                "--version",
                VERSION,
                "--release-dir",
                str(self.release_dir),
                "--state-dir",
                str(state_dir),
                "--summary",
                str(summary),
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
            text=True,
            capture_output=True,
            timeout=20,
            check=False,
        )
        calls = [
            json.loads(line)
            for line in log_path.read_text(encoding="utf-8").splitlines()
        ]
        sleeps = [
            int(line)
            for line in sleep_log.read_text(encoding="ascii").splitlines()
        ]
        return RunResult(process, calls, sleeps, state_dir, summary)

    @staticmethod
    def operations(result: RunResult) -> list[str]:
        return [str(call["operation"]) for call in result.calls]

    def assert_success(self, result: RunResult) -> None:
        self.assertEqual(
            result.process.returncode,
            0,
            msg=f"stdout:\n{result.process.stdout}\nstderr:\n{result.process.stderr}",
        )

    def test_delayed_created_release_list_visibility_retries(self) -> None:
        result = self.run_script(
            self.successful_create_responses(delayed_release_visibility=True),
            case="delayed-created-release",
        )

        self.assert_success(result)
        self.assertEqual(result.sleeps, [2])
        operations = self.operations(result)
        self.assertEqual(operations.count("create_release"), 1)
        self.assertLess(operations.index("create_release"), operations.index("upload_asset"))

    def test_bad_create_response_fails_before_upload(self) -> None:
        responses = self.successful_create_responses()
        responses["create_release"] = [
            queued(
                {
                    "id": RELEASE_ID,
                    "tag_name": TAG,
                    "target_commitish": "f" * 40,
                    "draft": True,
                    "prerelease": False,
                    "name": GENERATED_NAME,
                    "body": GENERATED_BODY,
                    "assets": [],
                }
            )
        ]
        result = self.run_script(responses, case="bad-create-response")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("release creation response", result.process.stderr)
        self.assertNotIn("upload_asset", self.operations(result))

    def test_bad_tag_create_response_fails_before_release_creation(self) -> None:
        responses = self.successful_create_responses()
        responses["create_tag"] = [
            queued(
                {
                    "ref": f"refs/tags/{TAG}",
                    "object": {"type": "tag", "sha": SHA},
                }
            )
        ]
        result = self.run_script(responses, case="bad-tag-create-response")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("tag creation response", result.process.stderr)
        self.assertNotIn("create_release", self.operations(result))
        self.assertNotIn("upload_asset", self.operations(result))

    def test_created_release_visibility_has_a_fixed_bound(self) -> None:
        base = initial_notes_base()
        responses = self.successful_create_responses()
        responses["list_releases"] = [queued(paginated([base]))]
        result = self.run_script(
            responses,
            case="bounded-created-release",
        )

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("did not converge within 60 seconds", result.process.stderr)
        self.assertEqual(result.sleeps, [2, 2, 2])
        self.assertNotIn("upload_asset", self.operations(result))

    def test_duplicate_release_is_immediately_terminal_without_sleep(self) -> None:
        responses = {
            "list_releases": [
                queued(
                    paginated(
                        [
                            initial_notes_base(),
                            release(release_id=RELEASE_ID),
                            release(release_id=RELEASE_ID + 1),
                        ]
                    )
                )
            ]
        }
        result = self.run_script(responses, case="duplicate-release")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("multiple releases", result.process.stderr)
        self.assertEqual(result.sleeps, [])

    def test_foreign_created_release_id_is_immediately_terminal_without_sleep(self) -> None:
        responses = self.successful_create_responses()
        responses["list_releases"] = [
            queued(paginated([initial_notes_base()])),
            queued(
                paginated(
                    [initial_notes_base(), release(release_id=RELEASE_ID + 1)]
                )
            ),
        ]
        result = self.run_script(responses, case="foreign-release-id")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("foreign release id", result.process.stderr)
        self.assertEqual(result.sleeps, [])
        self.assertNotIn("upload_asset", self.operations(result))

    def test_resume_never_creates_or_patches_and_preserves_text(self) -> None:
        resumed_release = release(
            name="Owner-edited title",
            body="Owner-edited body\nwith another line",
        )
        result = self.run_script(
            self.successful_resume_responses(release_value=resumed_release),
            case="resume-preserves-text",
        )

        self.assert_success(result)
        operations = self.operations(result)
        self.assertNotIn("create_release", operations)
        self.assertNotIn("create_tag", operations)
        self.assertNotIn("generate_notes", operations)
        self.assertNotIn("patch", operations)
        preserved = json.loads(
            (result.state_dir / "preserved-text.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            preserved,
            {"name": resumed_release["name"], "body": resumed_release["body"]},
        )

    def test_unmanaged_seventh_asset_fails_before_any_mutation(self) -> None:
        unmanaged = [
            *self.uploaded_assets(),
            {
                "id": 9999,
                "name": "unmanaged.txt",
                "state": "uploaded",
                "size": 1,
                "digest": f"sha256:{'0' * 64}",
            },
        ]
        responses = self.successful_resume_responses(
            asset_pages=[queued(paginated(unmanaged))]
        )
        result = self.run_script(responses, case="unmanaged-asset")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("unmanaged asset", result.process.stderr)
        operations = self.operations(result)
        self.assertNotIn("delete_asset", operations)
        self.assertNotIn("upload_asset", operations)

    def test_resume_deletes_only_managed_asset_ids_before_upload(self) -> None:
        existing_assets = self.uploaded_assets()[:2]
        final_assets = self.uploaded_assets()
        responses = self.successful_resume_responses(
            asset_pages=[
                queued(paginated(existing_assets)),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ]
        )
        result = self.run_script(responses, case="managed-asset-clobber")

        self.assert_success(result)
        delete_calls = [
            call for call in result.calls if call["operation"] == "delete_asset"
        ]
        self.assertEqual(
            [call["endpoint"] for call in delete_calls],
            [
                f"repos/{REPOSITORY}/releases/assets/{asset['id']}"
                for asset in existing_assets
            ],
        )
        operations = self.operations(result)
        self.assertLess(
            max(index for index, value in enumerate(operations) if value == "delete_asset"),
            operations.index("upload_asset"),
        )

    def test_starter_asset_retries_until_uploaded(self) -> None:
        starter_assets = self.uploaded_assets(first_state="starter", first_digest=None)
        uploaded_assets = self.uploaded_assets()
        responses = self.successful_resume_responses(
            asset_pages=[
                queued(paginated([])),
                queued(paginated(starter_assets)),
                queued(paginated(uploaded_assets)),
                queued(paginated(uploaded_assets)),
            ]
        )
        result = self.run_script(responses, case="starter-asset")

        self.assert_success(result)
        self.assertEqual(result.sleeps, [2])

    def test_uploaded_wrong_digest_fails_immediately(self) -> None:
        wrong_assets = self.uploaded_assets(first_digest=f"sha256:{'0' * 64}")
        responses = self.successful_resume_responses(
            asset_pages=[
                queued(paginated([])),
                queued(paginated(wrong_assets)),
            ]
        )
        result = self.run_script(responses, case="wrong-uploaded-digest")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("wrong non-null digest", result.process.stderr)
        self.assertEqual(result.sleeps, [])

    def test_exact_tag_without_visible_release_requires_consecutive_zeroes(self) -> None:
        base = initial_notes_base()
        final_assets = self.uploaded_assets()
        responses = {
            "list_releases": [
                queued(paginated([base])),
                queued(paginated([base])),
                queued(paginated([base, release()])),
                queued(paginated([base, release()])),
            ],
            "list_tags": [queued(paginated([exact_tag()]))],
            "list_assets": [
                queued(paginated([])),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ],
        }
        result = self.run_script(responses, case="tag-only-recovery")

        self.assert_success(result)
        operations = self.operations(result)
        create_index = operations.index("create_release")
        self.assertEqual(operations[:create_index].count("list_releases"), 2)
        self.assertEqual(result.sleeps, [2])
        self.assertNotIn("create_tag", operations)

    def test_tag_only_wait_switches_to_a_draft_that_appears(self) -> None:
        base = initial_notes_base()
        appearing_release = release(
            name="Preserved appeared title", body="Preserved appeared body"
        )
        final_assets = self.uploaded_assets()
        responses = {
            "list_releases": [
                queued(paginated([base])),
                queued(paginated([base, appearing_release])),
                queued(paginated([base, appearing_release])),
            ],
            "list_tags": [queued(paginated([exact_tag()]))],
            "list_assets": [
                queued(paginated([])),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ],
        }
        result = self.run_script(responses, case="tag-only-draft-appears")

        self.assert_success(result)
        operations = self.operations(result)
        self.assertEqual(result.sleeps, [2])
        self.assertNotIn("generate_notes", operations)
        self.assertNotIn("create_release", operations)
        self.assertNotIn("create_tag", operations)

    def test_absent_tag_creates_without_release_absence_wait(self) -> None:
        result = self.run_script(
            self.successful_create_responses(),
            case="no-tag-no-absence-wait",
        )

        self.assert_success(result)
        operations = self.operations(result)
        create_index = operations.index("create_release")
        self.assertEqual(operations[:create_index].count("list_releases"), 1)
        self.assertEqual(result.sleeps, [])
        self.assertEqual(operations.count("create_tag"), 1)
        create_tag_call = next(
            call for call in result.calls if call["operation"] == "create_tag"
        )
        self.assertEqual(
            create_tag_call["request_json"],
            {"ref": f"refs/tags/{TAG}", "sha": SHA},
        )
        create_release_call = next(
            call for call in result.calls if call["operation"] == "create_release"
        )
        self.assertEqual(
            create_release_call["request_json"],
            {
                "tag_name": TAG,
                "target_commitish": SHA,
                "name": GENERATED_NAME,
                "body": GENERATED_BODY,
                "draft": True,
                "prerelease": False,
                "generate_release_notes": False,
            },
        )
        upload_calls = [
            call for call in result.calls if call["operation"] == "upload_asset"
        ]
        self.assertEqual(len(upload_calls), 6)
        self.assertTrue(
            all(f"/releases/{RELEASE_ID}/assets?name=" in call["endpoint"] for call in upload_calls)
        )

    def test_release_match_on_a_later_page_resumes(self) -> None:
        final_assets = self.uploaded_assets()
        responses = {
            "list_releases": [
                queued(paginated([initial_notes_base()], [release()])),
                queued(paginated([initial_notes_base()], [release()])),
            ],
            "list_tags": [
                queued(
                    paginated(
                        [
                            {
                                "ref": f"refs/tags/{TAG}-other",
                                "object": {"type": "commit", "sha": "c" * 40},
                            }
                        ],
                        [exact_tag()],
                    )
                )
            ],
            "list_assets": [
                queued(paginated([])),
                queued(paginated(final_assets)),
                queued(paginated(final_assets)),
            ],
        }
        result = self.run_script(responses, case="later-page-match")

        self.assert_success(result)
        self.assertNotIn("create_release", self.operations(result))

    def test_api_error_fails_closed_without_retry(self) -> None:
        responses = {"list_releases": [failed()]}
        result = self.run_script(responses, case="api-500")

        self.assertNotEqual(result.process.returncode, 0)
        self.assertIn("HTTP 500", result.process.stderr)
        self.assertEqual(result.sleeps, [])
        self.assertEqual(self.operations(result), ["list_releases"])


if __name__ == "__main__":
    unittest.main()
