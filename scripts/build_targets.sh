#!/usr/bin/env bash
set -euo pipefail

# Build nodedb-ffi for all supported target triples.
# Usage: ./scripts/build_targets.sh [--release]
#
# Requirements:
#   - Rust toolchain with cross-compilation targets installed
#   - Android NDK (set ANDROID_NDK_HOME) for Android targets
#   - Xcode for iOS/macOS targets

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$ROOT_DIR/rust"
PROFILE="${1:---release}"

TARGETS=(
    aarch64-linux-android
    armv7-linux-androideabi
    x86_64-linux-android
    aarch64-apple-ios
    aarch64-apple-ios-sim
    aarch64-apple-darwin
    x86_64-apple-darwin
    x86_64-unknown-linux-gnu
    x86_64-pc-windows-msvc
)

BUILT=0
SKIPPED=0

echo "=== NodeDB cross-platform build ==="
echo "Rust directory: $RUST_DIR"
echo ""

# Add Android NDK toolchain to PATH if available
if [ -n "${ANDROID_NDK_HOME:-}" ]; then
    TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt"
    if [ -d "$TOOLCHAIN" ]; then
        HOST_TAG=$(ls "$TOOLCHAIN" | head -1)
        export PATH="$TOOLCHAIN/$HOST_TAG/bin:$PATH"
        echo "Android NDK toolchain added to PATH"
    fi
fi

for target in "${TARGETS[@]}"; do
    # Check if target is installed
    if ! rustup target list --installed | grep -q "^${target}$"; then
        echo "SKIP $target (not installed — run: rustup target add $target)"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Skip Android targets if NDK is not configured
    case "$target" in
        *-android*)
            if [ -z "${ANDROID_NDK_HOME:-}" ]; then
                echo "SKIP $target (ANDROID_NDK_HOME not set)"
                SKIPPED=$((SKIPPED + 1))
                continue
            fi
            ;;
    esac

    echo "BUILD $target ..."
    if cargo build $PROFILE --target "$target" -p nodedb-ffi --manifest-path "$RUST_DIR/Cargo.toml" 2>&1; then
        BUILT=$((BUILT + 1))
        echo "  OK $target"
    else
        echo "  FAIL $target"
    fi
    echo ""
done

echo "=== Done: $BUILT built, $SKIPPED skipped ==="
