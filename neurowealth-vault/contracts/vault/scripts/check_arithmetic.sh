#!/usr/bin/env bash
# =============================================================================
# check_arithmetic.sh
#
# CI guard that surfaces bare (unchecked) arithmetic in the vault contract
# source files.  It is designed to FAIL when a risky arithmetic path is
# introduced so the problem is caught before merge.
#
# What it checks
# ──────────────
# 1. Clippy lint: `clippy::integer_arithmetic` — rejects bare `+`, `-`, `*`
#    between integer types (overflow-sensitive operations that must use
#    checked_add / checked_sub / checked_mul / saturating_* / wrapping_*).
#
# 2. Grep sweep — catches patterns that Clippy sometimes misses in macro
#    expansions or when the lint is accidentally suppressed:
#      a) Bare `a + b`, `a - b`, `a * b` on i128/u128/u64/i64 values
#         (heuristic: detects the most common forms)
#      b) `as i128` / `as u64` casts that can silently truncate
#      c) `unwrap()` on arithmetic Results (e.g. `.checked_add(...).unwrap()`)
#         which defeats the purpose of checked math — `expect()` with a
#         message is the required alternative.
#
# Usage
# ─────
#   # From repo root:
#   bash contracts/vault/scripts/check_arithmetic.sh
#
#   # Or from contracts/vault/:
#   bash scripts/check_arithmetic.sh
#
# Exit codes
# ──────────
#   0 — No issues found.
#   1 — One or more checks failed.  Details are printed to stdout.
#
# Adding exceptions
# ─────────────────
# If a grep match is a false positive (e.g. a comment, a test helper, or a
# deliberate saturating operation already covered by a different check), add
# the file:line to GREP_EXCEPTIONS below.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

# Resolve script location so the script works regardless of CWD.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Vault root is one directory up from scripts/
VAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_DIR="$VAULT_ROOT/src"

# Files/patterns to skip in grep checks (space-separated "file:pattern" pairs).
# Add entries here when a match is a confirmed false positive.
GREP_EXCEPTIONS=(
    # Example: "src/tests/utils.rs:as i128"
)

FAILED=0  # track overall pass/fail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

section() { echo; echo "════════════════════════════════════════"; echo "  $*"; echo "════════════════════════════════════════"; }
pass()    { echo "  ✅  $*"; }
fail()    { echo "  ❌  $*"; FAILED=1; }
info()    { echo "  ℹ   $*"; }

is_exception() {
    local match="$1"
    for exc in "${GREP_EXCEPTIONS[@]:-}"; do
        if [[ "$match" == *"$exc"* ]]; then
            return 0
        fi
    done
    return 1
}

# Run grep across vault src; filter exceptions; return non-zero if hits remain.
grep_check() {
    local description="$1"
    local pattern="$2"
    shift 2
    local extra_args=("$@")

    local raw_hits
    # grep exits 1 when no matches — that's fine here, don't abort.
    raw_hits=$(grep -rn "${extra_args[@]}" "$pattern" "$SRC_DIR" 2>/dev/null || true)

    local hits=""
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        is_exception "$line" || hits+="$line"$'\n'
    done <<< "$raw_hits"

    if [[ -n "$hits" ]]; then
        fail "$description"
        echo "$hits" | sed 's/^/      /'
        return 1
    else
        pass "$description"
        return 0
    fi
}

# ---------------------------------------------------------------------------
# Check 1 — Clippy integer_arithmetic
# ---------------------------------------------------------------------------

section "Check 1: Clippy integer_arithmetic lint"

info "Running: cargo clippy -p neurowealth-vault -- -D clippy::integer_arithmetic"

# We forbid the lint at the clippy invocation level so it cannot be suppressed
# in source with #[allow(clippy::integer_arithmetic)] without being caught by
# check 2 below.
if (
    cd "$VAULT_ROOT"
    cargo clippy \
        --all-features \
        -- \
        -D clippy::integer_arithmetic \
        2>&1
); then
    pass "Clippy integer_arithmetic: no violations"
else
    fail "Clippy integer_arithmetic: violations found (see output above)"
    FAILED=1
fi

