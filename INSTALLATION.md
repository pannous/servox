# Quick Installation & Usage Guide

## Installation Options

### Option 1: Use from Build Directory (Recommended for Development)
```bash
# Build
cd /opt/other/servo-light
./build.sh

# Run directly
./mach run test-typescript.html
```

### Option 2: Symlink to ~/.cargo/bin (Command-line Usage)
```bash
# Create symlink
ln -sf /opt/cargo/debug/servo ~/.cargo/bin/servo

# Now use from anywhere
servo --version
cd ~/projects/mysite
servo index.html
```

### Option 3: Use Packaged App
```bash
# Build and install
./build.sh

# App is installed to /Applications/Servo.app
# Open files with:
open -a /Applications/Servo.app index.html
```

### Option 4: Release Build (Production)
```bash
# Build optimized version
./build.sh --release

# Symlink release binary
ln -sf /opt/cargo/release/servo ~/.cargo/bin/servo
```

## Quick Test

### Test TypeScript
```bash
servo test-typescript.html
```

Expected output:
```
✓ Test 0: External TypeScript file
✓ Test 1: Basic TypeScript
✓ Test 2: Function with Types
✓ Test 3: Interface
✓ Test 4: Generic Function
```

### Test WebAssembly
```bash
servo test-wasm.html
```

Expected output:
```
✓ Test 0: External WAT file
✓ Test 1: Simple Addition
✓ Test 2: Multiplication
✓ Test 3: Factorial
✓ Test 4: Multiple Exports
```

## Using DevTools

### Start with DevTools
```bash
servo test-typescript.html
# DevTools server listening on 127.0.0.1:6080
```

### Connect Firefox
1. Open Firefox
2. Navigate to: `about:debugging#/setup`
3. Add network location: `localhost:6080`
4. Click "Inspect" on Servo

### Test in Console
```javascript
// Check available functions
Object.keys(window).filter(k => typeof window[k] === 'function');

// Test WASM
window.add(5, 3);        // 8
window.multiply(7, 6);   // 42
window.factorial(5);     // 120
```

## Command-line Options

```bash
# Show help
servo --help

# Set window size
servo --width=1920 --height=1080 index.html

# Enable debugging
servo --debug index.html

# Disable sandboxing (if needed)
servo --allow-file-access-from-files index.html
```

## Environment Variables

```bash
# Disable media (faster startup)
export SERVO_ENABLE_MEDIA=0
servo index.html

# Set log level
export RUST_LOG=debug
servo index.html
```

## Updating

```bash
# Pull latest changes
cd /opt/other/servo-light
git pull

# Rebuild
./build.sh

# Symlink will automatically use new binary
servo --version
```

## Uninstallation

```bash
# Remove symlink
rm ~/.cargo/bin/servo

# Remove app
rm -rf /Applications/Servo.app

# Keep source for rebuilding
cd /opt/other/servo-light
```

## Troubleshooting

### "servo: command not found"
```bash
# Check if symlink exists
ls -la ~/.cargo/bin/servo

# Recreate symlink
ln -sf /opt/cargo/debug/servo ~/.cargo/bin/servo

# Ensure ~/.cargo/bin is in PATH
echo $PATH | grep .cargo/bin
```

### "permission denied"
```bash
# Make binary executable
chmod +x /opt/cargo/debug/servo
```

### Build fails
```bash
# Clean and rebuild
./mach clean
./build.sh
```

### TypeScript not compiling
```bash
# Check logs
servo test-typescript.html 2>&1 | grep TypeScript

# Should see:
# [INFO] TypeScript: Compiling ... (N bytes)
```

### DevTools won't connect
```bash
# Check if server started
servo test.html 2>&1 | grep -i devtools
# Should see: DevTools Server listening on 127.0.0.1:6080

# Check port not in use
lsof -i :6080
```

## Git Repository

```bash
# View commit history
git log --oneline | head -5

# Check current status
git status

# Pull updates
git pull origin main

# Your changes are now on GitHub:
# https://github.com/pannous/servo
```

## Resources

- **README**: [README-TYPESCRIPT-WASM.md](./README-TYPESCRIPT-WASM.md)
- **DevTools Guide**: [DEVTOOLS-GUIDE.md](./DEVTOOLS-GUIDE.md)
- **TypeScript Tests**: [test-typescript.html](./test-typescript.html)
- **WASM Tests**: [test-wasm.html](./test-wasm.html)
- **Build Script**: [build.sh](./build.sh)

---

**Quick Start:**
```bash
cd /opt/other/servo-light
./build.sh
ln -sf /opt/cargo/debug/servo ~/.cargo/bin/servo
servo test-typescript.html
```

Done! 🚀
