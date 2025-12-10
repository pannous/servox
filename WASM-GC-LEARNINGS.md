# WASM GC Struct Property Access Investigation

## Goal
Enable transparent JavaScript property access to WebAssembly GC struct fields:
- `box.val` or `box[0]` instead of `get_val(box)`
- `box.val = 99` instead of `set_val(box, 99)`

## Investigation Summary

### Approach 1: SpiderMonkey C++ Hooks
**Attempt**: Add custom property access handlers via `JSClass` ObjectOps (obj_getProperty, obj_lookupProperty)

**Files Modified**:
- `/opt/other/mozjs/mozjs-sys/mozjs/js/src/wasm/WasmGcObject.cpp`
- Added extensive logging to hooks
- Implemented jsid-to-field-index conversion

**Result**: ❌ Hooks were NEVER called

**Discovery**: Property access doesn't reach C++ hooks because JIT compiles fast paths that bypass them entirely

---

### Approach 2: IC (Inline Cache) Handlers
**Attempt**: Add WASM GC property access to CacheIR system

**Files Modified**:
- `/opt/other/mozjs/mozjs-sys/mozjs/js/src/jit/CacheIR.cpp`
- Added tryAttachWasmGc handler
- `/opt/other/mozjs/mozjs-sys/mozjs/js/src/jit/BaselineIC.cpp`
- Added logging to DoGetPropFallback

**Result**: ❌ Handler never invoked

**Discovery**: IC system filters non-native objects before reaching custom attach logic

---

### Approach 3: JIT Megamorphic Path Modification
**Attempt**: Modify JIT code generation for property access on non-native objects

**Files Modified**:
- `/opt/other/mozjs/mozjs-sys/mozjs/js/src/jit/CodeGenerator.cpp`
  - `visitMegamorphicLoadSlot` (line 4581)
  - `visitMegamorphicLoadSlotByValue`

**Critical Code**:
```cpp
masm.branchIfNonNativeObj(obj, temp0, &bail);  // THIS LINE BLOCKS WASM GC!
```

**Result**: ❌ Even with modifications, property access returned undefined

**Discovery**: Guards execute in generated machine code BEFORE any C++ calls

---

### Approach 4: High-Level Property Intercept
**Attempt**: Hack GetPropertyNoGC to return dummy value (42) for objects with custom ops

**Files Modified**:
- `/opt/other/mozjs/mozjs-sys/mozjs/js/src/vm/ObjectOperations-inl.h`

**Result**: ❌ Still returned undefined

**Discovery**: JIT completely bypasses this high-level function

---

### Key Architectural Finding

**SpiderMonkey JIT intentionally blocks property access on non-native objects**:

1. Property access compiles to machine code with type guards
2. Guards check `obj->is<NativeObject>()`
3. Non-native objects (including WASM GC) bail out → return undefined
4. This happens BEFORE any C++ hooks, IC handlers, or high-level functions
5. Design is intentional for security and performance

**Why transparent access is impractical**:
- Would require modifying JIT code generation templates
- Complex changes across multiple JIT tiers (Baseline, Ion)
- Would need special-case handling for WASM GC types
- Potential performance/security implications

---

### Approach 5: Automatic Getter/Setter Injection
**Attempt**: Parse WASM binary and inject getter/setter functions automatically

**Files Modified**:
- `/opt/other/servo-light/components/script/wasm_compiler.rs`
- Added `inject_gc_accessors()` function
- Added dependencies: `wasmparser`, `wasm-encoder`, `walrus`

**Challenges**:
1. Parsing type section to find struct definitions ✅ (doable)
2. Creating new function types for getters/setters
3. Generating function code with `struct.get`/`struct.set` instructions
4. Managing function/type indices correctly as new items are added
5. Handling different storage types (i32, i64, f32, f64, ref types)
6. Binary format complexity - sections have dependencies and ordering requirements

**Result**: ⚠️ Too complex for practical implementation

**Complexity**: Requires deep understanding of:
- WASM binary format section layout
- Type/function index management
- GC-specific instruction encoding
- Cross-section reference updates

---

## ✅ Working Solution: Manual Getter/Setter Exports

**Pragmatic approach**: Export getter/setter functions explicitly in WASM code

### Example WAT Code:
```wat
(module
  (type $box (struct (field $val (mut i32))))

  (func $makeBox (export "makeBox") (param i32) (result (ref $box))
    local.get 0
    struct.new $box
  )

  (func $get_val (export "get_val") (param (ref $box)) (result i32)
    local.get 0
    struct.get $box $val
  )

  (func $set_val (export "set_val") (param (ref $box)) (param i32)
    local.get 0
    local.get 1
    struct.set $box $val
  )
)
```

### JavaScript Usage:
```javascript
const box = window.makeBox(42);
const value = window.get_val(box);  // ✅ Returns 42
window.set_val(box, 99);             // ✅ Sets field to 99
```

### Test Results:
- ✅ getter: `get_val(box)` returns correct value (42)
- ✅ setter: `set_val(box, 99)` modifies field correctly
- ✅ mutation: Field changes persist across calls
- ✅ All tests passing in `test-wasm-gc-with-getters.html`

---

## Alternative Engines Considered

### Boa (Rust-based JavaScript Engine)
- **Pros**: Pure Rust, easier to modify
- **Cons**: Servo+Boa integration would be "huge amount of work" per maintainers
- **Conclusion**: Not practical for this feature

---

## Lessons Learned

1. **JIT Architecture is Complex**: Property access optimization happens at multiple layers, with guards at the lowest levels

2. **Non-Native Objects Are Special**: SpiderMonkey treats them differently by design, not by accident

3. **Binary Format Manipulation is Hard**: WASM binary transformation requires careful handling of interdependent sections

4. **Pragmatic Solutions Win**: Sometimes the workaround (manual exports) is better than fighting the architecture

5. **Test Early**: We should have tested the manual getter approach before deep diving into SpiderMonkey internals

---

## Future Possibilities

### Option 1: Upstream SpiderMonkey Changes
- Propose WASM GC property access as SpiderMonkey feature
- Would require buy-in from Mozilla SpiderMonkey team
- Significant JIT engineering effort

### Option 2: JavaScript Proxy Wrapper
- Wrap WASM GC objects in JavaScript Proxies
- Intercept property access and call getters/setters
- Performance overhead, but transparent to users

### Option 3: WASM Tooling Pass
- Use `wasm-tools` or `binaryen` to inject getters automatically
- Run as preprocessing step before loading into Servo
- Cleaner separation of concerns

---

## Files Modified (Later Reverted)

All SpiderMonkey modifications were reverted via `git checkout -- .` in `/opt/other/mozjs/`

**Why reverted**: Changes didn't work and automatic injection approach was abandoned in favor of manual exports

---

## Final Recommendation

**For WASM GC struct access in Servo**: Manually export getter/setter functions

**Why**:
- Works reliably with current architecture
- No complex JIT modifications needed
- No binary manipulation required
- Clean, explicit API
- Good performance (direct function calls)

**Trade-off**: Slightly more verbose WASM code, but explicit and maintainable
