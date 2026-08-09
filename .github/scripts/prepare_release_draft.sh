#!/usr/bin/env bash

set -euo pipefail

die() {
  echo "prepare_release_draft.sh: $*" >&2
  exit 1
}

usage() {
  cat >&2 <<'EOF'
usage: prepare_release_draft.sh \
  --repository OWNER/REPO \
  --sha COMMIT_SHA \
  --tag TAG \
  --version VERSION \
  --release-dir DIRECTORY \
  --state-dir DIRECTORY \
  --summary FILE
EOF
  exit 2
}

repository=""
release_sha=""
release_tag=""
release_version=""
release_dir=""
state_dir=""
summary_file=""

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --repository)
      [[ "$#" -ge 2 ]] || usage
      repository="$2"
      shift 2
      ;;
    --sha)
      [[ "$#" -ge 2 ]] || usage
      release_sha="$2"
      shift 2
      ;;
    --tag)
      [[ "$#" -ge 2 ]] || usage
      release_tag="$2"
      shift 2
      ;;
    --version)
      [[ "$#" -ge 2 ]] || usage
      release_version="$2"
      shift 2
      ;;
    --release-dir)
      [[ "$#" -ge 2 ]] || usage
      release_dir="$2"
      shift 2
      ;;
    --state-dir)
      [[ "$#" -ge 2 ]] || usage
      state_dir="$2"
      shift 2
      ;;
    --summary)
      [[ "$#" -ge 2 ]] || usage
      summary_file="$2"
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

