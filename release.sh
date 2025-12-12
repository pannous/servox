#!/bin/bash
# Complete Homebrew binary release workflow
set -e

VERSION="${1:-$(date +%Y.%m.%d)}"
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
RELEASE_NAME="servo-${VERSION}-${OS}-${ARCH}"
TARBALL="/tmp/${RELEASE_NAME}.tar.gz"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Servo Homebrew Release v${VERSION}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Step 1: Find and verify binary
echo "📦 Step 1/5: Locating release binary..."
BINARY_PATH=""
if [ -f "/opt/cargo/release/servo" ]; then
    BINARY_PATH="/opt/cargo/release/servo"
elif [ -f "target/release/servo" ]; then
    BINARY_PATH="target/release/servo"
else
    echo "❌ No release binary found!"
    echo "   Run: ./mach build --release"
    exit 1
fi

BINARY_SIZE=$(ls -lh "${BINARY_PATH}" | awk '{print $5}')
echo "   ✅ Found: ${BINARY_PATH} (${BINARY_SIZE})"

# Step 2: Create tarball
echo ""
echo "📦 Step 2/5: Creating release tarball..."
rm -rf "/tmp/${RELEASE_NAME}"
mkdir -p "/tmp/${RELEASE_NAME}"
cp "${BINARY_PATH}" "/tmp/${RELEASE_NAME}/"
cp README.md "/tmp/${RELEASE_NAME}/" 2>/dev/null || true
cp -r resources "/tmp/${RELEASE_NAME}/" 2>/dev/null || true

# Include GStreamer libs if present
if [ -d "/opt/cargo/release/lib" ]; then
    echo "   Including GStreamer libraries..."
    cp -r /opt/cargo/release/lib "/tmp/${RELEASE_NAME}/"
elif [ -d "target/release/lib" ]; then
    echo "   Including GStreamer libraries..."
    cp -r target/release/lib "/tmp/${RELEASE_NAME}/"
fi

cd /tmp
tar -czf "${TARBALL}" "${RELEASE_NAME}"
SHA256=$(shasum -a 256 "${TARBALL}" | cut -d' ' -f1)
TARBALL_SIZE=$(ls -lh "${TARBALL}" | awk '{print $5}')

echo "   ✅ Created: ${TARBALL} (${TARBALL_SIZE})"
echo "   SHA256: ${SHA256}"

# Step 3: Create GitHub release
echo ""
echo "📤 Step 3/5: Publishing to GitHub..."

# Check if gh is authenticated with workflow scope
if ! gh auth status 2>&1 | grep -q "workflow"; then
    echo "   ⚠️  GitHub workflow scope needed"
    echo "   Running: gh auth refresh -h github.com -s workflow"
    gh auth refresh -h github.com -s workflow || {
        echo "   ❌ Auth failed. Create release manually:"
        echo "      https://github.com/pannous/servo/releases/new"
        echo "      Tag: v${VERSION}"
        echo "      Upload: ${TARBALL}"
        exit 1
    }
fi

# Create release
gh release create "v${VERSION}" \
    "${TARBALL}" \
    --repo pannous/servo \
    --title "Servo ${VERSION}" \
    --notes "Binary release with WASM GC and TypeScript support

## ✨ Features
- WebAssembly Text format (\`<script type=\"text/wast\">\`)
- TypeScript inline execution (\`<script type=\"text/typescript\">\`)
- WASM GC structs with named field access
- Direct property access (\`box.val\`, \`box[0]\`)
- toString/valueOf support for GC objects

## 🍺 Installation

\`\`\`bash
brew tap pannous/servo
brew install servo
\`\`\`

## 🧪 Quick Test

\`\`\`bash
curl -O https://raw.githubusercontent.com/pannous/servo/main/test-all.html
servo test-all.html
\`\`\`

## 📦 Platform
- **OS**: macOS ${ARCH}
- **Binary**: ${BINARY_SIZE}
- **Tarball**: ${TARBALL_SIZE}
- **Build**: Release optimized

## 🔗 Links
- [Source Code](https://github.com/pannous/servo)
- [Homebrew Tap](https://github.com/pannous/homebrew-servo)
- [All Tests](https://github.com/pannous/servo/blob/main/test-all.html)" 2>&1

if [ $? -eq 0 ]; then
    echo "   ✅ Release published!"
else
    echo "   ❌ Release creation failed"
    exit 1
fi

DOWNLOAD_URL="https://github.com/pannous/servo/releases/download/v${VERSION}/${RELEASE_NAME}.tar.gz"

# Step 4: Update Homebrew formula
echo ""
echo "🍺 Step 4/5: Updating Homebrew formula..."

# Clone or update homebrew-servo
if [ ! -d "/tmp/homebrew-servo" ]; then
    echo "   Cloning homebrew-servo..."
    git clone https://github.com/pannous/homebrew-servo /tmp/homebrew-servo
else
    echo "   Updating homebrew-servo..."
    cd /tmp/homebrew-servo
    git pull
fi

cd /tmp/homebrew-servo

# Update servo.rb
cat > servo.rb << EOF
class Servo < Formula
  desc "Servo browser engine with WASM GC and TypeScript support"
  homepage "https://github.com/pannous/servo"
  license "MPL-2.0"
  version "${VERSION}"

  on_macos do
    if Hardware::CPU.arm?
      url "${DOWNLOAD_URL}"
      sha256 "${SHA256}"
    end
  end

  def install
    bin.install "servo"
    # Install GStreamer libraries next to binary (rpath expects bin/lib/)
    if (buildpath/"lib").exist?
      (bin/"lib").install Dir["lib/*"]
    end
  end

  def caveats
    <<~EOS
      🎉 Servo with WASM GC and TypeScript support!

      Features:
        • <script type="text/wast"> - WebAssembly Text format
        • <script type="text/typescript"> - TypeScript support
        • WASM GC structs with named field access
        • Direct property access: box.val, box[0]

      Quick test:
        curl -O https://raw.githubusercontent.com/pannous/servo/main/test-all.html
        servo test-all.html

      Links:
        Source: https://github.com/pannous/servo
        Tests:  https://github.com/pannous/servo/tree/main/test-*.html
    EOS
  end

  test do
    system "#{bin}/servo", "--version"
  end
end
EOF

echo "   ✅ Formula updated"

# Step 5: Push to GitHub
echo ""
echo "📤 Step 5/5: Pushing formula to GitHub..."

git add servo.rb
git commit -m "Release v${VERSION} - binary distribution

- Binary size: ${BINARY_SIZE}
- Tarball: ${TARBALL_SIZE}
- Platform: macOS ${ARCH}
- Install time: ~30 seconds (vs 30+ min build)"

git push

echo "   ✅ Formula pushed!"

# Done!
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Release v${VERSION} Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🍺 Users can now install:"
echo "   brew tap pannous/servo"
echo "   brew install servo"
echo ""
echo "📦 Release details:"
echo "   URL: ${DOWNLOAD_URL}"
echo "   SHA: ${SHA256}"
echo ""
echo "🔗 View release:"
echo "   https://github.com/pannous/servo/releases/tag/v${VERSION}"
echo ""
echo "⚡ Install time: ~30 seconds (instead of 30+ minutes!)"
echo ""
