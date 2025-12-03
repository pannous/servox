# Firefox DevTools Connection Guide for Servo

## Quick Start

### 1. Start Servo
```bash
./mach run test-typescript.html
# DevTools server will start on port 6080
```

### 2. Connect Firefox DevTools

**Method 1: Using about:debugging (Recommended)**

1. Open Firefox
2. Type in address bar: `about:debugging#/setup`
3. Click "**This Firefox**" in left sidebar
4. Scroll to "**Setup**" section
5. Under "Network Location", add: `localhost:6080`
6. Click "**Add**"
7. You should see your Servo connection appear under "**Remote Targets**"
8. Click "**Inspect**" to open DevTools

**Method 2: Using Browser Console**

1. Open Firefox
2. Press `Cmd+Shift+K` (Mac) or `Ctrl+Shift+K` (Windows/Linux)
3. In the console, execute:
   ```javascript
   Services.devtools.enableConnection("localhost", 6080);
   ```

### 3. Available DevTools Features

Once connected, you'll have access to:

- **Console**: View `console.log()` output from your TypeScript/WASM
- **Debugger**:
  - Set breakpoints in compiled JavaScript
  - Step through code execution
  - Inspect variables
- **Network**: Monitor script fetching
- **Storage**: View localStorage, sessionStorage
- **Performance**: Profile script execution

## Debugging TypeScript

### Viewing Source Maps
Currently, the TypeScript compiler generates plain JavaScript without source maps. You'll see the compiled JavaScript in the debugger, not the original TypeScript.

To see TypeScript compilation output:
```bash
./mach run test-typescript.html 2>&1 | grep "TypeScript:"
```

### Common Console Commands
```javascript
// Check if functions are exported
console.log(typeof window.add);  // From WASM
console.log(window);  // View all exports

// Test WASM functions
window.add(5, 3);  // Call WASM function
```

## Debugging WebAssembly

### Viewing WASM in DevTools
1. Open the **Debugger** tab
2. Look for files like `data:application/wasm;base64,...`
3. Firefox will display the WASM text format (WAT) if available

### WASM Console Debugging
```javascript
// Check exported WASM functions
console.log(Object.keys(window).filter(k => typeof window[k] === 'function'));

// Test WASM functions
console.log(window.subtract(50, 8));  // Should return 42
console.log(window.factorial(5));    // Should return 120
```

## Troubleshooting

### DevTools Won't Connect
1. **Check if DevTools server started**:
   ```bash
   ./mach run test.html 2>&1 | grep -i devtools
   ```
   Should show: `DevTools Server listening on 127.0.0.1:6080`

2. **Check if port is in use**:
   ```bash
   lsof -i :6080
   ```

3. **Try restarting Servo**:
   ```bash
   killall servo
   ./mach run test.html
   ```

### Can't See Console Output
- Make sure you're connected to the right target
- Check the **Console** tab is selected
- Try `console.log("test")` in your script

### Breakpoints Not Working
- Set breakpoints in the compiled JavaScript (not TypeScript)
- Use `debugger;` statement in your code to force breaks

## Advanced: Changing DevTools Port

Edit `components/config/prefs.rs`:
```rust
devtools_server_enabled: true,
devtools_server_port: 9000,  // Change to your preferred port
```

Then rebuild:
```bash
./build.sh
```

## Example Debugging Session

```bash
# Terminal 1: Start Servo
./mach run test-typescript.html

# Terminal 2: Watch logs
tail -f servo.log | grep -E "TypeScript|WASM|error"
```

In Firefox:
1. Connect to `localhost:6080`
2. Open Console tab
3. You should see all console.log outputs
4. Type `window` to see all exported functions
5. Test: `window.add(1, 2)` (WASM function)

## References

- [Firefox Remote Debugging Docs](https://firefox-source-docs.mozilla.org/devtools-user/about_colon_debugging/)
- [Servo DevTools](https://github.com/servo/servo/wiki/Devtools)
