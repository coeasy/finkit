#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit unified usage-package builder + verifier.
#
# Replaces (or supersedes) the legacy `build-all-packages.sh`. For each
# enabled language it:
#   1. Invokes `scripts/build-usage-<lang>.sh` to produce the installable
#      artifact in dist/<lang>/<plat>/.
#   2. Invokes `packaging/usage/<lang>/tests/*` to **verify the artifact
#      by actually installing + running it** (not by re-running the source
#      tree's smoke tests).
#   3. Emits a SHA256 + size manifest to dist/manifest.json.
#   4. Bundles the full tree into
#      dist/finkit-<version>-<plat>-usage-bundle.zip
#
# Usage:
#   ./scripts/build-usage-packages.sh                        # all languages
#   ./scripts/build-usage-packages.sh python node            # subset
#   ./scripts/build-usage-packages.sh --verify               # only verify
#   ./scripts/build-usage-packages.sh --no-bundle            # skip zip
#   ./scripts/build-usage-packages.sh --no-verify            # build only
#   ./scripts/build-usage-packages.sh node --no-clean        # keep existing trees
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
DIST="${ROOT}/dist"
VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"

# platform normalize --------------------------------------------------------
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*)
    PLATFORM="windows-x64"
    PY_PY="python"
    PY_PIP="pip" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)  PLATFORM="macos-arm64" ;;
      x86_64) PLATFORM="macos-x64"   ;;
    esac
    PY_PY="python3"; PY_PIP="pip3" ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64"   ;;
    esac
    PY_PY="python3"; PY_PIP="pip3" ;;
  *) echo "unsupported platform: $( uname -s )" >&2; exit 1 ;;
esac

# --- CLI --------------------------------------------------------------------
LANGS=( "python" "node" "java" "go" "c" "dotnet" "wasm" )
WANT=()
DO_VERIFY=1
DO_BUNDLE=1
DO_CLEAN=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    python|node|java|go|c|dotnet|wasm) WANT+=( "$1" ); shift ;;
    --verify) DO_BUNDLE=0; shift ;;
    --no-bundle) DO_BUNDLE=0; shift ;;
    --no-verify) DO_VERIFY=0; shift ;;
    --no-clean) DO_CLEAN=0; shift ;;
    -h|--help) sed -n '2,18p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done
[[ ${#WANT[@]} -eq 0 ]] && WANT=( "${LANGS[@]}" )

# --- helpers ----------------------------------------------------------------
ok()    { echo -e "  \033[32m[OK]\033[0m   $*"; }
err()   { echo -e "  \033[31m[FAIL]\033[0m $*" >&2; }
info()  { echo -e "  \033[36m[INFO]\033[0m $*"; }
hdr()   { echo -e "\n\033[1;36m=== $* ===\033[0m"; }

# Convert an MSYS/POSIX path into the form the native toolchain can open.
#
# This is not cosmetic. Under Git Bash on Windows `$ROOT` is `/p/repo` — no
# drive letter. Handed to Windows Python as `pathlib.Path("/p/repo/dist")` it
# becomes `\p\repo\dist`, rooted at the current drive and nonexistent, so the
# manifest step died with
#   FileNotFoundError: [Errno 2] No such file or directory: '\p\...\dist\manifest.json'
# *after* the C and Go builds had already succeeded — the artifacts existed and
# simply were not recorded, which is why `refresh_release_manifests.py --check`
# reported them missing and why the checked-in record could not be regenerated.
# Go's `replace` directive has the same requirement. `cygpath` ships with
# MSYS/Git Bash; on Linux and macOS the path is already native, so the branch is
# skipped and the helper is a no-op.
native_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s' "$1"
  fi
}

# Each language: (build_fn, verify_fn, output_glob)
declare -A BUILDERS
BUILDERS[python]="build_python"
BUILDERS[node]="build_node"
BUILDERS[java]="build_java"
BUILDERS[go]="build_go"
BUILDERS[c]="build_c"
BUILDERS[dotnet]="build_dotnet"
BUILDERS[wasm]="build_wasm"

declare -A VERIFIERS
VERIFIERS[python]="verify_python"
VERIFIERS[node]="verify_node"
VERIFIERS[java]="verify_java"
VERIFIERS[go]="verify_go"
VERIFIERS[c]="verify_c"
VERIFIERS[dotnet]="verify_dotnet"
VERIFIERS[wasm]="verify_wasm"

# --- per-language wrappers --------------------------------------------------

build_python() {
  "${SCRIPT_DIR}/build-usage-python.sh" >/dev/null
}

build_node() {
  "${SCRIPT_DIR}/build-usage-node.sh" >/dev/null
}

build_java() {
  "${SCRIPT_DIR}/build-usage-java.sh" >/dev/null
}

build_go() {
  "${SCRIPT_DIR}/build-usage-go.sh" >/dev/null
}

build_c() {
  "${SCRIPT_DIR}/build-usage-c.sh" >/dev/null
}

build_dotnet() {
  "${SCRIPT_DIR}/build-usage-dotnet.sh" >/dev/null
}

build_wasm() {
  "${SCRIPT_DIR}/build-usage-wasm.sh" >/dev/null
}

verify_python() {
  local whl
  whl="$( find "${DIST}/python/${PLATFORM}" -name 'finkit-*-abi3-*.whl' 2>/dev/null | head -1 )"
  if [[ -z "${whl}" ]]; then
    err "no abi3 wheel to verify"
    return 1
  fi
  local venv="${ROOT}/.test_venv/_usage_python"
  rm -rf "${venv}"
  "${PY_PY}" -m venv "${venv}"
  case "$( uname -s )" in
    MINGW*|MSYS*|CYGWIN*) PIP="${venv}/Scripts/pip.exe"; PY="${venv}/Scripts/python.exe" ;;
    *)                     PIP="${venv}/bin/pip";          PY="${venv}/bin/python" ;;
  esac
  "${PIP}" install --quiet "${whl}"
  "${PY}" "${ROOT}/packaging/usage/python/verify_install.py"
}