[[ "${repository}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] ||
  die "repository must be OWNER/REPO"
[[ "${release_sha}" =~ ^[0-9a-f]{40}$ ]] ||
  die "release SHA must be exactly 40 lowercase hexadecimal characters"
[[ -n "${release_version}" ]] || die "release version must not be empty"
[[ "${release_tag}" == "v${release_version}" ]] ||
  die "release tag must be v<version>"
[[ -d "${release_dir}" && ! -L "${release_dir}" ]] ||
  die "release directory must be a non-symlink directory"
[[ -n "${state_dir}" && -n "${summary_file}" ]] || usage

readonly poll_timeout_seconds=60
readonly poll_interval_seconds=2

if [[ -e "${state_dir}" ]]; then
  [[ -d "${state_dir}" && ! -L "${state_dir}" ]] ||
    die "state directory must be a non-symlink directory"
  if find "${state_dir}" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
    die "state directory must be empty"
  fi
else
  mkdir -p "${state_dir}"
fi

for command_name in cat cmp date find gh grep jq mkdir python3 sha256sum sleep sort stat; do
  command -v "${command_name}" > /dev/null ||
    die "required command is unavailable: ${command_name}"
done

jq -rn --arg value "${release_tag}" '$value | @uri' > "${state_dir}/encoded-tag.txt"
IFS= read -r encoded_tag < "${state_dir}/encoded-tag.txt"
[[ -n "${encoded_tag}" ]] || die "release tag could not be URL-encoded"

read_clock() {
  date +%s > "${state_dir}/clock.txt"
  IFS= read -r now_epoch < "${state_dir}/clock.txt"
  [[ "${now_epoch}" =~ ^[0-9]+$ ]] || die "date returned an invalid epoch"
}

start_poll_deadline() {
  local description="$1"
  poll_description="${description}"
  read_clock
  poll_deadline=$((now_epoch + poll_timeout_seconds))
}

wait_for_next_poll() {
  read_clock
  if (( now_epoch >= poll_deadline )); then
    die "${poll_description} did not converge within ${poll_timeout_seconds} seconds"
  fi
  sleep "${poll_interval_seconds}"
  read_clock
  if (( now_epoch > poll_deadline )); then
    die "${poll_description} did not converge within ${poll_timeout_seconds} seconds"
  fi
}

flatten_paginated_arrays() {
  local pages_path="$1"
  local output_path="$2"
  jq -e 'type == "array" and all(.[]; type == "array")' \
    "${pages_path}" > /dev/null || die "GitHub pagination returned malformed pages"
  jq '[.[][]]' "${pages_path}" > "${output_path}"
}

enumerate_releases() {
  gh api --paginate --slurp \
    "repos/${repository}/releases?per_page=100" \
    > "${state_dir}/release-pages.json"
  flatten_paginated_arrays \
    "${state_dir}/release-pages.json" \
    "${state_dir}/releases.json"
}

analyze_release_snapshot() {
  jq -e '
    all(.[];
      type == "object" and
      (.id as $id |
        ($id | type == "number") and
        ($id > 0) and
        (($id | floor) == $id)) and
      (.tag_name | type == "string") and
      (.target_commitish | type == "string") and
      (.draft | type == "boolean") and
      (.prerelease | type == "boolean") and
      ((.published_at | type) == "string" or .published_at == null)
    )
  ' "${state_dir}/releases.json" > /dev/null ||
    die "GitHub release enumeration returned a malformed release"

  jq --arg tag "${release_tag}" \
    '[.[] | select(.tag_name == $tag)]' \
    "${state_dir}/releases.json" > "${state_dir}/matching-releases.json"
  jq 'length' "${state_dir}/matching-releases.json" \
    > "${state_dir}/matching-release-count.txt"
  IFS= read -r matching_release_count \
    < "${state_dir}/matching-release-count.txt"

  if [[ "${matching_release_count}" -gt 1 ]]; then
    die "multiple releases use tag ${release_tag}; refusing ambiguous state"
  fi
  if [[ "${matching_release_count}" -eq 0 ]]; then
    release_observation="absent"
    matching_release_id=""
    return
  fi

  jq '.[0]' "${state_dir}/matching-releases.json" \
    > "${state_dir}/matching-release.json"
  jq -e \
    --arg tag "${release_tag}" \
    --arg sha "${release_sha}" '
      (.tag_name == $tag) and
      (.target_commitish == $sha) and
      (.draft == true) and
      (.prerelease == false) and
      (.name | type == "string") and
      (.body | type == "string") and
      (.html_url | type == "string")
    ' "${state_dir}/matching-release.json" > /dev/null || {
      jq '{id, tag_name, target_commitish, draft, prerelease, name, body, html_url}' \
        "${state_dir}/matching-release.json" >&2
      die "${release_tag} is not an exact-SHA, non-prerelease draft"
    }
  jq -r '.id' "${state_dir}/matching-release.json" \
    > "${state_dir}/matching-release-id.txt"
  IFS= read -r matching_release_id \
    < "${state_dir}/matching-release-id.txt"
  release_observation="present"
}

enumerate_tag_refs() {
  gh api --paginate --slurp \
    "repos/${repository}/git/matching-refs/tags/${encoded_tag}?per_page=100" \
    > "${state_dir}/tag-pages.json"
  flatten_paginated_arrays \
    "${state_dir}/tag-pages.json" \
    "${state_dir}/tag-refs.json"
}

analyze_tag_snapshot() {
  jq -e '
    all(.[];
      type == "object" and
      (.ref | type == "string") and
      (.object | type == "object") and
      (.object.type | type == "string") and
      (.object.sha | type == "string")
    )
  ' "${state_dir}/tag-refs.json" > /dev/null ||
    die "GitHub tag enumeration returned a malformed ref"
  jq --arg ref "refs/tags/${release_tag}" \
    '[.[] | select(.ref == $ref)]' \
    "${state_dir}/tag-refs.json" > "${state_dir}/matching-tags.json"
  jq 'length' "${state_dir}/matching-tags.json" \
    > "${state_dir}/matching-tag-count.txt"
  IFS= read -r matching_tag_count < "${state_dir}/matching-tag-count.txt"

  if [[ "${matching_tag_count}" -gt 1 ]]; then
    die "Git ref enumeration returned duplicate exact tags for ${release_tag}"
  fi
  if [[ "${matching_tag_count}" -eq 0 ]]; then
    tag_observation="absent"
    return
  fi
  jq -e \
    --arg ref "refs/tags/${release_tag}" \
    --arg sha "${release_sha}" '
      (.[0].ref == $ref) and
      (.[0].object.type == "commit") and
      (.[0].object.sha == $sha)
    ' "${state_dir}/matching-tags.json" > /dev/null || {
      jq '.[0]' "${state_dir}/matching-tags.json" >&2
      die "${release_tag} is not a lightweight tag at ${release_sha}"
    }
  tag_observation="present"
}

enumerate_assets() {
  gh api --paginate --slurp \
    "repos/${repository}/releases/${release_id}/assets?per_page=100" \
    > "${state_dir}/asset-pages.json"
  flatten_paginated_arrays \
    "${state_dir}/asset-pages.json" \
    "${state_dir}/assets.json"
}

