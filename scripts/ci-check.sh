#!/bin/bash
# Pre-commit CI check script
# Run this before committing to ensure all CI checks pass

set -e

echo "=== CI Pre-commit Checks ==="
echo ""

cd "$(dirname "$0")/.."

# 1. Check formatting
echo "[1/4] Checking formatting..."
if ! cargo fmt --all -- --check; then
    echo "FAILED: Formatting check failed. Run 'cargo fmt --all' to fix."
    exit 1
fi
echo "PASSED: Formatting OK"
echo ""

# 2. Clippy
echo "[2/4] Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "FAILED: Clippy found issues."
    exit 1
fi
echo "PASSED: Clippy OK"
echo ""

# 3. Tests
echo "[3/4] Running tests..."
if ! cargo test --all-features; then
    echo "FAILED: Tests failed."
    exit 1
fi
echo "PASSED: Tests OK"
echo ""

# 4. Build release
echo "[4/4] Building release..."
if ! cargo build --release --all; then
    echo "FAILED: Release build failed."
    exit 1
fi
echo "PASSED: Build OK"
echo ""

echo "=== All CI checks passed! ==="
exit 0
