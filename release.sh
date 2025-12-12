#!/bin/bash
# Complete Homebrew binary release workflow
set -e

VERSION="$(date +%Y.%m.%d)"
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
RELEASE_NAME="servox-${VERSION}-${OS}-${ARCH}"
TARBALL="${RELEASE_NAME}.tar.gz"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Servox Homebrew Release v${VERSION}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Step 1: Find and verify binary
echo "📦 Step 1/5: Locating release binary..."
BINARY_PATH=""
if [ -f "/opt/cargo/release/servox" ]; then
    BINARY_PATH="/opt/cargo/release/servox"
elif [ -f "target/release/servox" ]; then
    BINARY_PATH="target/release/servox"
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
rm -rf "${RELEASE_NAME}"
mkdir -p "${RELEASE_NAME}"
cp "${BINARY_PATH}" "${RELEASE_NAME}/"
cp README.md "${RELEASE_NAME}/" 2>/dev/null || true
cp -r resources "${RELEASE_NAME}/" 2>/dev/null || true

# Include GStreamer libs if present
if [ -d "/opt/cargo/release/lib" ]; then
    echo "   Including GStreamer libraries..."
    cp -r /opt/cargo/release/lib "${RELEASE_NAME}/"
elif [ -d "target/release/lib" ]; then
    echo "   Including GStreamer libraries..."
    cp -r target/release/lib "${RELEASE_NAME}/"
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
        echo "      https://github.com/pannous/servox/releases/new"
        echo "      Tag: v${VERSION}"
        echo "      Upload: ${TARBALL}"
        exit 1
    }
fi

# Create release
gh release create "v${VERSION}" \
    "${TARBALL}" \
    --repo pannous/servox \
    --title "Servox ${VERSION}" \
    --notes "Binary release with WASM GC and TypeScript support

## ✨ Features
- WebAssembly Text format (\`<script type=\"text/wast\">\`)
- TypeScript inline execution (\`<script type=\"text/typescript\">\`)
- WASM GC structs with named field access
- Direct property access (\`box.val\`, \`box[0]\`)
- toString/valueOf support for GC objects

## 🍺 Installation

\`\`\`bash
brew tap pannous/servox
brew install servox
\`\`\`

## 🧪 Quick Test

\`\`\`bash
curl -O https://raw.githubusercontent.com/pannous/servox/main/test-all.html
servox test-all.html
\`\`\`

## 📦 Platform
- **OS**: macOS ${ARCH}
- **Binary**: ${BINARY_SIZE}
- **Tarball**: ${TARBALL_SIZE}
- **Build**: Release optimized

## 🔗 Links
- [Source Code](https://github.com/pannous/servox)
- [Homebrew Tap](https://github.com/pannous/homebrew-servox)
- [All Tests](https://github.com/pannous/servox/blob/main/test-all.html)" 2>&1

if [ $? -eq 0 ]; then
    echo "   ✅ Release published!"
else
    echo "   ❌ Release creation failed"
    exit 1
fi

DOWNLOAD_URL="https://github.com/pannous/servox/releases/download/v${VERSION}/${RELEASE_NAME}.tar.gz"

# Step 4: Update Homebrew formula
echo ""
echo "🍺 Step 4/5: Updating Homebrew formula..."

# Clone or update homebrew-servox
if [ ! -d "homebrew-servox" ]; then
    echo "   Cloning homebrew-servox..."
    git clone https://github.com/pannous/homebrew-servox homebrew-servox
else
    echo "   Updating homebrew-servox..."
    cd homebrew-servox
    git pull
fi

cd homebrew-servox

# Update servox.rb
cat > servox.rb << EOF
class Servox < Formula
  desc "Servox browser with WASM GC and TypeScript support"
  homepage "https://github.com/pannous/servox"
  license "MPL-2.0"
  version "${VERSION}"

  on_macos do
    if Hardware::CPU.arm?
      url "${DOWNLOAD_URL}"
      sha256 "${SHA256}"
    end
  end

  def install
    bin.install "servox"
    (share/"servox").install "resources" if File.exist?("resources")
    (bin/"lib").install Dir["lib/*"] if File.exist?("lib")
  end

  def caveats
    <<~EOS
      🎉 Servox with WASM GC and TypeScript support!

      Features:
        • <script type="text/wast"> - WebAssembly Text format
        • <script type="text/typescript"> - TypeScript support
        • WASM GC structs with named field access
        • Direct property access: box.val, box[0]

      Quick test:
        servox https://raw.githack.com/pannous/servox/main/test-all.html

      Links:
        Live Demo: https://raw.githack.com/pannous/servox/main/test-all.html
        Source: https://github.com/pannous/servox
        Tests:  https://github.com/pannous/servox/tree/main/test-*.html
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

git add servox.rb
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
echo "   brew tap pannous/servoxx"
echo "   brew install servox"
echo ""
echo "📦 Release details:"
echo "   URL: ${DOWNLOAD_URL}"
echo "   SHA: ${SHA256}"
echo ""
echo "🔗 View release:"
echo "   https://github.com/pannous/servox/releases/tag/v${VERSION}"
echo ""
echo "⚡ Install time: ~30 seconds (instead of 30+ minutes!)"
echo ""