validate_asset_snapshot_shape() {
  jq -e '
    all(.[];
      type == "object" and
      (.id as $id |
        ($id | type == "number") and
        ($id > 0) and
        (($id | floor) == $id)) and
      (.name | type == "string") and
      (.state | type == "string") and
      (((.size | type) == "number" and .size >= 0 and (.size | floor) == .size) or
       .size == null) and
      (((.digest | type) == "string") or .digest == null)
    ) and
    ([.[].id] | length == (unique | length)) and
    ([.[].name] | length == (unique | length))
  ' "${state_dir}/assets.json" > /dev/null ||
    die "GitHub returned malformed or duplicate release assets"

  jq -e --slurpfile expected "${state_dir}/expected-assets.json" '
    $expected[0] as $expected_assets |
    all(.[];
      .name as $name |
      ($expected_assets | map(.name) | index($name)) != null
    )
  ' "${state_dir}/assets.json" > /dev/null || {
    jq '[.[] | {id, name, state}]' "${state_dir}/assets.json" >&2
    die "draft contains an unmanaged asset"
  }
}

classify_final_asset_snapshot() {
  local expected_name asset_match_count expected_size expected_digest
  local actual_size actual_digest actual_state
  validate_asset_snapshot_shape
  asset_snapshot_complete="true"

  while IFS= read -r expected_name; do
    jq --arg name "${expected_name}" \
      '[.[] | select(.name == $name)] | length' \
      "${state_dir}/assets.json" > "${state_dir}/asset-match-count.txt"
    IFS= read -r asset_match_count < "${state_dir}/asset-match-count.txt"
    if [[ "${asset_match_count}" -eq 0 ]]; then
      asset_snapshot_complete="false"
      continue
    fi

    jq --arg name "${expected_name}" \
      '.[] | select(.name == $name)' \
      "${state_dir}/assets.json" > "${state_dir}/current-asset.json"
    jq -r --arg name "${expected_name}" \
      '.[] | select(.name == $name) | .size' \
      "${state_dir}/expected-assets.json" > "${state_dir}/expected-size.txt"
    jq -r --arg name "${expected_name}" \
      '.[] | select(.name == $name) | .digest' \
      "${state_dir}/expected-assets.json" > "${state_dir}/expected-digest.txt"
    jq -r '.size // "null"' "${state_dir}/current-asset.json" \
      > "${state_dir}/actual-size.txt"
    jq -r '.digest // "null"' "${state_dir}/current-asset.json" \
      > "${state_dir}/actual-digest.txt"
    jq -r '.state' "${state_dir}/current-asset.json" \
      > "${state_dir}/actual-state.txt"
    IFS= read -r expected_size < "${state_dir}/expected-size.txt"
    IFS= read -r expected_digest < "${state_dir}/expected-digest.txt"
    IFS= read -r actual_size < "${state_dir}/actual-size.txt"
    IFS= read -r actual_digest < "${state_dir}/actual-digest.txt"
    IFS= read -r actual_state < "${state_dir}/actual-state.txt"

    if [[ "${actual_size}" != "null" && "${actual_size}" != "${expected_size}" ]]; then
      die "asset ${expected_name} has the wrong non-null size"
    fi
    if [[ "${actual_digest}" != "null" && "${actual_digest}" != "${expected_digest}" ]]; then
      die "asset ${expected_name} has the wrong non-null digest"
    fi
    if [[ "${actual_state}" == "uploaded" &&
          "${actual_size}" == "${expected_size}" &&
          "${actual_digest}" == "${expected_digest}" ]]; then
      continue
    fi
    if [[ "${actual_state}" != "starter" && "${actual_state}" != "uploaded" ]]; then
      die "asset ${expected_name} has unexpected state ${actual_state}"
    fi
    asset_snapshot_complete="false"
  done < "${state_dir}/expected-names.txt"
}

capture_preserved_release_text() {
  jq '{name, body}' "${state_dir}/matching-release.json" \
    > "${state_dir}/preserved-text.json"
}

verify_matching_release_identity_and_text() {
  local expected_release_id="$1"
  if [[ "${matching_release_id}" != "${expected_release_id}" ]]; then
    die "${release_tag} resolved to foreign release id ${matching_release_id}; expected ${expected_release_id}"
  fi
  jq '{name, body}' "${state_dir}/matching-release.json" \
    > "${state_dir}/observed-text.json"
  jq -e -s '.[0] == .[1]' \
    "${state_dir}/preserved-text.json" \
    "${state_dir}/observed-text.json" > /dev/null ||
    die "draft title or body changed during resume"
}

