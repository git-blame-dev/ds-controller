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
if [ "$#" -eq 5 ] && [ "$1" = api ] && [ "$2" = --paginate ] \
  && [ "$3" = 'repos/synthetic/project/releases?per_page=100' ] && [ "$4" = --jq ] && [ "$5" = '.[]' ]; then
  if [ "${GH_API_FAILURE:-false}" = true ]; then
    printf 'synthetic GitHub API failure\n' >&2
    exit 75
  fi
  cat "$RELEASE_HISTORY_JSON_FIXTURE"
elif [ "$#" -eq 5 ] && [ "$1" = release ] && [ "$2" = view ] \
  && [ "$4" = --json ] && [ "$5" = assets,isDraft,isPrerelease,targetCommitish ]; then
  if [ "${GH_API_FAILURE:-false}" = true ]; then
    printf 'synthetic GitHub API failure\n' >&2
    exit 75
  fi
  case "$3" in
    v2026.9.14-4)
      printf '{"isDraft":false,"isPrerelease":false,"assets":[{"name":"synthetic.deb","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/1"},{"name":"synthetic.exe","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/2"}]}\n'
      ;;
    v2026.9.15-8|v2026.9.17-10|v2026.9.18-11)
      printf '{"isDraft":true,"isPrerelease":false,"targetCommitish":"target-sha","assets":[{"name":"synthetic.deb","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/1"},{"name":"synthetic.exe","apiUrl":"https://api.github.com/repos/synthetic/project/releases/assets/2"}]}\n'
      ;;
    *) exit 1 ;;
  esac
elif [ "$#" -eq 5 ] && [ "$1" = release ] && [ "$2" = upload ] \
  && [ "$3" = v2026.9.17-10 ] && [ "$4" = release-assets/final/SHA256SUMS ] && [ "$5" = --clobber ]; then
  if [ -n "${CHECKSUM_UPLOAD_MARKER:-}" ]; then
    touch "$CHECKSUM_UPLOAD_MARKER"
  fi
elif [ "$#" -eq 6 ] && [ "$1" = release ] && [ "$2" = edit ] && [ "$3" = v2026.9.17-10 ] \
  && [ "$4" = --draft=false ] && [ "$5" = --prerelease=false ] && [ "$6" = --latest ]; then
  printf 'release edit must not be reached\n' >&2
  touch "${RELEASE_EDIT_MARKER:?}"
else
  printf 'unexpected gh invocation: %s\n' "$*" >&2
  exit 2
fi
EOF

cat > "$mock_bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -eq 4 ] && [ "$1" = ls-remote ] && [ "$2" = --exit-code ] \
  && [ "$3" = origin ] && [ "$4" = refs/heads/main ]; then
  if [ "${REMOTE_LOOKUP_FAIL:-false}" = true ]; then
    printf 'synthetic remote lookup failure\n' >&2
    exit 128
  fi
  printf '%s\trefs/heads/main\n' "${REMOTE_MAIN_SHA:-newer-main}"
