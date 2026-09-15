#!/usr/bin/env bash
set -euo pipefail

version=${1:?usage: preflight-release.sh VERSION TARGET_SHA}
target_sha=${2:?usage: preflight-release.sh VERSION TARGET_SHA}
tag="v${version}"

if [[ ! "$version" =~ ^[0-9]{4}\.[1-9][0-9]?\.[1-9][0-9]?-[1-9][0-9]*$ ]]; then
  printf 'release version is not canonical: %s\n' "$version" >&2
  exit 1
fi

skip_mutation=false
if release_json=$(gh release view "$tag" --json isDraft 2>/dev/null); then
  remote_sha=$(git ls-remote origin "refs/tags/$tag^{}" "refs/tags/$tag" | awk 'NR == 1 { value=$1 } /\^\{\}$/ { value=$1 } END { print value }')
  if [ "$remote_sha" != "$target_sha" ]; then
    printf 'existing release %s points to %s, expected %s\n' "$tag" "${remote_sha:-unknown}" "$target_sha" >&2
    exit 1
  fi
  is_draft=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isDraft"]).lower())' <<<"$release_json")
  if [ "$is_draft" = false ]; then
    skip_mutation=true
  fi
elif git ls-remote --exit-code origin "refs/tags/$tag" >/dev/null 2>&1; then
  printf 'tag %s exists without a GitHub release\n' "$tag" >&2
  exit 1
fi

if [ "$skip_mutation" = true ]; then
  printf 'skip_mutation=true\n' >> "${GITHUB_OUTPUT:-/dev/stdout}"
  printf 'tag=%s\n' "$tag" >> "${GITHUB_OUTPUT:-/dev/stdout}"
  printf 'Published release verification preflight passed for %s at %s\n' "$version" "$target_sha"
  exit 0
fi

repository=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}
newest=$(gh api --paginate "repos/$repository/releases?per_page=100" \
  --jq '.[] | select(.draft == false and .prerelease == false) | .tag_name' \
  | sed -nE 's/^v([0-9]{4}\.[1-9][0-9]?\.[1-9][0-9]?-[1-9][0-9]*)$/\1/p' \
  | sort -V | tail -n 1)
if [ -n "$newest" ] && [ "$newest" != "$version" ] \
  && [ "$(printf '%s\n' "$newest" "$version" | sort -V | tail -n 1)" != "$version" ]; then
  printf 'release version %s would not advance published updater version %s\n' "$version" "$newest" >&2
  exit 1
fi

printf 'skip_mutation=%s\n' "$skip_mutation" >> "${GITHUB_OUTPUT:-/dev/stdout}"
printf 'tag=%s\n' "$tag" >> "${GITHUB_OUTPUT:-/dev/stdout}"
printf 'Release preflight passed for %s at %s\n' "$version" "$target_sha"
