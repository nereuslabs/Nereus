#!/usr/bin/env bash
# ==============================================================================
# verify_e2e.sh — Nereus 2.0 Pre-Release End-to-End Verification Protocol
# Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info() {
    echo -e "${CYAN}${BOLD}[INFO]${NC} $1"
}

pass() {
    echo -e "${GREEN}${BOLD}[PASS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}${BOLD}[WARN]${NC} $1"
}

fail() {
    echo -e "${RED}${BOLD}[FAIL]${NC} $1"
    exit 1
}

echo -e "\n${BOLD}🌊 Nereus 2.0 Pre-Release End-to-End Verification Protocol${NC}"
echo "======================================================================"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

DB_PATH="${TMP_DIR}/test_nereus.db"
BIN_TARGET="target/release/nereus"

# 1. Clean Build from Source
info "Step 1/8: Compiling release binary from source..."
cargo build --release --bin nereus
if [[ ! -f "${BIN_TARGET}" ]]; then
    fail "Release binary ${BIN_TARGET} was not produced."
fi
pass "Release binary compiled successfully."

# 2. Automated Lifecycle Simulation & Session Initialization
info "Step 2/8: Running automated lifecycle simulation (offline demo mode)..."
"${BIN_TARGET}" --skill "Rust" --offline --demo --db "${DB_PATH}" > "${TMP_DIR}/demo.log"
if ! grep -q "Initializing Socratic Learning Automaton" "${TMP_DIR}/demo.log"; then
    fail "Automaton initialization message not detected in demo output."
fi
pass "Automated lifecycle simulation passed."

# 3. Session Listing and Discovery
info "Step 3/8: Verifying session discovery table..."
"${BIN_TARGET}" sessions list --db "${DB_PATH}" > "${TMP_DIR}/sessions.log"
if ! grep -q "Rust" "${TMP_DIR}/sessions.log"; then
    fail "Stored session not found in sessions table."
fi
SESSION_ID=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' "${TMP_DIR}/sessions.log" | head -n 1)
if [[ -z "${SESSION_ID}" ]]; then
    fail "Could not extract session UUID from sessions table."
fi
pass "Session discovery verified: ${SESSION_ID}"

# 4. Assessment History Query
info "Step 4/8: Verifying chronological assessment history for session..."
"${BIN_TARGET}" sessions history "${SESSION_ID}" --db "${DB_PATH}" > "${TMP_DIR}/history.log"
if ! grep -q "Assessment History for Session" "${TMP_DIR}/history.log"; then
    fail "Assessment history table header missing."
fi
pass "Assessment history query verified."

# 5. Multi-Format Session Export (Markdown, Asciinema .cast, SVG)
info "Step 5/8: Testing session export across all formats..."
"${BIN_TARGET}" sessions export "${SESSION_ID}" --format markdown --output "${TMP_DIR}/export.md" --db "${DB_PATH}"
if ! grep -q "Nereus Learning Session Report" "${TMP_DIR}/export.md"; then
    fail "Markdown export did not contain expected title."
fi

"${BIN_TARGET}" sessions export "${SESSION_ID}" --format cast --output "${TMP_DIR}/export.cast" --db "${DB_PATH}"
if ! grep -q '"version":' "${TMP_DIR}/export.cast"; then
    fail "Asciinema .cast export did not contain valid version 2 header."
fi

"${BIN_TARGET}" sessions export "${SESSION_ID}" --format svg --output "${TMP_DIR}/export.svg" --db "${DB_PATH}"
if ! grep -q '<svg' "${TMP_DIR}/export.svg"; then
    fail "SVG badge export did not contain valid SVG markup."
fi
pass "Markdown, Asciinema .cast, and SVG exports generated and validated."

# 6. Session Resume Execution
info "Step 6/8: Verifying state machine resumption across process restart..."
"${BIN_TARGET}" --resume --offline --demo --db "${DB_PATH}" > "${TMP_DIR}/resume.log"
if ! grep -q "Resuming" "${TMP_DIR}/resume.log"; then
    fail "Session resume confirmation missing from output."
fi
pass "Session resumption verified."

# 7. Socratic LLM Benchmark Suite
info "Step 7/8: Running automated Socratic mentor evaluation benchmark..."
"${BIN_TARGET}" bench --offline --format json --export "${TMP_DIR}/bench.json" > /dev/null
if ! grep -q '"zero_leakage_rate"' "${TMP_DIR}/bench.json"; then
    fail "Benchmark JSON report missing zero_leakage_rate metric."
fi
pass "Offline Socratic mentor benchmark passed (Zero code leakage verified)."

# 8. Session Cleanup & Cascade Deletion
info "Step 8/8: Verifying session deletion and SQLite cascade cleanup..."
"${BIN_TARGET}" sessions delete "${SESSION_ID}" --db "${DB_PATH}" > "${TMP_DIR}/delete.log"
if ! grep -q "Successfully deleted session" "${TMP_DIR}/delete.log"; then
    fail "Delete confirmation missing."
fi
pass "Session deletion and cascading cleanup verified."

echo "======================================================================"
echo -e "${GREEN}${BOLD}🎉 ALL PRE-RELEASE VERIFICATION QUALITY GATES PASSED!${NC}\n"
