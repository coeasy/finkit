#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIX=false

if [[ "${1:-}" == "--fix" ]]; then
    FIX=true
fi

get_workspace_version() {
    grep -m1 'version\s*=' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/'
}

WS_VERSION=$(get_workspace_version)
echo "=== FTA Version Consistency Check ==="
echo "Workspace version: $WS_VERSION"
echo ""

MISMATCHES=0
TOTAL=0

check_file() {
    local file="$1"
    local version="$2"
    local type="$3"
    local rel_path="${file#$ROOT/}"

    TOTAL=$((TOTAL + 1))

    local effective="$version"
    if [[ "$version" == "workspace" ]]; then
        effective="$WS_VERSION"
    fi

    if [[ "$effective" == "$WS_VERSION" ]]; then
        echo "  OK        $rel_path: $version"
    else
        echo "  MISMATCH  $rel_path: $version (expected $WS_VERSION)"
        MISMATCHES=$((MISMATCHES + 1))

        if $FIX; then
            case "$type" in
                pyproject)
                    sed -i.bak "s/version = \"[^\"]*\"/version = \"$WS_VERSION\"/" "$file"
                    rm -f "$file.bak"
                    echo "    Fixed -> $WS_VERSION"
                    ;;
                npm)
                    if command -v jq >/dev/null 2>&1; then
                        tmp=$(mktemp)
                        jq --arg v "$WS_VERSION" '.version = $v' "$file" > "$tmp" && mv "$tmp" "$file"
                        echo "    Fixed -> $WS_VERSION"
                    else
                        sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$WS_VERSION\"/" "$file"
                        rm -f "$file.bak"
                        echo "    Fixed -> $WS_VERSION"
                    fi
                    ;;
                cargo)
                    sed -i.bak "s/version = \"[^\"]*\"/version = \"$WS_VERSION\"/" "$file"
                    rm -f "$file.bak"
                    echo "    Fixed -> $WS_VERSION"
                    ;;
            esac
        fi
    fi
}

# Check Cargo.toml files
while IFS= read -r -d '' file; do
    [[ "$file" == "$ROOT/Cargo.toml" ]] && continue
    if grep -q 'version\.workspace\s*=\s*true' "$file" 2>/dev/null; then
        check_file "$file" "workspace" "cargo"
    else
        ver=$(grep -m1 'version\s*=' "$file" 2>/dev/null | sed 's/.*"\(.*\)".*/\1/' || true)
        [[ -n "$ver" ]] && check_file "$file" "$ver" "cargo"
    fi
done < <(find "$ROOT" -name "Cargo.toml" -print0 2>/dev/null)

# Check pyproject.toml files
while IFS= read -r -d '' file; do
    ver=$(grep -m1 'version\s*=' "$file" 2>/dev/null | sed 's/.*"\(.*\)".*/\1/' || true)
    [[ -n "$ver" ]] && check_file "$file" "$ver" "pyproject"
done < <(find "$ROOT" -name "pyproject.toml" -print0 2>/dev/null)

# Check package.json files
while IFS= read -r -d '' file; do
    [[ "$file" == *"node_modules"* ]] && continue
    if command -v jq >/dev/null 2>&1; then
        ver=$(jq -r '.version // empty' "$file" 2>/dev/null || true)
    else
        ver=$(grep -m1 '"version"' "$file" 2>/dev/null | sed 's/.*: *"\(.*\)".*/\1/' || true)
    fi
    [[ -n "$ver" ]] && check_file "$file" "$ver" "npm"
done < <(find "$ROOT" -name "package.json" -print0 2>/dev/null)

echo ""
echo "--- Summary ---"
echo "Total files checked: $TOTAL"
echo "Mismatches: $MISMATCHES"

if [[ $MISMATCHES -gt 0 && "$FIX" == "false" ]]; then
    echo ""
    echo "Run with --fix to automatically sync versions."
    exit 1
elif [[ $MISMATCHES -gt 0 && "$FIX" == "true" ]]; then
    echo ""
    echo "All versions synced to $WS_VERSION"
    exit 0
else
    echo ""
    echo "All versions are consistent!"
    exit 0
fi
