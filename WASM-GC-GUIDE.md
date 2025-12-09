# WASM GC Struct Field Access Guide

## Summary

This guide documents how WASM GC struct fields are made accessible in JavaScript within Servo.

## The Problem

WASM GC structs are opaque objects in JavaScript. Unlike regular JavaScript objects, you cannot directly access their fields using property access (e.g., `struct.fieldName`). This is because WASM GC types are managed by the WebAssembly runtime and require explicit getter/setter functions.

## The Solution

### 1. Patched WASM Compiler (components/script/wasm_compiler.rs)

The WASM compiler now generates JavaScript helpers when loading WASM modules:

- **`window._wasmExports`**: Stores all exported WASM functions for introspection
- **`WasmGcStructGet(structObj, fieldIndex)`**: Helper to read struct fields
  - Tries `get_<fieldName>` pattern (e.g., `getValue`)
  - Fallback to `struct_get_<fieldName>`
  - Last resort: direct property access (for externref wrapping)
- **`WasmListGetters()`**: Lists all available getter functions

### 2. Example: WASM GC Struct

```wat
(module
  ;; Define a struct with one mutable i32 field
  (type $box (struct (field $val (mut i32))))

  ;; Create instance
  (func $makeBox (export "makeBox") (param $v i32) (result (ref $box))
    local.get $v
    struct.new $box
  )

  ;; Read field (this is what makes it accessible!)
  (func $getValue (export "getValue") (param $b (ref $box)) (result i32)
    local.get $b
    struct.get $box 0
  )

  ;; Write field
  (func $setValue (export "setValue") (param $b (ref $box)) (param $v i32)
    local.get $b
    local.get $v
    struct.set $box 0
  )
)
```

### 3. JavaScript Usage

```javascript
// Create a struct
const box = makeBox(42);

// Read field via exported getter
const value = getValue(box);  // Returns 42

// Write field via exported setter
setValue(box, 100);

// Use the helper (looks for 'getValue' automatically)
const value2 = WasmGcStructGet(box, 'val');
```

## Files

### Core Implementation
- `components/script/wasm_compiler.rs` - WASM compiler with GC struct accessors

### Examples
- `test-wasm-gc-simple.wat` - WASM GC struct definition
- `test-wasm-gc-simple.wasm` - Pre-compiled binary (183 bytes)
- `test-wasm-gc-load-binary.html` - Demo loading from binary
- `test-wasm-gc-struct.html` - Inline WASM GC example

### Build Tools
- `components/script/tests/wasm_gc_compile.rs` - Rust test to compile WAT→WASM

## Key Insights

1. **WASM GC structs are opaque in JS** - You cannot use `struct.field` syntax
2. **Getter/setter functions are required** - Must export functions using `struct.get`/`struct.set`
3. **Naming conventions matter** - Use `get<FieldName>` or `getValue` patterns
4. **The wat crate supports GC** - Rust's `wat::parse_str()` handles GC proposal syntax
5. **Binary loading works** - `fetch()` + `WebAssembly.instantiate()` works for GC modules

## Compilation

### Using Rust (Recommended)
```bash
cargo test --package script --test wasm_gc_compile
```

### Using wat2wasm (Limited GC support)
```bash
wat2wasm input.wat -o output.wasm --enable-gc
```

## Testing

Run Servo with any of the test files:
```bash
./mach run test-wasm-gc-load-binary.html
./mach run test-wasm-gc-simple.html
```

## Commits

- `8162aa794f4` - Add WASM GC struct field accessor support
- `f09fbcf30fe` - Add WASM binary loading examples and GC struct test
