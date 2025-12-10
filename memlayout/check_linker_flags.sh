
#!/usr/bin/env bash

# Directory where Cargo stores build artifacts
TARGET_DIR="target/debug/build"

echo "🔍 Checking linker flags for Rust build artifacts in $TARGET_DIR..."

# Find all build scripts and extract rustc-link-arg lines
grep -R "cargo:rustc-link-arg" "$TARGET_DIR" | sort | uniq > linker_flags.txt

echo "✅ Extracted linker flags to linker_flags.txt"

# Check for -nodefaultlibs
if grep -q "\-nodefaultlibs" linker_flags.txt; then
    echo "❌ Found '-nodefaultlibs' in linker flags!"
    grep "\-nodefaultlibs" linker_flags.txt
else
    echo "✅ No '-nodefaultlibs' detected."
fi

# Show all linked libraries for inspection
echo "🔗 Linked libraries detected:"
grep -R "cargo:rustc-link-lib" "$TARGET_DIR" | sort | uniq

