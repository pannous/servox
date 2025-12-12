#!/bin/bash
# Automated release publishing workflow
set -e

VERSION="${1:-$(date +%Y.%m.%d)}"

echo "🚀 Publishing Servo v${VERSION}"
echo ""

# Step 1: Create release package
echo "📦 Step 1/3: Creating release package..."
./create-release.sh "${VERSION}" > /tmp/release-info.txt
cat /tmp/release-info.txt

# Extract info
TARBALL=$(grep "Release created:" /tmp/release-info.txt | awk '{print $4}')
SHA256=$(grep "SHA256:" /tmp/release-info.txt | awk '{print $2}')

if [ -z "$TARBALL" ] || [ -z "$SHA256" ]; then
    echo "❌ Failed to create release package"
    exit 1
fi

# Step 2: Create GitHub release
echo ""
echo "📤 Step 2/3: Publishing to GitHub..."
gh release create "v${VERSION}" \
    "${TARBALL}" \
    --title "Servo ${VERSION}" \
    --notes "Binary release with WASM GC and TypeScript support

Features:
- WebAssembly Text format support
- TypeScript inline execution
- WASM GC structs with named fields
- Direct property access (box.val, box[0])

Install via Homebrew:
\`\`\`bash
brew tap pannous/servo
brew install servo
\`\`\`

Test:
\`\`\`bash
servo test-all.html
\`\`\`"

DOWNLOAD_URL="https://github.com/pannous/servo/releases/download/v${VERSION}/$(basename ${TARBALL})"

# Step 3: Update Homebrew formula
echo ""
echo "🍺 Step 3/3: Updating Homebrew formula..."

cd /tmp/homebrew-servo || {
    echo "❌ homebrew-servo not found. Clone it first:"
    echo "   git clone https://github.com/pannous/homebrew-servo /tmp/homebrew-servo"
    exit 1
}

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
    # Install resources if present
    if (buildpath/"resources").exist?
      prefix.install "resources"
    end
  end

  def caveats
    <<~EOS
      🎉 Servo with WASM GC and TypeScript support!

      Features:
        • <script type="text/wast"> - WebAssembly Text format
        • <script type="text/typescript"> - TypeScript support
        • WASM GC structs with named field access

      Try it:
        curl -O https://raw.githubusercontent.com/pannous/servo/main/test-all.html
        servo test-all.html
    EOS
  end

  test do
    system "#{bin}/servo", "--version"
  end
end
EOF

git add servo.rb
git commit -m "Release v${VERSION} - binary distribution"
git push

echo ""
echo "✅ Release v${VERSION} published!"
echo ""
echo "Users can now install with:"
echo "  brew tap pannous/servo"
echo "  brew install servo"
echo ""
echo "Installation time: ~30 seconds (instead of 30+ minutes!)"
