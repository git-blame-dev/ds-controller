#!/usr/bin/env bash
set -euo pipefail

before_publish=false
if [ "${1:-}" = --before-publish ]; then
  before_publish=true
  shift
fi

version=${1:?usage: preflight-release.sh [--before-publish] VERSION TARGET_SHA}
target_sha=${2:?usage: preflight-release.sh [--before-publish] VERSION TARGET_SHA}
tag="v${version}"
repository=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}

if [[ ! "$version" =~ ^[0-9]{4}\.[1-9][0-9]?\.[1-9][0-9]?-[1-9][0-9]*$ ]]; then
  printf 'release version is not canonical: %s\n' "$version" >&2
  exit 1
fi

releases_file=$(mktemp)
trap 'rm -f "$releases_file"' EXIT
if ! gh api --paginate "repos/$repository/releases?per_page=100" --jq '.[]' > "$releases_file"; then
  printf 'could not query GitHub releases\n' >&2
  exit 1
fi

release_summary=$(python3 - "$releases_file" "$tag" <<'PY'
import json
import pathlib
import re
import sys

source, tag = sys.argv[1:]
releases = []
for line_number, line in enumerate(pathlib.Path(source).read_text().splitlines(), 1):
    try:
        release = json.loads(line)
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid GitHub release response on line {line_number}: {error}")
    if not isinstance(release, dict):
        raise SystemExit(f"invalid GitHub release response on line {line_number}")
    releases.append(release)

matching = [release for release in releases if release.get("tag_name") == tag]
if len(matching) > 1:
    raise SystemExit(f"GitHub returned duplicate releases for {tag}")

state = "absent"
target = ""
if matching:
    release = matching[0]
    if release.get("prerelease") is True:
        state = "prerelease"
    elif release.get("draft") is True:
        state = "draft"
    elif release.get("draft") is False:
        state = "published"
    else:
        raise SystemExit(f"GitHub returned invalid draft metadata for {tag}")
    target = release.get("target_commitish", "")
    if not isinstance(target, str):
        raise SystemExit(f"GitHub returned invalid target metadata for {tag}")

canonical = re.compile(r"^v(\d{4})\.([1-9]\d?)\.([1-9]\d?)-([1-9]\d*)$")
published_versions = []
for release in releases:
    if release.get("draft") is False and release.get("prerelease") is False:
        match = canonical.fullmatch(str(release.get("tag_name", "")))
        if match:
            published_versions.append((tuple(map(int, match.groups())), match.group(0)[1:]))
newest = max(published_versions, default=((), ""))[1]
print(json.dumps({"state": state, "target": target, "newest": newest}))
PY
)

release_state=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"$release_summary")
release_sha=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["target"])' <<<"$release_summary")
newest=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["newest"])' <<<"$release_summary")

