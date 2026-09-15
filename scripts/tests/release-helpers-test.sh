#!/usr/bin/env bash
set -euo pipefail

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
fixture_root=$(mktemp -d)
trap 'rm -rf "$fixture_root"' EXIT
mock_bin="$fixture_root/bin"
mkdir -p "$mock_bin"

cat > "$mock_bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1 $2 ${3:-}" = "release view v2026.9.14-4" ]; then
  printf '{"isDraft":false,"isPrerelease":false,"assets":[{"name":"synthetic.deb","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/1"},{"name":"synthetic.exe","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/2"}]}\n'
elif [ "$1 $2 ${3:-}" = "release view v2026.9.15-8" ]; then
  printf '{"isDraft":true,"isPrerelease":false,"targetCommitish":"target-sha","assets":[{"name":"synthetic.deb","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/1"},{"name":"synthetic.exe","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/2"}]}\n'
elif [ "$1 $2 ${3:-}" = "release view v2026.9.17-10" ]; then
  printf '{"isDraft":true,"isPrerelease":false,"targetCommitish":"target-sha","assets":[{"name":"synthetic.deb","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/1"},{"name":"synthetic.exe","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/2"}]}\n'
elif [ "$1 $2" = "release view" ]; then
  exit 1
elif [ "$1" = api ] && [[ "$*" == *'--paginate repos/synthetic/project/releases?per_page=100 --jq'* ]]; then
  cat "$RELEASE_HISTORY_FIXTURE"
else
  printf 'unexpected gh invocation: %s\n' "$*" >&2
  exit 2
fi
EOF

cat > "$mock_bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = ls-remote ] && [[ "$*" == *'refs/heads/main'* ]]; then
  printf '%s\trefs/heads/main\n' "${REMOTE_MAIN_SHA:-newer-main}"
elif [ "$1" = ls-remote ] && [[ "$*" == *'--exit-code'* ]]; then
  exit 1
elif [ "$1" = ls-remote ]; then
  if [ "${REMOTE_TAG_ABSENT:-false}" != true ]; then
    printf '%s\trefs/tags/synthetic\n' "${REMOTE_TAG_SHA:-target-sha}"
  fi
else
  exit 2
fi
EOF

cat > "$mock_bin/minisign" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

[ "$#" -eq 6 ] || { printf 'unexpected minisign argument count: %s\n' "$#" >&2; exit 2; }
[ "$1" = -Vm ] || { printf 'unexpected minisign mode: %s\n' "$1" >&2; exit 2; }
payload=$2
[ "$3" = -p ] || { printf 'unexpected minisign public-key flag: %s\n' "$3" >&2; exit 2; }
public_key=$4
[ "$5" = -x ] || { printf 'unexpected minisign signature flag: %s\n' "$5" >&2; exit 2; }
signature=$6

case "$(basename "$payload")" in
  synthetic.deb|synthetic.exe) ;;
  *) printf 'unexpected minisign payload: %s\n' "$payload" >&2; exit 2 ;;
esac
[ "$payload" = "${MINISIGN_ASSET_DIR:?}/$(basename "$payload")" ] || exit 2
[ "$public_key" = "$MINISIGN_ASSET_DIR/updater.pub" ] || exit 2
[ "$signature" = "$payload.minisig" ] || exit 2

