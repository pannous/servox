# Creating Binary Releases

## One-Command Release

```bash
./release.sh
```

This single command:
1. Locates release binary (target/release or /opt/cargo/release)
2. Creates tarball with binary and resources
3. Publishes GitHub release with auto-generated notes
4. Updates Homebrew formula in homebrew-servox tap
5. Pushes to GitHub

## Build Release Binary First

```bash
./mach build --release
# Takes 30-60 minutes depending on your machine
```

## What Gets Published

- Binary package: `servox-{VERSION}-{OS}-{ARCH}.tar.gz`
- GitHub release with feature notes
- Updated Homebrew formula with SHA256

## Installation (for users)

```bash
brew tap pannous/servox
brew install servox
```

**Installation time**: ~30 seconds (vs 30+ minutes building from source!)
