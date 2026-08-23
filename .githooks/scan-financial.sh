#!/bin/sh
# Scan a git diff for live personal financial data.
#
#   scan-financial.sh --cached          # staged changes (pre-commit hook)
#   scan-financial.sh <base>...<head>   # a branch's changes (CI)
#
# Exits 0 when clean, 1 when something matched (printing the offending lines).
# Set ALLOW_FINANCIAL=1 to skip the scan entirely.
set -eu

if [ "${ALLOW_FINANCIAL:-0}" = "1" ]; then
    echo "⚠️  ALLOW_FINANCIAL=1 -- skipping the personal financial data scan."
    exit 0
fi

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PATTERN=$(grep -vE '^[[:space:]]*(#|$)' "$DIR/financial-patterns.txt" | paste -sd'|' -)

if [ -z "$PATTERN" ]; then
    echo "❌ No patterns found in financial-patterns.txt -- refusing to pass vacuously."
    exit 1
fi

# These files must quote the patterns in order to define, document, or scan for
# them, so they would always match themselves. Review changes to them by eye.
HITS=$(git diff "$@" -U0 --diff-filter=ACM -- . \
        ':(exclude).githooks/*' \
        ':(exclude)AGENTS.md' \
        ':(exclude).github/workflows/financial-data-scan.yml' \
    | grep -E '^\+[^+]' \
    | grep -inE "$PATTERN" || true)

if [ -n "$HITS" ]; then
    echo "$HITS"
    echo ""
    echo "❌ The added lines above look like live portfolio data, and this repo is public."
    echo "   Scrub them, or set ALLOW_FINANCIAL=1 if this is a false positive."
    exit 1
fi

echo "✅ No personal financial data in the scanned changes."