verify_node() {
  local tgz
  tgz="$( find "${DIST}/node/${PLATFORM}" -name 'finkit-*.tgz' 2>/dev/null | head -1 )"
  if [[ -z "${tgz}" ]]; then
    err "no tgz to verify"
    return 1
  fi
  local scratch="${ROOT}/.test_venv/_usage_node"
  rm -rf "${scratch}"
  mkdir -p "${scratch}"
  ( cd "${scratch}" && npm init -y >/dev/null && npm install --silent "${tgz}" )
  cp "${ROOT}/packaging/usage/node/verify_install.js" "${scratch}/"
  ( cd "${scratch}" && node verify_install.js )
}

verify_java() {
  local jar
  jar="$( find "${DIST}/java/${PLATFORM}" -name 'finkit-*.jar' 2>/dev/null | head -1 )"
  if [[ -z "${jar}" ]]; then
    err "no jar to verify"
    return 1
  fi
  local tmp="${ROOT}/.test_venv/_usage_java"
  rm -rf "${tmp}"; mkdir -p "${tmp}"
  cp "${ROOT}/packaging/usage/java/verify_install.java" "${tmp}/"
  ( cd "${tmp}" && javac -cp "${jar}" verify_install.java && java -cp ".:${jar}" verify_install )
}

verify_go() {
  # Rewrite the replace directive in tests/go.mod to point at the built tree.
  local mod="${ROOT}/packaging/usage/go/tests/go.mod"
  local go_dist="${DIST}/go/${PLATFORM}/finkit"
  if [[ ! -d "${go_dist}" ]]; then
    err "no go module to verify"
    return 1
  fi
  # Replace the entire replace directive. Both paths cross a tool boundary --
  # one into Python, one into `go.mod` -- so both are passed as environment
  # variables (see `native_path`): an MSYS `/p/...` path is neither openable by
  # Windows Python nor acceptable to Windows Go. Forward slashes are written
  # into `go.mod` because Go accepts them on every platform, which avoids
  # backslash escaping inside the module file.
  MOD_NATIVE="$(native_path "${mod}")" \
  GO_DIST_NATIVE="$(native_path "${go_dist}")" \
  python3 - <<'PY'
import os, pathlib, re

p = pathlib.Path(os.environ["MOD_NATIVE"])
text = p.read_text(encoding="utf-8")
replacement = ("replace github.com/coeasy/finkit => "
               + os.environ["GO_DIST_NATIVE"].replace("\\", "/"))
text = re.sub(r"replace github\.com/coeasy/finkit => .*", replacement, text)
p.write_text(text, encoding="utf-8")
print(f"[go] replace -> {replacement.split('=> ')[1]}")
PY
  ( cd "${ROOT}/packaging/usage/go/tests" && go run ../verify_install.go )
}