lookup_remote_tag() {
  local output status parsed
  if output=$(git ls-remote --exit-code origin "refs/tags/$tag" "refs/tags/$tag^{}"); then
    status=0
  else
    status=$?
  fi

  if [ "$status" -eq 2 ]; then
    remote_tag_state=absent
    remote_tag_sha=
    return
  fi
  if [ "$status" -ne 0 ]; then
    printf 'could not look up remote tag %s\n' "$tag" >&2
    exit 1
  fi

  if ! parsed=$(TAG="$tag" python3 -c '
import os, sys
base = "refs/tags/" + os.environ["TAG"]
values = {}
for line in sys.stdin:
    fields = line.rstrip("\n").split("\t")
    if len(fields) != 2 or fields[1] not in (base, base + "^{}"):
        raise SystemExit("invalid remote tag response")
    values.setdefault(fields[1], set()).add(fields[0])
if base not in values or any(len(items) != 1 for items in values.values()):
    raise SystemExit("ambiguous remote tag response")
print(next(iter(values.get(base + "^{}", values[base]))))
' <<<"$output"); then
    printf 'could not resolve remote tag %s\n' "$tag" >&2
    exit 1
  fi
  remote_tag_state=present
  remote_tag_sha=$parsed
}

lookup_remote_main() {
  local output status parsed
  if output=$(git ls-remote --exit-code origin refs/heads/main); then
    status=0
  else
    status=$?
  fi
  if [ "$status" -ne 0 ]; then
    printf 'could not look up remote main\n' >&2
    exit 1
  fi
  if ! parsed=$(python3 -c '
import sys
lines = [line.rstrip("\n").split("\t") for line in sys.stdin]
matches = [fields[0] for fields in lines if len(fields) == 2 and fields[1] == "refs/heads/main"]
if len(set(matches)) != 1:
    raise SystemExit("ambiguous remote main response")
print(matches[0])
' <<<"$output"); then
    printf 'could not resolve remote main\n' >&2
    exit 1
  fi
  remote_main_sha=$parsed
}

lookup_remote_tag

if [ "$before_publish" = true ]; then
  [ "$release_state" = draft ] || { printf 'release %s is not a draft immediately before publication\n' "$tag" >&2; exit 1; }
  [ "$release_sha" = "$target_sha" ] || { printf 'draft release %s points to %s, expected %s\n' "$tag" "${release_sha:-unknown}" "$target_sha" >&2; exit 1; }
  if [ "$remote_tag_state" = present ] && [ "$remote_tag_sha" != "$target_sha" ]; then
    printf 'existing tag %s points to %s, expected %s\n' "$tag" "$remote_tag_sha" "$target_sha" >&2
    exit 1
  fi
  lookup_remote_main
  [ "$remote_main_sha" = "$target_sha" ] || { printf 'main advanced before promotion\n' >&2; exit 1; }
else
  case "$release_state" in
    absent)
      [ "$remote_tag_state" = absent ] || { printf 'tag %s exists without a GitHub release\n' "$tag" >&2; exit 1; }
      ;;
    draft)
      [ "$release_sha" = "$target_sha" ] || { printf 'existing release %s points to %s, expected %s\n' "$tag" "${release_sha:-unknown}" "$target_sha" >&2; exit 1; }
      if [ "$remote_tag_state" = present ] && [ "$remote_tag_sha" != "$target_sha" ]; then
        printf 'existing tag %s points to %s, expected %s\n' "$tag" "$remote_tag_sha" "$target_sha" >&2
        exit 1
      fi
      ;;
    published)
      [ "$remote_tag_state" = present ] || { printf 'published release tag %s is absent\n' "$tag" >&2; exit 1; }
      [ "$remote_tag_sha" = "$target_sha" ] || { printf 'existing release %s points to %s, expected %s\n' "$tag" "$remote_tag_sha" "$target_sha" >&2; exit 1; }
      ;;
    *)
      printf 'release %s is unexpectedly a prerelease\n' "$tag" >&2
      exit 1
      ;;
  esac
fi

if [ "$before_publish" = false ] && [ "$release_state" = published ]; then
  printf 'skip_mutation=true\n' >> "${GITHUB_OUTPUT:-/dev/stdout}"
  printf 'tag=%s\n' "$tag" >> "${GITHUB_OUTPUT:-/dev/stdout}"
  printf 'Published release verification preflight passed for %s at %s\n' "$version" "$target_sha"
  exit 0
fi

if [ -n "$newest" ] && [ "$newest" != "$version" ] \
  && [ "$(printf '%s\n' "$newest" "$version" | sort -V | tail -n 1)" != "$version" ]; then
  printf 'release version %s would not advance published updater version %s\n' "$version" "$newest" >&2
  exit 1
fi

printf 'skip_mutation=false\n' >> "${GITHUB_OUTPUT:-/dev/stdout}"
printf 'tag=%s\n' "$tag" >> "${GITHUB_OUTPUT:-/dev/stdout}"
if [ "$before_publish" = true ]; then
  printf 'Pre-publication release check passed for %s at %s\n' "$version" "$target_sha"
else
  printf 'Release preflight passed for %s at %s\n' "$version" "$target_sha"
fi