# ---------------------------------------------------------------------------
# Check 2 — Ensure the lint cannot be silently suppressed in source
# ---------------------------------------------------------------------------

section "Check 2: No source-level suppression of integer_arithmetic"

grep_check \
    "No #[allow(clippy::integer_arithmetic)] in source" \
    'allow(clippy::integer_arithmetic)' \
    --include='*.rs'

# ---------------------------------------------------------------------------
# Check 3 — Detect bare addition / subtraction / multiplication on integers
#           (heuristic grep for the most common forms in contract logic)
# ---------------------------------------------------------------------------

section "Check 3: No bare integer arithmetic operators (heuristic)"

# Pattern explanation:
#   [0-9a-z_] [+\-\*] [0-9a-z_]   — operands around the operator
# We exclude:
#   - lines that are pure comments (//)
#   - lines inside string literals (hard to exclude perfectly; we flag and review)
#   - the saturating_* / checked_* / wrapping_* methods (safe alternatives)
# NOTE: This is a heuristic.  The Clippy check above is authoritative; this
#       grep is a belt-and-suspenders fallback for macro-expanded code.

BARE_OPS_PATTERN='[0-9a-z_]\s*[+\-\*]\s*[0-9a-z_]'

info "Scanning for bare arithmetic operators outside checked/saturating methods..."

# We want to exclude lines that already use checked/saturating/wrapping and
# lines that are comments.
raw=$(grep -rn --include='*.rs' -E "$BARE_OPS_PATTERN" "$SRC_DIR" \
      | grep -v '^\s*//' \
      | grep -v 'checked_add\|checked_sub\|checked_mul\|checked_div\|saturating_\|wrapping_\|//.*[+\-\*]' \
      | grep -v 'test\|#\[' \
      || true)

if [[ -n "$raw" ]]; then
    # Secondary filter: only flag lines from the main lib.rs / non-test files
    # (test arithmetic isn't safety-critical in the same way).
    critical=$(echo "$raw" | grep -v '/tests/' || true)
    if [[ -n "$critical" ]]; then
        fail "Potential bare arithmetic found in contract source (review each line):"
        echo "$critical" | head -30 | sed 's/^/      /'
        info "(Showing first 30 matches.  If these are false positives, add to GREP_EXCEPTIONS.)"
        FAILED=1
    else
        pass "No bare arithmetic in contract source (test files excluded)"
    fi
else
    pass "No bare arithmetic operators detected"
fi

# ---------------------------------------------------------------------------
# Check 4 — Detect .unwrap() on checked arithmetic Results
# ---------------------------------------------------------------------------

section "Check 4: No .unwrap() on checked arithmetic results"

# .checked_add(...).unwrap() silently panics without a message — use .expect()
grep_check \
    "No .checked_*.unwrap() (use .expect(\"msg\") instead)" \
    '\.checked_[a-z_]*([^)]*)\s*\.\s*unwrap()' \
    --include='*.rs' \
    -E

# ---------------------------------------------------------------------------
# Check 5 — Detect potentially truncating `as` casts
# ---------------------------------------------------------------------------

section "Check 5: No silent truncating 'as' casts on integer types"

grep_check \
    "No bare 'as i128 / as u64 / as u32 / as i64' casts in contract source" \
    '\bас\s\+\(i128\|u128\|u64\|i64\|u32\|i32\)\b' \
    --include='*.rs'

# Also catch the ASCII 'as' (previous used Cyrillic 'с' — reset to ASCII).
raw_as=$(grep -rn --include='*.rs' \
         -E '\bas\s+(i128|u128|u64|i64|u32|i32)\b' "$SRC_DIR" \
         | grep -v '^\s*//' \
         | grep -v '/tests/' \
         || true)
if [[ -n "$raw_as" ]]; then
    fail "Silent 'as' casts detected — use try_into() / checked conversions:"
    echo "$raw_as" | sed 's/^/      /'
    FAILED=1
else
    pass "No truncating 'as' casts in contract source"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

section "Summary"

if [[ $FAILED -eq 0 ]]; then
    echo "  ✅  All arithmetic checks passed."
    exit 0
else
    echo "  ❌  One or more arithmetic checks FAILED."
    echo "      Fix the issues above and re-run this script before merging."
    exit 1
fi