verify_c() {
  bash "${ROOT}/packaging/usage/c/tests/test_c_install.sh"
}

verify_dotnet() {
  local nupkg
  nupkg="$( find "${DIST}/dotnet/${PLATFORM}" -name 'Finkit.*.nupkg' 2>/dev/null | head -1 )"
  if [[ -z "${nupkg}" ]]; then
    err "no nupkg to verify"
    return 1
  fi
  # Set up a local feed + restore
  local feed="${ROOT}/.test_venv/_usage_dotnet_feed"
  rm -rf "${feed}"; mkdir -p "${feed}"
  cp "${nupkg}" "${feed}/"
  ( cd "${ROOT}/packaging/usage/dotnet/tests" && \
    dotnet restore --source "${feed}" --force 2>&1 | tail -3 && \
    dotnet run --no-restore 2>&1 | tail -10 )
}

verify_wasm() {
  # WASM Node.js bundle smoke (web bundle needs a browser; documented).
  ( cd "${ROOT}/packaging/usage/wasm/tests" && node verify_install.js )
}

# --- main --------------------------------------------------------------------

hdr "Finkit usage-package builder"
info "version : ${VERSION}"
info "platform: ${PLATFORM}"
info "langs   : ${WANT[*]}"
info "verify  : ${DO_VERIFY}"
info "bundle  : ${DO_BUNDLE}"
info "clean   : ${DO_CLEAN}"

mkdir -p "${DIST}"

fail_count=0
ok_count=0
skip_count=0

for lang in "${WANT[@]}"; do
  if [[ ! " ${LANGS[*]} " == *" ${lang} "* ]]; then
    err "unknown language: ${lang}"
    fail_count=$(( fail_count + 1 ))
    continue
  fi

  hdr "[${lang}] build"

  # Clean this language's output tree before building it.
  #
  # `dist/` is gitignored and accumulates across runs, and the manifest step
  # below walks it with `rglob`, so anything an older build left behind is
  # recorded as *this* run's output. That is not hypothetical: the tree on this
  # host still held `dist/go/windows-x64/AlphaTA/`, `lib/alpha_ta_go.dll` and
  # `alpha_ta.h` from the product's previous name, and the first successful
  # manifest write duly recorded all nine of them as finkit 0.2.0 components.
  # A record that describes two product generations at once is the same defect
  # as the stale top-level manifest -- quieter, because each digest is correct.
  #
  # `${VAR:?}` rather than `${VAR}`: this is the only `rm -rf` in the script
  # whose argument is assembled from three variables, and an unset `PLATFORM`
  # would silently turn it into `rm -rf /dist/<lang>/`, or worse.
  #
  # `--no-clean` keeps whatever is already there, which is what you want while
  # iterating on one language. It also means the manifest then describes the
  # whole tree rather than this run's output, so it is not a release setting.
  if [[ "${DO_CLEAN}" -eq 1 ]]; then
    rm -rf "${DIST:?}/${lang:?}/${PLATFORM:?}"
  else
    info "keeping the existing ${lang} tree (--no-clean)"
  fi

  if "${BUILDERS[$lang]}"; then
    ok "${lang} build"
  else
    err "${lang} build failed"
    fail_count=$(( fail_count + 1 ))
    continue
  fi

  if [[ "${DO_VERIFY}" -eq 1 ]]; then
    hdr "[${lang}] verify (install + run)"
    if "${VERIFIERS[$lang]}"; then
      ok "${lang} verify"
      ok_count=$(( ok_count + 1 ))
    else
      err "${lang} verify failed"
      fail_count=$(( fail_count + 1 ))
    fi
  fi
done

# --- manifest ----------------------------------------------------------------

