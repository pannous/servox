#!/bin/bash
# Post-build script to fix library paths on macOS

BINARY_DEBUG="/opt/cargo/debug/servox"
BINARY_RELEASE="/opt/cargo/release/servox"

fix_binary() {
    local binary="$1"
    if [ ! -f "$binary" ]; then
        return
    fi

    echo "Fixing library paths for $binary..."

    # Add homebrew lib path to rpath if not already present
    if ! otool -l "$binary" | grep -q "/opt/homebrew/lib"; then
        install_name_tool -add_rpath /opt/homebrew/lib "$binary" 2>/dev/null || true
    fi

    # Create symlinks for versioned libraries
    for lib in libz.1.dylib libfreetype.6.dylib libharfbuzz.0.dylib libpng16.16.dylib; do
        base=$(echo "$lib" | sed -E 's/\.[0-9]+\.dylib/.dylib/')
        if [ -f "/opt/homebrew/lib/$base" ] && [ ! -f "/opt/homebrew/lib/$lib" ]; then
            ln -sf "$base" "/opt/homebrew/lib/$lib"
        fi
    done

    echo "Done fixing $binary"
}

fix_binary "$BINARY_DEBUG"
fix_binary "$BINARY_RELEASE"