elif [ "$#" -eq 5 ] && [ "$1" = ls-remote ] && [ "$2" = --exit-code ] && [ "$3" = origin ] \
  && [[ "$4" == refs/tags/* ]] && [ "$5" = "$4^{}" ]; then
  if [ "${REMOTE_LOOKUP_FAIL:-false}" = true ]; then
    printf 'synthetic remote lookup failure\n' >&2
    exit 128
  fi
  if [ "${REMOTE_TAG_ABSENT:-false}" = true ]; then
    exit 2
  fi
  if [ "${REMOTE_TAG_ANNOTATED:-false}" = true ]; then
    printf 'tag-object\t%s\n' "$4"
    printf '%s\t%s\n' "${REMOTE_TAG_SHA:-target-sha}" "$5"
  else
    printf '%s\t%s\n' "${REMOTE_TAG_SHA:-target-sha}" "$4"
  fi
else
  printf 'unexpected git invocation: %s\n' "$*" >&2
  exit 64
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
history_json_fixture="$fixture_root/releases-objects.json"
python3 - "$history_fixture" <<'PY'
import sys
with open(sys.argv[1], "w") as fixture:
    for index in range(1, 101):
        fixture.write(f"v2026.9.1-{index}\n")
    fixture.write("v2026.9.16-1\n")
PY
python3 - "$history_fixture" "$history_json_fixture" <<'PY'
import json, pathlib, sys
tags = pathlib.Path(sys.argv[1]).read_text().splitlines()
releases = [
    {"tag_name": tag, "draft": False, "prerelease": False, "target_commitish": "published-sha"}
    for tag in tags
]
releases.extend([
    {"tag_name": "v2026.9.14-4", "draft": False, "prerelease": False, "target_commitish": "ignored-for-published"},
    {"tag_name": "v2026.9.15-8", "draft": True, "prerelease": False, "target_commitish": "target-sha"},
    {"tag_name": "v2026.9.17-10", "draft": True, "prerelease": False, "target_commitish": "target-sha"},
    {"tag_name": "v2026.9.18-11", "draft": True, "prerelease": False, "target_commitish": "target-sha"},
])
pathlib.Path(sys.argv[2]).write_text("".join(json.dumps(release) + "\n" for release in releases))
PY

run_preflight() {
  local version=$1
  shift
  PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" "$@" \
    bash "$repo_root/scripts/preflight-release.sh" "$version" target-sha
}

printf 'TEST published older rerun remains verify-only with newer history and advanced main\n'
rm -f "$fixture_root/output"
run_preflight 2026.9.14-4 env REMOTE_TAG_SHA=target-sha REMOTE_MAIN_SHA=advanced-main
if [ "$(awk -F= '$1 == "skip_mutation" { print $2 }' "$fixture_root/output")" != true ]; then
  printf 'published older rerun did not select verify-only behavior\n' >&2
  exit 1
fi

printf 'TEST complete paginated history catches release beyond first 100\n'
if run_preflight 2026.9.15-9 env REMOTE_TAG_ABSENT=true; then
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

printf 'TEST draft preflight accepts a matching annotated tag\n'
run_preflight 2026.9.18-11 env REMOTE_TAG_ANNOTATED=true REMOTE_TAG_SHA=target-sha

printf 'TEST draft preflight accepts a matching lightweight tag\n'
run_preflight 2026.9.18-11 env REMOTE_TAG_SHA=target-sha

printf 'TEST draft preflight refuses a mismatched existing tag\n'
if run_preflight 2026.9.18-11 env REMOTE_TAG_ANNOTATED=true REMOTE_TAG_SHA=wrong-sha; then
  printf 'draft preflight accepted a mismatched existing tag\n' >&2
  exit 1
fi

printf 'TEST new release preflight fails closed on remote lookup failure\n'
if run_preflight 2026.9.20-13 env REMOTE_LOOKUP_FAIL=true; then
  printf 'new release preflight swallowed remote lookup failure\n' >&2
  exit 1
fi

printf 'TEST new release preflight refuses an existing tag\n'
if run_preflight 2026.9.20-13 env REMOTE_TAG_SHA=wrong-sha; then
  printf 'new release preflight accepted an existing tag\n' >&2
  exit 1
fi

printf 'TEST preflight fails closed on GitHub API failure\n'
if run_preflight 2026.9.20-13 env GH_API_FAILURE=true REMOTE_TAG_ABSENT=true; then
  printf 'preflight swallowed GitHub API failure\n' >&2
  exit 1
fi

printf 'TEST before-publish accepts a tagless exact draft\n'
PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" \
  GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" REMOTE_TAG_ABSENT=true REMOTE_MAIN_SHA=target-sha \
  bash "$repo_root/scripts/preflight-release.sh" --before-publish 2026.9.17-10 target-sha

printf 'TEST preflight mock rejects a mutated remote\n'
mutant_preflight="$fixture_root/preflight-wrong-remote.sh"
python3 - "$repo_root/scripts/preflight-release.sh" "$mutant_preflight" <<'PY'
import pathlib, sys
source = pathlib.Path(sys.argv[1]).read_text()
mutant = source.replace("git ls-remote --exit-code origin", "git ls-remote --exit-code upstream")
if mutant == source:
    raise SystemExit("preflight remote mutation was not applied")
pathlib.Path(sys.argv[2]).write_text(mutant)
PY
if PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" \
  GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" REMOTE_TAG_ABSENT=true REMOTE_MAIN_SHA=target-sha \
  bash "$mutant_preflight" --before-publish 2026.9.17-10 target-sha; then
  printf 'release helper mocks accepted a mutated remote\n' >&2
  exit 1
fi

printf 'TEST before-publish refuses a published release\n'
if PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" \
  GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" REMOTE_TAG_SHA=target-sha REMOTE_MAIN_SHA=target-sha \
  bash "$repo_root/scripts/preflight-release.sh" --before-publish 2026.9.14-4 target-sha; then
  printf 'before-publish accepted an already-published release\n' >&2
  exit 1
fi

printf 'TEST published rerun still requires exact source\n'
if run_preflight 2026.9.14-4 env REMOTE_TAG_SHA=wrong-sha; then
  printf 'published rerun accepted the wrong source\n' >&2
  exit 1
fi

printf 'TEST published rerun refuses an absent tag\n'
if run_preflight 2026.9.14-4 env REMOTE_TAG_ABSENT=true; then
  printf 'published rerun accepted an absent tag\n' >&2
  exit 1
fi

printf 'TEST published rerun fails closed on tag lookup failure\n'
if run_preflight 2026.9.14-4 env REMOTE_LOOKUP_FAIL=true; then
  printf 'published rerun swallowed tag lookup failure\n' >&2
  exit 1
fi

printf 'TEST exact current published rerun remains verify-only\n'
published_history_json="$fixture_root/published-current.json"
printf '%s\n' '{"tag_name":"v2026.9.14-4","draft":false,"prerelease":false,"target_commitish":"ignored-for-published"}' > "$published_history_json"
published_output="$fixture_root/published-output"
PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$published_history_json" \
  GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$published_output" REMOTE_TAG_SHA=target-sha \
  bash "$repo_root/scripts/preflight-release.sh" 2026.9.14-4 target-sha
if [ "$(awk -F= '$1 == "skip_mutation" { print $2 }' "$published_output")" != true ]; then
  printf 'published rerun did not select verify-only behavior\n' >&2
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
PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" GITHUB_REPOSITORY=synthetic/project MINISIGN_ASSET_DIR="$draft_dir" REMOTE_TAG_ABSENT=true REMOTE_MAIN_SHA=target-sha \
  bash "$repo_root/scripts/finalize-release.sh" "$draft_dir" 2026.9.17-10 v2026.9.17-10 target-sha cHVi

printf 'TEST draft finalization refuses a mismatched annotated tag\n'
if PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" GITHUB_REPOSITORY=synthetic/project \
  MINISIGN_ASSET_DIR="$draft_dir" REMOTE_TAG_ANNOTATED=true REMOTE_TAG_SHA=wrong-sha REMOTE_MAIN_SHA=target-sha \
  bash "$repo_root/scripts/finalize-release.sh" "$draft_dir" 2026.9.17-10 v2026.9.17-10 target-sha cHVi; then
  printf 'draft finalization accepted a mismatched annotated tag\n' >&2
  exit 1
fi

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

printf 'TEST published verification refuses an absent tag\n'
if PATH="$mock_bin:$PATH" GITHUB_REPOSITORY=synthetic/project VERIFY_ONLY=true MINISIGN_ASSET_DIR="$asset_dir" REMOTE_TAG_ABSENT=true \
  bash "$repo_root/scripts/finalize-release.sh" "$asset_dir" 2026.9.14-4 v2026.9.14-4 target-sha cHVi; then
  printf 'published verification accepted an absent tag\n' >&2
  exit 1
fi

workflow="$repo_root/.github/workflows/ci.yml"
promotion_commands=$(awk '
  /- name: Validate draft and promote automatically$/ { in_step=1; next }
  in_step && /^      - name:/ { exit }
  in_step && /^          gh release upload "\$TAG" release-assets\/final\/SHA256SUMS --clobber$/ { emit=1 }
  emit && /^          / { sub(/^          /, ""); print }
  emit && /gh release edit/ { exit }
' "$workflow")

expected_finalize='bash scripts/finalize-release.sh release-assets/final "$VERSION" "$TAG" "$TARGET_SHA" "$public_key"'
if ! awk -v expected="$expected_finalize" '
  /- name: Validate draft and promote automatically$/ { in_step=1 }
  in_step && index($0, expected) { found=1 }
  in_step && /^      - name:/ && $0 !~ /Validate draft/ { exit }
  END { exit !found }
' "$workflow"; then
  printf 'workflow does not invoke the tested finalization entrypoint\n' >&2
  exit 1
fi

printf 'TEST workflow rechecks authoritative state after checksum upload\n'
release_edit_marker="$fixture_root/release-edit"
checksum_upload_marker="$fixture_root/checksum-upload"
set +e
PATH="$mock_bin:$PATH" RELEASE_HISTORY_FIXTURE="$history_fixture" RELEASE_HISTORY_JSON_FIXTURE="$history_json_fixture" \
  RELEASE_EDIT_MARKER="$release_edit_marker" CHECKSUM_UPLOAD_MARKER="$checksum_upload_marker" \
  GITHUB_REPOSITORY=synthetic/project GITHUB_OUTPUT="$fixture_root/output" \
  TAG=v2026.9.17-10 VERSION=2026.9.17-10 TARGET_SHA=target-sha REMOTE_TAG_ABSENT=true REMOTE_MAIN_SHA=advanced-main \
  bash -e -c "$promotion_commands"
promotion_status=$?
set -e
if [ "$promotion_status" -eq 0 ]; then
  printf 'workflow promotion commands ignored stale main\n' >&2
  exit 1
fi
if [ ! -e "$checksum_upload_marker" ]; then
  printf 'workflow test did not execute the checksum upload command\n' >&2
  exit 1
fi
if [ -e "$release_edit_marker" ]; then
  printf 'workflow attempted publication after stale-main failure\n' >&2
  exit 1
fi

printf 'all release helper tests passed\n'