hdr "manifest"
LANGS_CSV="$( IFS=,; echo "${WANT[*]}" )"
# Quoted heredoc + environment variables, deliberately: the previous version
# interpolated `${DIST}` straight into the Python source, which is what turned
# the Windows path into `\p\...` and killed the step. Nothing here should have
# to reason about shell escaping or drive letters.
DIST_NATIVE="$(native_path "${DIST}")" \
PLATFORM="${PLATFORM}" \
LANGS_CSV="${LANGS_CSV}" \
VERSION="${VERSION}" \
python3 - <<'PY'
import hashlib, json, os, pathlib

root = pathlib.Path(os.environ["DIST_NATIVE"])
langs = os.environ["LANGS_CSV"].split(",")
platform = os.environ["PLATFORM"]
root.mkdir(parents=True, exist_ok=True)

components = []
for lang in langs:
    base = root / lang / platform
    if not base.exists():
        continue
    for f in sorted(base.rglob("*")):
        if not f.is_file():
            continue
        if any(part.startswith('.') for part in f.parts):
            continue
        if f.suffix in {".pyc"}:
            continue
        components.append({
            "language": lang,
            "platform": platform,
            "path": str(f.relative_to(root)).replace(os.sep, "/"),
            "size_bytes": f.stat().st_size,
            "sha256": hashlib.sha256(f.read_bytes()).hexdigest(),
        })

# Coverage is what the run actually produced, not what it was asked for and not
# which directories happen to exist. Deriving it from directories was wrong in a
# way worth recording: a language whose build failed leaves an *empty*
# directory behind, `base.exists()` is then True, and the manifest claimed
# coverage for a language that contributed zero components -- python and c, on
# the run that found this. An over-claiming coverage field is worse than none,
# because it is the field a reader trusts to know what is in the file.
covered = sorted({component["language"] for component in components})
uncovered = [lang for lang in langs if lang not in covered]

# `languages` records what this run actually covered. Without it a subset build
# rewrote the manifest into a record that *looks* authoritative while holding
# only the two languages it was asked for and silently dropping every other
# one -- which is how the file came to describe a product that no longer exists
# without anything saying so.
manifest = {
    # Lowercase, matching the Cargo package name in `core/Cargo.toml` and the
    # other two writers (`build-quick.sh`, `build-quick.ps1`). This script used
    # to write "Finkit" while those wrote "finkit", so the same file had two
    # spellings of the same field depending on which script produced it -- and
    # nothing could check the field against the repository, because there was no
    # single expected value to check it against.
    "name": "finkit",
    "version": os.environ["VERSION"],
    "platform": platform,
    "languages": covered,
    "components": components,
}
out = root / "manifest.json"
out.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
print(f"wrote {out} ({len(components)} components, "
      f"languages: {', '.join(covered) or '(none)'})")
if uncovered:
    print(f"note: no components for: {', '.join(uncovered)} "
          f"(build failed, or produced nothing this run)")
PY

# --- bundle ------------------------------------------------------------------

if [[ "${DO_BUNDLE}" -eq 1 && "${fail_count}" -eq 0 ]]; then
  hdr "bundle"
  BUNDLE="${DIST}/finkit-${VERSION}-${PLATFORM}-usage-bundle.zip"
  if command -v zip >/dev/null 2>&1; then
    ( cd "${DIST}" && zip -qr "${BUNDLE}" \
        python/${PLATFORM} \
        node/${PLATFORM}  \
        java/${PLATFORM}  \
        go/${PLATFORM}    \
        c/${PLATFORM}     \
        dotnet/${PLATFORM}\
        wasm              \
        manifest.json )
    ok "bundle: ${BUNDLE}"
  elif command -v powershell >/dev/null 2>&1; then
    powershell -NoProfile -Command "Compress-Archive -Path '${DIST}\\python\\${PLATFORM}','${DIST}\\node\\${PLATFORM}','${DIST}\\java\\${PLATFORM}','${DIST}\\go\\${PLATFORM}','${DIST}\\c\\${PLATFORM}','${DIST}\\dotnet\\${PLATFORM}','${DIST}\\wasm','${DIST}\\manifest.json' -DestinationPath '${BUNDLE}' -Force"
    ok "bundle (ps1): ${BUNDLE}"
  else
    err "no zip or powershell found, skipping bundle"
  fi
fi

hdr "summary"
echo "  built+verified: ${ok_count}"
echo "  failed        : ${fail_count}"
echo "  skipped       : ${skip_count}"
[[ "${fail_count}" -eq 0 ]]
