#!/usr/bin/env bash
set -euo pipefail

asset_dir=${1:?usage: finalize-release.sh ASSET_DIR VERSION TAG TARGET_SHA PUBLIC_KEY}
version=${2:?usage: finalize-release.sh ASSET_DIR VERSION TAG TARGET_SHA PUBLIC_KEY}
tag=${3:?usage: finalize-release.sh ASSET_DIR VERSION TAG TARGET_SHA PUBLIC_KEY}
target_sha=${4:?usage: finalize-release.sh ASSET_DIR VERSION TAG TARGET_SHA PUBLIC_KEY}
public_key=${5:?usage: finalize-release.sh ASSET_DIR VERSION TAG TARGET_SHA PUBLIC_KEY}

mapfile -t debs < <(find "$asset_dir" -maxdepth 1 -type f -name '*.deb' -printf '%f\n')
mapfile -t exes < <(find "$asset_dir" -maxdepth 1 -type f -name '*.exe' -printf '%f\n')
[ "${#debs[@]}" -eq 1 ] || { printf 'expected one direct Debian payload\n' >&2; exit 1; }
[ "${#exes[@]}" -eq 1 ] || { printf 'expected one direct NSIS payload\n' >&2; exit 1; }

for payload in "${debs[0]}" "${exes[0]}"; do
  [ -s "$asset_dir/$payload.sig" ] || { printf 'missing signature for %s\n' "$payload" >&2; exit 1; }
done
[ -s "$asset_dir/latest.json" ] || { printf 'missing latest.json\n' >&2; exit 1; }
[ -s "$asset_dir/ds-controller-windows-$tag.zip" ] || { printf 'missing Windows convenience ZIP\n' >&2; exit 1; }
[ -s "$asset_dir/ds-controller-ubuntu-$tag.zip" ] || { printf 'missing Ubuntu convenience ZIP\n' >&2; exit 1; }

release_json=$(gh release view "$tag" --json assets,isDraft,isPrerelease,targetCommitish)
is_prerelease=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isPrerelease"]).lower())' <<<"$release_json")
is_draft=$(python3 -c 'import json,sys; print(str(json.load(sys.stdin)["isDraft"]).lower())' <<<"$release_json")
[ "$is_prerelease" = false ] || { printf 'release is unexpectedly a prerelease\n' >&2; exit 1; }
if [ "${VERIFY_ONLY:-false}" = true ]; then
  [ "$is_draft" = false ] || { printf 'published rerun found a draft\n' >&2; exit 1; }
else
  [ "$is_draft" = true ] || { printf 'refusing to mutate a published release\n' >&2; exit 1; }
fi

printf '%s' "$public_key" | base64 --decode > "$asset_dir/updater.pub"
for payload in "${debs[0]}" "${exes[0]}"; do
  base64 --decode < "$asset_dir/$payload.sig" > "$asset_dir/$payload.minisig"
  minisign -Vm "$asset_dir/$payload" -p "$asset_dir/updater.pub" -x "$asset_dir/$payload.minisig"
done

REPOSITORY=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required} RELEASE_JSON="$release_json" python3 - "$asset_dir" "$version" "$tag" "${debs[0]}" "${exes[0]}" <<'PY'
import json, os, pathlib, sys

directory, version, tag, deb_name, exe_name = sys.argv[1:]
manifest = json.loads(pathlib.Path(directory, "latest.json").read_text())
if manifest.get("version") != version:
    raise SystemExit("latest.json version mismatch")
assets = json.loads(os.environ["RELEASE_JSON"])["assets"]
asset_urls_by_name = {asset["name"]: asset["apiUrl"] for asset in assets}
expected = {"linux-x86_64-deb": deb_name, "windows-x86_64-nsis": exe_name}
for platform, filename in expected.items():
    entry = manifest.get("platforms", {}).get(platform)
    if not entry:
        raise SystemExit(f"missing {platform} updater entry")
    asset_url = asset_urls_by_name.get(filename)
    if not asset_url:
        raise SystemExit(f"release does not contain {filename}")
    if entry.get("url") != asset_url:
        raise SystemExit(f"{platform} URL does not exactly reference {filename}")
    signature = pathlib.Path(directory, filename + ".sig").read_text().strip()
    if entry.get("signature") != signature:
        raise SystemExit(f"{platform} inline signature mismatch")
PY

if [ "$is_draft" = true ]; then
  release_sha=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["targetCommitish"])' <<<"$release_json")
  [ "$release_sha" = "$target_sha" ] || { printf 'release source changed before promotion\n' >&2; exit 1; }
  GITHUB_OUTPUT=/dev/null bash "$(dirname -- "$0")/preflight-release.sh" --before-publish "$version" "$target_sha" >/dev/null
else
  if tag_output=$(git ls-remote --exit-code origin "refs/tags/$tag" "refs/tags/$tag^{}"); then
    tag_status=0
  else
    tag_status=$?
  fi
  [ "$tag_status" -eq 0 ] || { printf 'could not look up published release tag\n' >&2; exit 1; }
  remote_sha=$(TAG="$tag" python3 -c '
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
' <<<"$tag_output") || { printf 'could not resolve published release tag\n' >&2; exit 1; }
  [ "$remote_sha" = "$target_sha" ] || { printf 'release tag moved after publication\n' >&2; exit 1; }
fi

rm -f "$asset_dir/updater.pub" "$asset_dir"/*.minisig
if [ "${VERIFY_ONLY:-false}" = true ]; then
  (cd "$asset_dir" && sha256sum -c SHA256SUMS)
else
  (cd "$asset_dir" && sha256sum -- *.deb *.deb.sig *.exe *.exe.sig latest.json ds-controller-*.zip > SHA256SUMS)
  (cd "$asset_dir" && sha256sum -c SHA256SUMS)
fi
