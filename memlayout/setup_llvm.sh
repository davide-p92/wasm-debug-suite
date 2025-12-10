#!/usr/bin/env bash
set -e

# ==========================================================
# 🔧 LLVM Environment Auto-Configurator for Rust / llvm-sys
# ==========================================================

echo "🔍 Searching for installed LLVM versions..."
LLVM_DIR=""
for ver in 18; do
    if [ -d "/usr/lib/llvm-$ver" ]; then
        LLVM_DIR="/usr/lib/llvm-$ver"
        LLVM_VER=$ver
        break
    fi
done

if [ -z "$LLVM_DIR" ]; then
    echo "❌ No suitable LLVM installation found! Please install one (e.g. llvm-20-dev)."
    exit 1
fi

LLVM_CONFIG=$(command -v llvm-config-$LLVM_VER || true)
if [ -z "$LLVM_CONFIG" ]; then
    echo "⚠️ llvm-config-$LLVM_VER not found, trying generic llvm-config..."
    LLVM_CONFIG=$(command -v llvm-config || true)
fi

if [ -z "$LLVM_CONFIG" ]; then
    echo "❌ llvm-config not found. Please install llvm-$LLVM_VER-dev."
    exit 1
fi

# ==========================================================
# 🧠 Export environment variables for llvm-sys / inkwell / wasmer
# ==========================================================
echo "✅ Using LLVM $LLVM_VER at: $LLVM_DIR"
export LLVM_SYS_NO_Polly=1
export LLVM_SYS_180_NO_Polly=1
export LLVM_CONFIG_PATH="$LLVM_CONFIG"

# Set correct LLVM_SYS_*_PREFIX variable automatically
case "$LLVM_VER" in
  21) export LLVM_SYS_201_PREFIX="$LLVM_DIR" ;;
  20) export LLVM_SYS_200_PREFIX="$LLVM_DIR" ;;
  19) export LLVM_SYS_190_PREFIX="$LLVM_DIR" ;;
  18) export LLVM_SYS_180_PREFIX="$LLVM_DIR" ;;
esac

export PATH="$LLVM_DIR/bin:$PATH"

# ==========================================================
# 🧱 Build settings summary
# ==========================================================
echo "-----------------------------------------------------------"
echo "🔧 LLVM_SYS_NO_Polly     = $LLVM_SYS_NO_Polly"
echo "🔧 LLVM_CONFIG_PATH      = $LLVM_CONFIG_PATH"
echo "🔧 LLVM library prefix   = $LLVM_DIR/lib"
echo "🔧 Active PATH           = $(echo $PATH | cut -d: -f1-3)"
echo "-----------------------------------------------------------"

# ==========================================================
# 🛠️ Optional: build Rust project
# ==========================================================
if [ "$1" == "--build" ]; then
    echo "🧹 Cleaning Cargo build cache..."
    cargo clean
    echo "🚀 Building with LLVM backend..."
    cargo build --features llvm -vv
else
    echo "✅ Environment ready. Run:"
    echo "   source ./setup_llvm.sh --build"
fi

