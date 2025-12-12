#!/bin/bash
# Test TypeScript integration in Servo-Light

set -e  # Exit on error

echo "🚀 Testing TypeScript Integration in Servo-Light"
echo "================================================"
echo ""

# Check if test file exists
if [ ! -f "test-typescript.html" ]; then
    echo "❌ Error: test-typescript.html not found!"
    exit 1
fi

# Parse command line arguments
MODE="dev"
INSTALL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            MODE="release"
            shift
            ;;
        --install)
            INSTALL=true
            shift
            ;;
        --help|-h)
            echo "Usage: ./test-typescript.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release     Build in release mode (optimized, slower build)"
            echo "  --install     Install Servo.app to /Applications (requires --release)"
            echo "  --help, -h    Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./test-typescript.sh                    # Quick dev test"
            echo "  ./test-typescript.sh --release          # Release test"
            echo "  ./test-typescript.sh --release --install  # Build, install, and test"
            exit 0
            ;;
        *)
            echo "❌ Unknown option: $1"
            echo "Run './test-typescript.sh --help' for usage information."
            exit 1
            ;;
    esac
done

# Validate install flag
if [ "$INSTALL" = true ] && [ "$MODE" != "release" ]; then
    echo "❌ Error: --install requires --release mode"
    echo "Use: ./test-typescript.sh --release --install"
    exit 1
fi

echo "📄 Test file: test-typescript.html"
echo "🔧 Mode: $MODE"
echo ""

# Build Servo
BUILD_TARGET="target/$MODE/servo"
if [ ! -f "$BUILD_TARGET" ] || [ "$MODE" = "release" ]; then
    echo "🔨 Building Servo ($MODE mode)..."
    export SERVO_ENABLE_MEDIA=0

    if [ "$MODE" = "release" ]; then
        ./mach build --release
    else
        ./mach build
    fi
    echo ""
fi

# Package and install if requested
if [ "$INSTALL" = true ]; then
    echo "📦 Packaging Servo..."
    ./mach package --release
    echo ""

    DMG_PATH="/opt/cargo/release/servo-tech-demo.dmg"

    if [ ! -f "$DMG_PATH" ]; then
        echo "❌ Error: DMG not found at $DMG_PATH"
        exit 1
    fi

    echo "💿 Mounting DMG..."
    # Unmount if already mounted
    if [ -d "/Volumes/Servo" ]; then
        diskutil unmount force /Volumes/Servo || true
    fi

    hdiutil attach "$DMG_PATH"
    echo ""

    echo "📲 Installing to /Applications..."
    # Kill any running Servo instances
    killall -SIGKILL servo 2>/dev/null || true

    # Copy to Applications
    cp -R /Volumes/Servo/Servo.app /Applications/
    echo "   ✅ Installed: /Applications/Servo.app"
    echo ""

    # Unmount
    diskutil unmount /Volumes/Servo

    echo "🌐 Opening test page in installed Servo.app..."
    TEST_PATH="$(pwd)/test-typescript.html"
    open -a /Applications/Servo.app "$TEST_PATH"
    echo ""
    echo "✅ Servo.app launched from /Applications"
    echo ""
else
    # Quick dev test
    echo "🌐 Launching Servo with TypeScript test page..."
    echo ""
    echo "Expected behavior:"
    echo "  ✓ Page should load without errors"
    echo "  ✓ All 4 TypeScript tests should show '✓ PASS'"
    echo "  ✓ Green success message at bottom"
    echo ""
    echo "If tests fail, check console for TypeScript compilation errors."
    echo ""
    echo "Press Ctrl+C to exit Servo when done testing."
    echo ""

    # Run Servo with the test page
    ./mach run test-typescript.html
fi

echo ""
echo "================================================"
echo "📝 Test Details:"
echo "   Test 1: Basic type annotations (string, number)"
echo "   Test 2: Typed functions"
echo "   Test 3: Interface definitions"
echo "   Test 4: Generic functions"
echo ""
echo "All tests use <script type=\"text/typescript\"> tags"
echo "TypeScript is compiled to JavaScript on-the-fly by Oxc"
echo "================================================"