prepare_expected_assets() {
  local -a expected_names
  local expected_name asset_path local_size local_sha256
  python3 tools/cli_release.py expected-assets --version "${release_version}" \
    > "${state_dir}/expected-names.txt"
  mapfile -t expected_names < "${state_dir}/expected-names.txt"
  if [[ "${#expected_names[@]}" -ne 6 ]]; then
    die "release helper did not return exactly six assets"
  fi

  find "${release_dir}" -mindepth 1 -maxdepth 1 -printf '%f\n' \
    > "${state_dir}/local-names-unsorted.txt"
  sort "${state_dir}/local-names-unsorted.txt" > "${state_dir}/local-names.txt"
  sort "${state_dir}/expected-names.txt" > "${state_dir}/expected-names-sorted.txt"
  if ! cmp -s "${state_dir}/local-names.txt" "${state_dir}/expected-names-sorted.txt"; then
    die "downloaded release directory does not contain the exact six assets"
  fi

  : > "${state_dir}/expected-assets.ndjson"
  for expected_name in "${expected_names[@]}"; do
    asset_path="${release_dir}/${expected_name}"
    [[ -f "${asset_path}" && ! -L "${asset_path}" ]] ||
      die "release asset must be a regular, non-symlink file: ${expected_name}"
    stat --format '%s' "${asset_path}" > "${state_dir}/local-size.txt"
    sha256sum "${asset_path}" > "${state_dir}/local-sha256.txt"
    IFS= read -r local_size < "${state_dir}/local-size.txt"
    IFS=' ' read -r local_sha256 _ < "${state_dir}/local-sha256.txt"
    [[ "${local_size}" =~ ^[0-9]+$ ]] || die "invalid local size for ${expected_name}"
    [[ "${local_sha256}" =~ ^[0-9a-f]{64}$ ]] ||
      die "invalid local SHA-256 for ${expected_name}"
    jq -n \
      --arg name "${expected_name}" \
      --argjson size "${local_size}" \
      --arg digest "sha256:${local_sha256}" \
      '{name: $name, size: $size, digest: $digest}' \
      >> "${state_dir}/expected-assets.ndjson"
  done
  jq -s '.' "${state_dir}/expected-assets.ndjson" \
    > "${state_dir}/expected-assets.json"
}

confirm_tag_only_release_absence() {
  local consecutive_absences=1
  start_poll_deadline "tag-only release absence confirmation"
  while [[ "${consecutive_absences}" -lt 2 ]]; do
    wait_for_next_poll
    enumerate_releases
    analyze_release_snapshot
    if [[ "${release_observation}" == "present" ]]; then
      return
    fi
    consecutive_absences=$((consecutive_absences + 1))
  done
}

generate_release_notes() {
  local stable_base notes_base initial_base_count
  jq -r '
    [.[] |
      select(
        (.draft == false) and
        (.prerelease == false) and
        (.published_at | type == "string")
      )] |
    sort_by(.published_at) |
    if length == 0 then "" else .[-1].tag_name end
  ' "${state_dir}/releases.json" > "${state_dir}/stable-base.txt"
  IFS= read -r stable_base < "${state_dir}/stable-base.txt"
  if [[ -n "${stable_base}" ]]; then
    notes_base="${stable_base}"
  else
    jq '[.[] |
          select(
            (.tag_name == "pr-artifacts") and
            (.draft == false) and
            (.prerelease == true) and
            (.published_at | type == "string")
          )] | length' \
      "${state_dir}/releases.json" > "${state_dir}/initial-base-count.txt"
    IFS= read -r initial_base_count < "${state_dir}/initial-base-count.txt"
    if [[ "${initial_base_count}" -ne 1 ]]; then
      die "the initial release requires exactly one published pr-artifacts prerelease base"
    fi
    notes_base="pr-artifacts"
  fi
  if [[ -z "${notes_base}" || "${notes_base}" == "${release_tag}" ]]; then
    die "invalid explicit release-notes base: ${notes_base}"
  fi

  echo "Generating ${release_tag} notes with exact comparison base ${notes_base}"
  jq -n \
    --arg tag "${release_tag}" \
    --arg target "${release_sha}" \
    --arg previous "${notes_base}" \
    '{tag_name: $tag, target_commitish: $target, previous_tag_name: $previous}' \
    > "${state_dir}/notes-request.json"
  gh api --method POST \
    "repos/${repository}/releases/generate-notes" \
    --input "${state_dir}/notes-request.json" \
    > "${state_dir}/generated-notes.json"
  jq -e '
    type == "object" and
    (.name | type == "string") and
    (.name | length > 0) and
    (((.name | contains("\n")) or (.name | contains("\r"))) | not) and
    (.body | type == "string") and
    (.body | length > 0)
  ' "${state_dir}/generated-notes.json" > /dev/null ||
    die "Release Notes API returned an invalid title or empty body"
  jq -r -j '.name' "${state_dir}/generated-notes.json" > "${state_dir}/title.txt"
  jq -r -j '.body' "${state_dir}/generated-notes.json" > "${state_dir}/notes.md"
  echo "Generated release title: $(< "${state_dir}/title.txt")"
  echo "Generated release notes:"
  cat "${state_dir}/notes.md"
  echo
}

