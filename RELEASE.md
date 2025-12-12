# Creating Binary Releases

## Build Release Binary

```bash
./mach build --release
# This takes 30-60 minutes depending on your machine
```

## Create Release Package

```bash
./create-release.sh 2025.12.12
```

This creates:
- `/tmp/servo-2025.12.12-darwin-arm64.tar.gz`
- Calculates SHA256 hash
- Provides GitHub release command

## Publish to GitHub

```bash
# Use the command from create-release.sh output
gh release create v2025.12.12 \
  /tmp/servo-2025.12.12-darwin-arm64.tar.gz \
  --title "Servo 2025.12.12" \
  --notes "Binary release with WASM GC and TypeScript support"
```

## Update Homebrew Formula

1. Update `servo.rb` in homebrew-servo repo:
   ```ruby
   url "https://github.com/pannous/servo/releases/download/v2025.12.12/servo-2025.12.12-darwin-arm64.tar.gz"
   sha256 "abc123..."  # Use SHA from create-release.sh
   ```

2. Commit and push:
   ```bash
   cd ~/homebrew-servo
   git add servo.rb
   git commit -m "Update to v2025.12.12"
   git push
   ```

3. Test installation:
   ```bash
   brew upgrade servo
   servo test-all.html
   ```

## Quick Install (for users)

```bash
brew tap pannous/servo
brew install servo
```

**Installation time**: ~30 seconds (vs 30+ minutes building from source!)
