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

release_json=$(gh release view "$tag" --json isDraft,isPrerelease)
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

REPOSITORY=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required} python3 - "$asset_dir" "$version" "$tag" "${debs[0]}" "${exes[0]}" <<'PY'
import json, os, pathlib, subprocess, sys

directory, version, tag, deb_name, exe_name = sys.argv[1:]
manifest = json.loads(pathlib.Path(directory, "latest.json").read_text())
if manifest.get("version") != version:
    raise SystemExit("latest.json version mismatch")
assets = json.loads(subprocess.check_output([
    "gh", "api", f"repos/{os.environ['REPOSITORY']}/releases/tags/{tag}"
]))["assets"]
asset_ids_by_name = {asset["name"]: str(asset["id"]) for asset in assets}
expected = {"linux-x86_64-deb": deb_name, "windows-x86_64-nsis": exe_name}
for platform, filename in expected.items():
    entry = manifest.get("platforms", {}).get(platform)
    if not entry:
        raise SystemExit(f"missing {platform} updater entry")
    asset_id = asset_ids_by_name.get(filename)
    if not asset_id:
        raise SystemExit(f"release does not contain {filename}")
    expected_url = f"https://api.github.com/repos/{os.environ['REPOSITORY']}/releases/assets/{asset_id}"
    if entry.get("url") != expected_url:
        raise SystemExit(f"{platform} URL does not exactly reference {filename}")
    signature = pathlib.Path(directory, filename + ".sig").read_text().strip()
    if entry.get("signature") != signature:
        raise SystemExit(f"{platform} inline signature mismatch")
PY

remote_sha=$(git ls-remote origin "refs/tags/$tag^{}" "refs/tags/$tag" | awk 'NR == 1 { value=$1 } /\^\{\}$/ { value=$1 } END { print value }')
[ "$remote_sha" = "$target_sha" ] || { printf 'release tag moved before promotion\n' >&2; exit 1; }
if [ "${VERIFY_ONLY:-false}" != true ]; then
  current_main=$(git ls-remote origin refs/heads/main | awk 'NR == 1 { print $1 }')
  [ "$current_main" = "$target_sha" ] || { printf 'main advanced before promotion\n' >&2; exit 1; }
fi

rm -f "$asset_dir/updater.pub" "$asset_dir"/*.minisig
if [ "${VERIFY_ONLY:-false}" = true ]; then
  (cd "$asset_dir" && sha256sum -c SHA256SUMS)
else
  (cd "$asset_dir" && sha256sum -- *.deb *.deb.sig *.exe *.exe.sig latest.json ds-controller-*.zip > SHA256SUMS)
  (cd "$asset_dir" && sha256sum -c SHA256SUMS)
fi