create_tag() {
  jq -n \
    --arg ref "refs/tags/${release_tag}" \
    --arg sha "${release_sha}" \
    '{ref: $ref, sha: $sha}' > "${state_dir}/create-tag-request.json"
  gh api --method POST \
    "repos/${repository}/git/refs" \
    --input "${state_dir}/create-tag-request.json" \
    > "${state_dir}/created-tag.json"
  jq -e \
    --arg ref "refs/tags/${release_tag}" \
    --arg sha "${release_sha}" '
      type == "object" and
      (.ref == $ref) and
      (.object | type == "object") and
      (.object.type == "commit") and
      (.object.sha == $sha)
    ' "${state_dir}/created-tag.json" > /dev/null ||
    die "tag creation response did not identify the exact lightweight tag"
}

poll_created_tag_visibility() {
  start_poll_deadline "created tag visibility"
  while :; do
    enumerate_tag_refs
    analyze_tag_snapshot
    if [[ "${tag_observation}" == "present" ]]; then
      return
    fi
    wait_for_next_poll
  done
}

create_release() {
  jq -n \
    --arg tag "${release_tag}" \
    --arg target "${release_sha}" \
    --rawfile name "${state_dir}/title.txt" \
    --rawfile body "${state_dir}/notes.md" '
      {
        tag_name: $tag,
        target_commitish: $target,
        name: $name,
        body: $body,
        draft: true,
        prerelease: false,
        generate_release_notes: false
      }
    ' > "${state_dir}/create-release-request.json"
  gh api --method POST \
    "repos/${repository}/releases" \
    --input "${state_dir}/create-release-request.json" \
    > "${state_dir}/created-release.json"
  jq -e \
    --arg tag "${release_tag}" \
    --arg sha "${release_sha}" \
    --rawfile expected_name "${state_dir}/title.txt" \
    --rawfile expected_body "${state_dir}/notes.md" '
      type == "object" and
      (.id as $id |
        ($id | type == "number") and
        ($id > 0) and
        (($id | floor) == $id)) and
      (.tag_name == $tag) and
      (.target_commitish == $sha) and
      (.draft == true) and
      (.prerelease == false) and
      (.name == $expected_name) and
      (.body == $expected_body) and
      (.assets | type == "array") and
      (.assets | length == 0)
    ' "${state_dir}/created-release.json" > /dev/null ||
    die "release creation response did not identify the exact empty draft"
  jq -r '.id' "${state_dir}/created-release.json" \
    > "${state_dir}/created-release-id.txt"
  IFS= read -r release_id < "${state_dir}/created-release-id.txt"
  jq '{name, body}' "${state_dir}/created-release.json" \
    > "${state_dir}/preserved-text.json"
}

poll_created_release_uniqueness() {
  local created_release_id="${release_id}"
  start_poll_deadline "created release visibility"
  while :; do
    enumerate_releases
    analyze_release_snapshot
    if [[ "${release_observation}" == "present" ]]; then
      verify_matching_release_identity_and_text "${created_release_id}"
      release_id="${created_release_id}"
      return
    fi
    wait_for_next_poll
  done
}

delete_existing_managed_assets() {
  local asset_id
  jq -r '.[].id' "${state_dir}/assets.json" > "${state_dir}/delete-asset-ids.txt"
  while IFS= read -r asset_id; do
    [[ "${asset_id}" =~ ^[1-9][0-9]*$ ]] || die "invalid managed asset id"
    gh api --method DELETE \
      "repos/${repository}/releases/assets/${asset_id}" \
      --silent
  done < "${state_dir}/delete-asset-ids.txt"
}