payload_name=$(basename "$payload")
[ "${MINISIGN_FAIL_PAYLOAD:-}" != "$payload_name" ]
EOF
chmod +x "$mock_bin"/*

history_fixture="$fixture_root/releases.json"
python3 - "$history_fixture" <<'PY'
import sys
with open(sys.argv[1], "w") as fixture:
    for index in range(1, 101):
        fixture.write(f"v2026.9.1-{index}\n")
    fixture.write("v2026.9.16-1\n")
PY

run_preflight() {
  local version=$1
  shift
  PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" "$@" \
    bash "$repo_root/scripts/preflight-release.sh" "$version" target-sha
}

printf 'TEST published older rerun ignores newer release monotonicity\n'
run_preflight 2026.9.14-4 env REMOTE_TAG_SHA=target-sha

printf 'TEST complete paginated history catches release beyond first 100\n'
if run_preflight 2026.9.15-9 env; then
  printf 'preflight ignored the newer release beyond the first page\n' >&2
  exit 1
fi

printf 'TEST partial draft still applies monotonicity\n'
if run_preflight 2026.9.15-8 env REMOTE_TAG_SHA=target-sha; then
  printf 'partial draft bypassed promotion-time monotonicity\n' >&2
  exit 1
fi

printf 'TEST draft preflight accepts exact source before tag creation\n'
run_preflight 2026.9.17-10 env REMOTE_TAG_ABSENT=true

printf 'TEST published rerun still requires exact source\n'
if run_preflight 2026.9.14-4 env REMOTE_TAG_SHA=wrong-sha; then
  printf 'published rerun accepted the wrong source\n' >&2
  exit 1
fi

partial_dir="$fixture_root/partial"
mkdir -p "$partial_dir"
printf payload > "$partial_dir/synthetic.deb"
printf payload > "$partial_dir/synthetic.exe"
printf 'TEST partial draft cannot pass final validation\n'
if PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project REMOTE_TAG_SHA=target-sha \
  bash "$repo_root/scripts/finalize-release.sh" "$partial_dir" 2026.9.15-8 v2026.9.15-8 target-sha cHVi; then
  printf 'partial draft with incomplete assets passed final validation\n' >&2
  exit 1
fi

asset_dir="$fixture_root/assets"
mkdir -p "$asset_dir"
printf payload > "$asset_dir/synthetic.deb"
printf payload > "$asset_dir/synthetic.exe"
printf c2ln > "$asset_dir/synthetic.deb.sig"
printf c2ln > "$asset_dir/synthetic.exe.sig"
printf zip > "$asset_dir/ds-controller-windows-v2026.9.14-4.zip"
printf zip > "$asset_dir/ds-controller-ubuntu-v2026.9.14-4.zip"
cat > "$asset_dir/latest.json" <<'JSON'
{"version":"2026.9.14-4","platforms":{"linux-x86_64-deb":{"id":1,"url":"https://api.github.com/repos/synthetic/project/releases/assets/1","signature":"c2ln"},"windows-x86_64-nsis":{"id":2,"url":"https://api.github.com/repos/synthetic/project/releases/assets/2","signature":"c2ln"}}}
JSON
(cd "$asset_dir" && sha256sum -- *.deb *.deb.sig *.exe *.exe.sig latest.json ds-controller-*.zip > SHA256SUMS)

printf 'TEST updater URLs require the exact GitHub API asset URL\n'
for bad_url in \
  'http://api.github.com/repos/synthetic/project/releases/assets/1' \
  'https://api.example.test/repos/synthetic/project/releases/assets/1' \
  'https://api.github.com/repos/synthetic/project/releases/assets/1/extra' \
  'https://api.github.com/repos/synthetic/project/releases/assets/1?download=1' \
  'https://api.github.com/repos/synthetic/project/releases/assets/1#asset'; do
  cp "$asset_dir/latest.json" "$asset_dir/latest.json.good"
  BAD_URL="$bad_url" python3 - "$asset_dir/latest.json" <<'PY'
import json, os, pathlib, sys
p = pathlib.Path(sys.argv[1])
data = json.loads(p.read_text())
data['platforms']['linux-x86_64-deb']['url'] = os.environ['BAD_URL']
p.write_text(json.dumps(data))
PY
  if PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project VERIFY_ONLY=true MINISIGN_ASSET_DIR="$asset_dir" REMOTE_TAG_SHA=target-sha \
    bash "$repo_root/scripts/finalize-release.sh" "$asset_dir" 2026.9.14-4 v2026.9.14-4 target-sha cHVi; then
    printf 'invalid updater URL was accepted: %s\n' "$bad_url" >&2
    exit 1
  fi
  mv "$asset_dir/latest.json.good" "$asset_dir/latest.json"
done

printf 'TEST published release refuses mutating finalization\n'
if PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project MINISIGN_ASSET_DIR="$asset_dir" REMOTE_TAG_SHA=target-sha \
  bash "$repo_root/scripts/finalize-release.sh" "$asset_dir" 2026.9.14-4 v2026.9.14-4 target-sha cHVi; then
  printf 'published release entered mutating finalization\n' >&2
  exit 1
fi

draft_dir="$fixture_root/draft"
cp -R "$asset_dir" "$draft_dir"
mv "$draft_dir/ds-controller-windows-v2026.9.14-4.zip" "$draft_dir/ds-controller-windows-v2026.9.17-10.zip"
mv "$draft_dir/ds-controller-ubuntu-v2026.9.14-4.zip" "$draft_dir/ds-controller-ubuntu-v2026.9.17-10.zip"
VERSION=2026.9.17-10 python3 - "$draft_dir/latest.json" <<'PY'
import json, os, pathlib, sys
p = pathlib.Path(sys.argv[1])
data = json.loads(p.read_text())
data["version"] = os.environ["VERSION"]
p.write_text(json.dumps(data))
PY
printf 'TEST draft finalization accepts exact source before tag creation\n'
PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project MINISIGN_ASSET_DIR="$draft_dir" REMOTE_TAG_ABSENT=true REMOTE_MAIN_SHA=target-sha \
  bash "$repo_root/scripts/finalize-release.sh" "$draft_dir" 2026.9.17-10 v2026.9.17-10 target-sha cHVi

printf 'TEST published verification fails when minisign rejects a payload\n'
if PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project VERIFY_ONLY=true MINISIGN_ASSET_DIR="$asset_dir" \
  MINISIGN_FAIL_PAYLOAD=synthetic.exe REMOTE_TAG_SHA=target-sha REMOTE_MAIN_SHA=newer-main \
  bash "$repo_root/scripts/finalize-release.sh" "$asset_dir" 2026.9.14-4 v2026.9.14-4 target-sha cHVi; then
  printf 'verification failure was ignored\n' >&2
  exit 1
fi

printf 'TEST published verification ignores advanced main while preserving assets and source\n'
PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project VERIFY_ONLY=true MINISIGN_ASSET_DIR="$asset_dir" REMOTE_TAG_SHA=target-sha REMOTE_MAIN_SHA=newer-main \
  bash "$repo_root/scripts/finalize-release.sh" "$asset_dir" 2026.9.14-4 v2026.9.14-4 target-sha cHVi

printf 'all release helper tests passed\n'