upload_expected_assets() {
  local expected_name encoded_asset_name expected_size expected_digest
  while IFS= read -r expected_name; do
    jq -rn --arg value "${expected_name}" '$value | @uri' \
      > "${state_dir}/encoded-asset-name.txt"
    IFS= read -r encoded_asset_name < "${state_dir}/encoded-asset-name.txt"
    jq -r --arg name "${expected_name}" \
      '.[] | select(.name == $name) | .size' \
      "${state_dir}/expected-assets.json" > "${state_dir}/expected-size.txt"
    jq -r --arg name "${expected_name}" \
      '.[] | select(.name == $name) | .digest' \
      "${state_dir}/expected-assets.json" > "${state_dir}/expected-digest.txt"
    IFS= read -r expected_size < "${state_dir}/expected-size.txt"
    IFS= read -r expected_digest < "${state_dir}/expected-digest.txt"

    gh api --method POST \
      --header "Content-Type: application/octet-stream" \
      --input "${release_dir}/${expected_name}" \
      "https://uploads.github.com/repos/${repository}/releases/${release_id}/assets?name=${encoded_asset_name}" \
      > "${state_dir}/uploaded-asset.json"
    jq -e \
      --arg name "${expected_name}" \
      --arg digest "${expected_digest}" \
      --argjson size "${expected_size}" '
        type == "object" and
        (.id as $id |
          ($id | type == "number") and
          ($id > 0) and
          (($id | floor) == $id)) and
        (.name == $name) and
        (.state == "starter" or .state == "uploaded") and
        (.size == null or .size == $size) and
        (.digest == null or .digest == $digest)
      ' "${state_dir}/uploaded-asset.json" > /dev/null ||
      die "upload response for ${expected_name} did not match the local asset"
  done < "${state_dir}/expected-names.txt"
}

poll_final_assets() {
  start_poll_deadline "final asset visibility"
  while :; do
    enumerate_assets
    classify_final_asset_snapshot
    if [[ "${asset_snapshot_complete}" == "true" ]]; then
      return
    fi
    wait_for_next_poll
  done
}

prepare_expected_assets

enumerate_releases
analyze_release_snapshot
enumerate_tag_refs
analyze_tag_snapshot

if [[ "${release_observation}" == "present" ]]; then
  [[ "${tag_observation}" == "present" ]] ||
    die "existing draft ${release_tag} has no matching git tag"
  release_id="${matching_release_id}"
  capture_preserved_release_text
else
  if [[ "${tag_observation}" == "present" ]]; then
    confirm_tag_only_release_absence
  fi

  if [[ "${release_observation}" == "present" ]]; then
    release_id="${matching_release_id}"
    capture_preserved_release_text
  else
    generate_release_notes
    if [[ "${tag_observation}" == "absent" ]]; then
      create_tag
      poll_created_tag_visibility
    else
      enumerate_tag_refs
      analyze_tag_snapshot
      [[ "${tag_observation}" == "present" ]] ||
        die "exact tag disappeared before release creation"
    fi
    create_release
    poll_created_release_uniqueness
  fi
fi

enumerate_assets
validate_asset_snapshot_shape
delete_existing_managed_assets
upload_expected_assets
poll_final_assets

enumerate_releases
analyze_release_snapshot
[[ "${release_observation}" == "present" ]] ||
  die "final release enumeration did not contain ${release_tag}"
verify_matching_release_identity_and_text "${release_id}"

enumerate_assets
classify_final_asset_snapshot
[[ "${asset_snapshot_complete}" == "true" ]] ||
  die "final release asset proof did not contain the exact uploaded six"

enumerate_tag_refs
analyze_tag_snapshot
[[ "${tag_observation}" == "present" ]] ||
  die "final tag verification failed"

jq -r '.html_url' "${state_dir}/matching-release.json" \
  > "${state_dir}/release-url.txt"
IFS= read -r release_url < "${state_dir}/release-url.txt"
case "${release_url}" in
  "https://github.com/${repository}/releases/"*) ;;
  *) die "unexpected release URL: ${release_url}" ;;
esac

{
  echo "### Draft CLI release"
  echo
  echo "[${release_tag}](${release_url}) targets \`${release_sha}\` and remains unpublished."
} >> "${summary_file}"
