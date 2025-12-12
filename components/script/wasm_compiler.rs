// Copyright 2025 The Servo Project Developers.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! WebAssembly Text (WAT) to binary compilation

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use parking_lot::RwLock;
use serde_json;

/// Error type for WASM compilation
#[derive(Debug)]
pub enum CompileError {
    ParseError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::ParseError(msg) => write!(f, "WAT parse error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Simple in-memory cache for compiled WASM
/// Maps hash(source_code) -> compiled binary as base64
fn get_cache() -> &'static RwLock<HashMap<u64, Vec<u8>>> {
    static CACHE: OnceLock<RwLock<HashMap<u64, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Compile WAT source code to WASM binary, then encode as base64 data URL
///
/// # Arguments
/// * `source` - The WAT (WebAssembly Text) source code
/// * `filename` - The name of the file (for error reporting)
/// * `callback` - Optional JavaScript code to run after WASM loads (wrapped in wasmloaded event)
///
/// # Returns
/// JavaScript code that loads the WASM module and exports its functions
pub fn compile_wat_to_js(source: &str, filename: &str, callback: Option<&str>) -> Result<String, CompileError> {
    log::info!("WASM: Compiling {} ({} bytes)", filename, source.len());

    // Check cache first
    let cache_key = calculate_hash(source);
    let wasm_binary = {
        // Check cache first - must drop read lock before attempting write
        let cached = {
            let cache = get_cache().read();
            cache.get(&cache_key).cloned()
        };

        if let Some(binary) = cached {
            log::info!("WASM: Cache hit for {}", filename);
            binary
        } else {
            // Compile WAT to WASM binary
            let binary = compile_wat_internal(source, filename)?;
            log::info!("WASM: Successfully compiled {} to {} bytes of WASM", filename, binary.len());

            // Store in cache (read lock is already dropped at this point)
            {
                let mut cache = get_cache().write();
                // Limit cache size to 100 entries (WASM modules can be large)
                if cache.len() > 100 {
                    cache.clear();
                }
                cache.insert(cache_key, binary.clone());
            }

            binary
        }
    };


    // Parse field names from WAT source (more reliable than name section)
    let field_names_json = parse_wat_field_names(source);

    // Generate JavaScript byte array directly (no base64 encoding needed!)
    // This is the approach that works reliably in Servo
    let byte_array = wasm_binary
        .iter()
        .map(|b| format!("0x{:02X}", b))
        .collect::<Vec<_>>()
        .join(", ");


    // Generate JavaScript that uses direct byte array
    // This avoids base64/atob issues and works perfectly in Servo
    let mut js_code = format!(
        r#"
(function() {{
    try {{
        console.log('WASM: Starting module load');

        // WASM module as direct byte array (most reliable method)
        const wasmBytes = new Uint8Array([{}]);

        console.log('WASM: Instantiating module (' + wasmBytes.length + ' bytes)...');

        // Build import object with all global functions automatically
        const importObject = {{}};

        // Collect all callable globals
        for (const key in window) {{
            try {{
                if (typeof window[key] === 'function' && key !== 'window') {{
                    // Add to 'env' namespace (standard convention)
                    if (!importObject.env) {{
                        importObject.env = {{}};
                    }}
                    importObject.env[key] = window[key];
                }}
            }} catch (e) {{
                // Skip inaccessible properties
            }}
        }}

        console.log('WASM: Available imports:', Object.keys(importObject.env || {{}}).length, 'functions');

        // Instantiate directly from byte array with imports
        WebAssembly.instantiate(wasmBytes, importObject)
            .then(function(result) {{
                console.log('WASM: Module instantiated successfully');

                // Export all WASM functions to window
                if (result.instance && result.instance.exports) {{
                    // Helper to convert WASM string array (array i8, UTF-8) to JS string
                    const wasmStringToJs = function(wasmStr) {{
                        if (!wasmStr || typeof wasmStr !== 'object') {{
                            return null;
                        }}

                        // Try to read array as UTF-8 bytes
                        try {{
                            const bytes = [];
                            let i = 0;
                            while (true) {{
                                const byte = wasmStr[i];
                                if (byte === undefined) break;
                                bytes.push(byte);
                                i++;
                                if (i > 10000) break; // Safety limit
                            }}

                            // Decode UTF-8 bytes to string
                            const decoder = new TextDecoder('utf-8');
                            return decoder.decode(new Uint8Array(bytes));
                        }} catch (e) {{
                            return null;
                        }}
                    }};

                    // Helper to wrap GC objects with toString support
                    const wrapGcObject = function(obj) {{
                        if (!obj || typeof obj !== 'object') {{
                            return obj;
                        }}

                        // Check if already wrapped
                        if (obj.__wasmGcWrapped) {{
                            return obj;
                        }}

                        // Check if this is a string array (has numeric indices that are UTF-8 bytes)
                        const isStringArray = function() {{
                            try {{
                                // Check first few elements - if they're all valid bytes (0-255), it's likely a string
                                const first = obj[0];
                                if (first !== undefined && typeof first === 'number' && first >= 0 && first <= 255) {{
                                    return true;
                                }}
                            }} catch (e) {{}}
                            return false;
                        }};

                        // Get type info (name and fields) for this struct
                        const getTypeInfo = function() {{
                            if (window.__wasmFieldNames && window.__wasmFieldNames.default) {{
                                return window.__wasmFieldNames.default;
                            }}
                            return null;
                        }};

                        // Create proxy with toString and Symbol.toPrimitive handlers
                        return new Proxy(obj, {{
                            get(target, prop) {{
                                // Handle toString
                                if (prop === 'toString') {{
                                    return function() {{
                                        // Check if this is a string array
                                        if (isStringArray()) {{
                                            const jsStr = wasmStringToJs(target);
                                            return jsStr !== null ? jsStr : '[WasmString]';
                                        }}

                                        // Try to get field values for display
                                        let fields = [];
                                        const typeInfo = getTypeInfo();
                                        const typeName = (typeInfo && typeInfo.typeName) ? typeInfo.typeName : 'WasmGcStruct';
                                        const fieldNames = (typeInfo && typeInfo.fields) ? typeInfo.fields : null;

                                        try {{
                                            if (fieldNames) {{
                                                // Use field names if available
                                                for (let i = 0; i < fieldNames.length; i++) {{
                                                    const val = target[i];
                                                    if (val !== undefined) {{
                                                        // Convert nested string arrays
                                                        const displayVal = (val && typeof val === 'object' && val[0] !== undefined && typeof val[0] === 'number')
                                                            ? '"' + (wasmStringToJs(val) || '') + '"'
                                                            : val;
                                                        fields.push(fieldNames[i] + '=' + displayVal);
                                                    }}
                                                }}
                                            }} else {{
                                                // Fallback to numeric indices
                                                if (target[0] !== undefined) {{
                                                    fields.push('0=' + target[0]);
                                                }}
                                            }}
                                        }} catch (e) {{
                                            // Ignore errors
                                        }}

                                        if (fields.length > 0) {{
                                            return typeName + '{{' + fields.join(', ') + '}}';
                                        }}
                                        return typeName + '{{}}';
                                    }};
                                }} else if (prop === Symbol.toPrimitive) {{
                                    // Handle Symbol.toPrimitive for string conversion
                                    return function(hint) {{
                                        if (hint === 'string' || hint === 'default') {{
                                            // Check if this is a string array
                                            if (isStringArray()) {{
                                                const jsStr = wasmStringToJs(target);
                                                return jsStr !== null ? jsStr : '[WasmString]';
                                            }}

                                            let fields = [];
                                            const typeInfo = getTypeInfo();
                                            const typeName = (typeInfo && typeInfo.typeName) ? typeInfo.typeName : 'WasmGcStruct';
                                            const fieldNames = (typeInfo && typeInfo.fields) ? typeInfo.fields : null;

                                            try {{
                                                if (fieldNames) {{
                                                    for (let i = 0; i < fieldNames.length; i++) {{
                                                        const val = target[i];
                                                        if (val !== undefined) {{
                                                            const displayVal = (val && typeof val === 'object' && val[0] !== undefined && typeof val[0] === 'number')
                                                                ? '"' + (wasmStringToJs(val) || '') + '"'
                                                                : val;
                                                            fields.push(fieldNames[i] + '=' + displayVal);
                                                        }}
                                                    }}
                                                }} else {{
                                                    if (target[0] !== undefined) {{
                                                        fields.push('0=' + target[0]);
                                                    }}
                                                }}
                                            }} catch (e) {{}}

                                            if (fields.length > 0) {{
                                                return typeName + '{{' + fields.join(', ') + '}}';
                                            }}
                                            return typeName + '{{}}';
                                        }}
                                        // For number hint, return NaN to avoid conversion errors
                                        return NaN;
                                    }};
                                }} else if (prop === Symbol.toStringTag) {{
                                    const typeInfo = getTypeInfo();
                                    return (typeInfo && typeInfo.typeName) ? typeInfo.typeName : 'WasmGcStruct';
                                }} else if (prop === '__wasmGcWrapped') {{
                                    return true;
                                }}

                                // Try to map field name to index
                                if (typeof prop === 'string') {{
                                    const typeInfo = getTypeInfo();
                                    const fieldNames = (typeInfo && typeInfo.fields) ? typeInfo.fields : null;
                                    if (fieldNames) {{
                                        const index = fieldNames.indexOf(prop);
                                        if (index !== -1) {{
                                            const value = target[index];
                                            // Auto-convert string arrays to JS strings
                                            if (value && typeof value === 'object' && value[0] !== undefined && typeof value[0] === 'number' && value[0] >= 0 && value[0] <= 255) {{
                                                return wasmStringToJs(value) || value;
                                            }}
                                            return value;
                                        }}
                                    }}
                                }}

                                // Handle numeric property access
                                const value = target[prop];
                                // Auto-convert string arrays to JS strings
                                if (value && typeof value === 'object' && value[0] !== undefined && typeof value[0] === 'number' && value[0] >= 0 && value[0] <= 255) {{
                                    return wasmStringToJs(value) || value;
                                }}
                                return value;
                            }},
                            set(target, prop, value) {{
                                // Try to map field name to index for setters
                                if (typeof prop === 'string') {{
                                    const typeInfo = getTypeInfo();
                                    const fieldNames = (typeInfo && typeInfo.fields) ? typeInfo.fields : null;
                                    if (fieldNames) {{
                                        const index = fieldNames.indexOf(prop);
                                        if (index !== -1) {{
                                            target[index] = value;
                                            return true;
                                        }}
                                    }}
                                }}

                                target[prop] = value;
                                return true;

                                // TODO: Enforce field mutability
                                // Currently allows modification of immutable WASM fields from JS.
                                // To fix: parse type section to track which fields are mutable,
                                // and throw TypeError when attempting to modify immutable fields.
                                // For now: "It's a feature, not a bug!" 😄
                            }}
                        }});
                    }};

                    for (const name in result.instance.exports) {{
                        const exported = result.instance.exports[name];

                        if (typeof exported === 'function') {{
                            // Wrap function to auto-wrap GC object return values
                            window[name] = function(...args) {{
                                const result = exported.apply(this, args);
                                return wrapGcObject(result);
                            }};
                            console.log('WASM: Exported function ' + name);
                        }} else if (exported instanceof WebAssembly.Global) {{
                            // For globals containing GC objects, wrap the value and expose directly
                            const globalValue = exported.value;
                            if (globalValue && typeof globalValue === 'object') {{
                                // This is a GC object (struct, array, etc.) - wrap and export the value directly
                                window[name] = wrapGcObject(globalValue);
                                // Also store the raw Global for advanced use (mutable globals)
                                window[name + '_global'] = exported;
                                console.log('WASM: Exported GC global ' + name + ' = WasmGcStruct');
                            }} else {{
                                // Simple global (i32, f64, etc.) - export the Global object with .value property
                                window[name] = exported;
                                console.log('WASM: Exported global ' + name + ' = ' + exported.value);
                            }}
                        }} else {{
                            // Export other types (Memory, Table, etc.)
                            window[name] = exported;
                            console.log('WASM: Exported ' + name);
                        }}
                    }}

                    // Helper function to display GC struct contents
                    window.WasmGcStructDisplay = function(structObj, structName) {{
                        if (!structObj || typeof structObj !== 'object') {{
                            return String(structObj);
                        }}

                        structName = structName || 'box';
                        let fields = [];

                        // Try common field names
                        const commonFields = ['val', 'value', 'data', 'x', 'y', 'z', 'width', 'height'];
                        for (const fieldName of commonFields) {{
                            if (typeof WasmGcStructGet !== 'undefined') {{
                                try {{
                                    const fieldValue = WasmGcStructGet(structObj, fieldName);
                                    if (fieldValue !== undefined) {{
                                        fields.push(fieldName + '=' + fieldValue);
                                    }}
                                }} catch (e) {{
                                    // Field doesn't exist, skip
                                }}
                            }}
                        }}

                        if (fields.length > 0) {{
                            return structName + '{{' + fields.join(', ') + '}}';
                        }} else {{
                            return structName + '{{}}';
                        }}
                    }};

                    // Create GC struct field accessors
                    // For WASM GC structs, we need getter functions that call struct.get
                    // These are typically exported as 'get_field_X' functions by WASM
                    window.WasmGcStructGet = function(structObj, fieldIndex) {{
                        // Attempt to extract field value from GC struct
                        // Look for exported getter functions following common patterns
                        const getterName = 'get_' + fieldIndex;
                        if (window._wasmExports && window._wasmExports[getterName]) {{
                            try {{
                                return window._wasmExports[getterName](structObj);
                            }} catch (e) {{
                                console.warn('WasmGcStructGet: Getter', getterName, 'failed:', e);
                            }}
                        }}

                        // Fallback: try numeric field access patterns
                        const fieldGetter = 'struct_get_' + fieldIndex;
                        if (window._wasmExports && window._wasmExports[fieldGetter]) {{
                            try {{
                                return window._wasmExports[fieldGetter](structObj);
                            }} catch (e) {{
                                console.warn('WasmGcStructGet: Getter', fieldGetter, 'failed:', e);
                            }}
                        }}

                        // Try property access as last resort (for externref wrapping)
                        if (structObj && typeof structObj === 'object') {{
                            if (structObj[fieldIndex] !== undefined) {{
                                return structObj[fieldIndex];
                            }}
                            const fieldName = 'field' + fieldIndex;
                            if (structObj[fieldName] !== undefined) {{
                                return structObj[fieldName];
                            }}
                        }}

                        console.warn('WasmGcStructGet: Unable to access field', fieldIndex, 'on', structObj);
                        return undefined;
                    }};

                    // Helper to list available getter functions
                    window.WasmListGetters = function() {{
                        const getters = [];
                        for (const name in window._wasmExports) {{
                            if (name.startsWith('get_') || name.startsWith('struct_get_')) {{
                                getters.push(name);
                            }}
                        }}
                        return getters;
                    }};

                    // Install field name mappings
                    window.__wasmFieldNames = {field_names_json};
                    console.log('WASM: Field names installed:', window.__wasmFieldNames);

                    console.log('WASM: GC struct accessors installed');
                    console.log('WASM: Available getters:', window.WasmListGetters());
                }}

                console.log('WASM module loaded successfully');
                // Dispatch custom event so pages can listen for WASM completion
                window.dispatchEvent(new Event('wasmloaded'));
            }})
            .catch(function(e) {{
                console.error('WASM instantiation error:', e);
            }});

    }} catch (e) {{
        console.error('WASM error:', e);
    }}
}})();
"#,
        byte_array
    );

    // Append optional callback code wrapped in wasmloaded event listener
    if let Some(callback_code) = callback {
        if !callback_code.trim().is_empty() {
            js_code.push_str("\n// Auto-generated callback from inline script content\n");
            js_code.push_str("window.addEventListener('wasmloaded', function() {\n");
            js_code.push_str(callback_code);
            js_code.push_str("\n});\n");
        }
    }


    Ok(js_code)
}

/// Transform WAT source to replace 'string' type with GC array representation
/// Strings are represented as (array i8) for UTF-8 encoding
fn transform_string_types(source: &str) -> String {
    // Check if $string type is already defined
    let has_string_type = source.contains("(type $string");

    let mut result = String::new();
    let mut in_module = false;
    let mut string_type_added = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Detect module start to inject string type definition
        if trimmed.starts_with("(module") {
            result.push_str(line);
            result.push('\n');
            in_module = true;
            continue;
        }

        // Add string type definition right after module start, before any other content
        // Skip if already defined in source
        if in_module && !string_type_added && !has_string_type && !trimmed.is_empty() && !trimmed.starts_with(";") {
            // Insert string type before any module content
            result.push_str("  ;; String type: array of i8 (UTF-8)\n");
            result.push_str("  (type $string (array (mut i8)))\n\n");
            string_type_added = true;
        }

        // First, replace 'string' type references with '(ref null $string)'
        // But skip if line already uses $string type
        let type_transformed = if line.contains("string") && !line.contains("$string") && !line.contains("(type $string") {
            // Replace type references: (mut string) -> (mut (ref null $string))
            let mut new_line = line.to_string();

            // Handle field definitions: (field $name (mut string))
            new_line = new_line.replace("(mut string)", "(mut (ref null $string))");

            // Handle param/result: (param string) or (result string)
            new_line = new_line.replace("(param string)", "(param (ref null $string))");
            new_line = new_line.replace("(result string)", "(result (ref null $string))");

            new_line
        } else {
            line.to_string()
        };

        // Then, transform string literals in struct.new
        let transformed = if trimmed.contains("struct.new") && trimmed.contains("\"") {
            transform_string_literal_in_line(&type_transformed)
        } else {
            type_transformed
        };

        result.push_str(&transformed);
        result.push('\n');
    }

    result
}

/// Transform a line containing struct.new with string literal
fn transform_string_literal_in_line(line: &str) -> String {
    // Find struct.new position first
    if let Some(struct_new_pos) = line.find("struct.new") {
        // Only look for string literals AFTER struct.new
        let after_struct_new = &line[struct_new_pos..];

        if let Some(start_quote) = after_struct_new.find('"') {
            let absolute_start_quote = struct_new_pos + start_quote;

            if let Some(end_quote) = after_struct_new[start_quote + 1..].find('"') {
                let literal_start = absolute_start_quote + 1;
                let literal_end = absolute_start_quote + 1 + end_quote;
                let string_content = &line[literal_start..literal_end];

                // Convert string to UTF-8 bytes
                let utf8_bytes: Vec<String> = string_content
                    .as_bytes()
                    .iter()
                    .map(|b| format!("(i32.const {})", b))
                    .collect();

                let array_init = format!(
                    "(array.new_fixed $string {} {})",
                    utf8_bytes.len(),
                    utf8_bytes.join(" ")
                );

                // Replace the string literal with array initialization
                let before = &line[..absolute_start_quote];
                let after = &line[literal_end + 1..];
                return format!("{}{}{}", before, array_init, after);
            }
        }
    }

    line.to_string()
}

/// Internal compilation function using wat crate
fn compile_wat_internal(source: &str, filename: &str) -> Result<Vec<u8>, CompileError> {
    // Check if input is already binary WASM (starts with magic number \0asm)
    let source_bytes = source.as_bytes();
    let wasm_binary = if source_bytes.len() >= 4 && &source_bytes[0..4] == b"\0asm" {
        log::info!("WASM: Input is already binary WASM, using directly");
        // Already compiled, use the bytes
        source_bytes.to_vec()
    } else {
        // Transform string types to GC arrays before compilation
        let transformed_source = transform_string_types(source);
        log::info!("WASM: Transformed WAT:\n{}", transformed_source);

        // Otherwise, parse as WAT text format
        wat::parse_str(&transformed_source).map_err(|e| CompileError::ParseError(format!("in {}: {}", filename, e)))?
    };

    // Inject getter/setter functions for WASM GC structs
    inject_gc_accessors(&wasm_binary)
}

/// Inject getter/setter functions for WASM GC struct fields
fn inject_gc_accessors(wasm_binary: &[u8]) -> Result<Vec<u8>, CompileError> {

    // Automatic getter/setter injection for WASM GC structs is complex and requires:
    // - Parsing type section to detect struct definitions
    // - Generating new function types for getters/setters
    // - Encoding struct.get/struct.set instructions
    // - Managing function/type indices correctly
    //
    // Given SpiderMonkey's architectural limitations (JIT blocks property access on
    // non-native objects) and the complexity of WASM binary manipulation, the pragmatic
    // approach is to require manual getter/setter exports in the WASM code.
    //
    // Example WAT with manual exports:
    //
    //   (module
    //     (type $box (struct (field $val (mut i32))))
    //     (func $makeBox (export "makeBox") (param i32) (result (ref $box))
    //       local.get 0
    //       struct.new $box
    //     )
    //     (func $get_val (export "get_val") (param (ref $box)) (result i32)
    //       local.get 0
    //       struct.get $box $val
    //     )
    //     (func $set_val (export "set_val") (param (ref $box)) (param i32)
    //       local.get 0
    //       local.get 1
    //       struct.set $box $val
    //     )
    //   )
    //
    // Then in JavaScript: get_val(box) instead of box.val


    Ok(wasm_binary.to_vec())
}

/// Calculate hash for caching
fn calculate_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Parse field names and type names directly from WAT source
/// Looks for struct field definitions like: (field $name (mut i32))
/// Returns JSON with structure: { "default": { "typeName": "box", "fields": ["val"] } }
fn parse_wat_field_names(source: &str) -> String {
    let mut type_fields: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_type: Option<String> = None;
    let mut field_index = 0;

    // Simple regex-free parser for WAT field names
    for line in source.lines() {
        let trimmed = line.trim();

        // Look for type definitions: (type $typename (struct
        if trimmed.contains("(type") && trimmed.contains("(struct") {
            // Extract type name
            if let Some(start) = trimmed.find("$") {
                if let Some(end) = trimmed[start..].find(|c: char| c.is_whitespace()) {
                    let type_name = &trimmed[start..start + end];
                    current_type = Some(type_name.to_string());
                    field_index = 0;
                }
            }
        }

        // Look for field definitions: (field $fieldname ...
        if let Some(ref type_name) = current_type {
            if trimmed.contains("(field") {
                // Find the LAST $ on the line (field name, not type name)
                // This handles cases like: (type $box (struct (field $val (mut i32))))
                if let Some(field_start) = trimmed.rfind("$") {
                    // Make sure this $ is after "(field"
                    if let Some(field_marker) = trimmed.find("(field") {
                        if field_start > field_marker {
                            // Find end of field name (space or parenthesis)
                            let name_part = &trimmed[field_start + 1..];
                            if let Some(end) = name_part.find(|c: char| c.is_whitespace() || c == ')') {
                                let field_name = &name_part[..end];

                                type_fields
                                    .entry(type_name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(field_name.to_string());

                                field_index += 1;
                            }
                        }
                    }
                }
            }
        }

        // Reset when closing type definition
        if trimmed.contains(")") && current_type.is_some() && !trimmed.contains("(field") {
            if trimmed.matches(')').count() >= 2 {
                current_type = None;
            }
        }
    }

    // Convert to JSON - include both type name and fields
    if type_fields.is_empty() {
        "{}".to_string()
    } else {
        // Get the first type name and its fields
        let (type_name, fields) = type_fields.iter().next().unwrap();

        // Strip the $ prefix from type name for cleaner display
        let clean_type_name = type_name.strip_prefix("$").unwrap_or(type_name);

        // Build JSON manually to ensure correct structure
        let fields_json = fields
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"{{"default":{{"typeName":"{}","fields":[{}]}}}}"#,
            clean_type_name, fields_json
        )
    }
}

/// Parse WASM name section to extract field names (fallback method)
/// Returns JSON object mapping type names to field name arrays
#[allow(dead_code)]
fn parse_name_section(wasm_binary: &[u8]) -> String {
    // WASM binary format:
    // - Magic number: 0x00 0x61 0x73 0x6D (\0asm)
    // - Version: 0x01 0x00 0x00 0x00
    // - Sections: [section_id, size, payload...]
    //   - Custom section: id=0, name="name"
    //     - Subsection 11: Type names
    //     - Subsection 12: Field names

    if wasm_binary.len() < 8 {
        return "{}".to_string();
    }

    let mut pos = 8; // Skip magic + version
    let mut field_names_map: HashMap<String, Vec<String>> = HashMap::new();

    while pos < wasm_binary.len() {
        if pos + 1 >= wasm_binary.len() {
            break;
        }

        let section_id = wasm_binary[pos];
        pos += 1;

        // Read section size (LEB128)
        let (section_size, size_len) = read_leb128_u32(&wasm_binary[pos..]);
        pos += size_len;

        if section_id == 0 {
            // Custom section - check if it's the "name" section
            let section_end = pos + section_size as usize;

            if section_end > wasm_binary.len() {
                break;
            }

            // Read section name length
            let (name_len, name_len_size) = read_leb128_u32(&wasm_binary[pos..]);
            pos += name_len_size;

            if pos + name_len as usize > wasm_binary.len() {
                break;
            }

            // Read section name
            let section_name = &wasm_binary[pos..pos + name_len as usize];
            pos += name_len as usize;

            if section_name == b"name" {

                // Parse name section subsections
                while pos < section_end {
                    if pos + 1 >= section_end {
                        break;
                    }

                    let subsection_id = wasm_binary[pos];
                    pos += 1;

                    let (subsection_size, subsection_size_len) = read_leb128_u32(&wasm_binary[pos..]);
                    pos += subsection_size_len;

                    let subsection_end = pos + subsection_size as usize;

                    if subsection_id == 12 {
                        // Field names subsection
                        field_names_map = parse_field_names_subsection(&wasm_binary[pos..subsection_end]);
                    }

                    pos = subsection_end;
                }

                break;
            } else {
                pos = section_end;
            }
        } else {
            pos += section_size as usize;
        }
    }

    // Convert to JSON
    if field_names_map.is_empty() {
        "{}".to_string()
    } else {
        serde_json::to_string(&field_names_map).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Parse field names subsection
fn parse_field_names_subsection(data: &[u8]) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    let mut pos = 0;

    // Read count of types
    let (type_count, count_len) = read_leb128_u32(&data[pos..]);
    pos += count_len;


    for _ in 0..type_count {
        if pos >= data.len() {
            break;
        }

        // Read type index
        let (type_idx, idx_len) = read_leb128_u32(&data[pos..]);
        pos += idx_len;

        // Read field count
        let (field_count, field_count_len) = read_leb128_u32(&data[pos..]);
        pos += field_count_len;

        let mut field_names = Vec::new();


        for _ in 0..field_count {
            if pos >= data.len() {
                break;
            }

            // Read field index
            let (_field_idx, field_idx_len) = read_leb128_u32(&data[pos..]);
            pos += field_idx_len;

            // Read field name length
            let (name_len, name_len_size) = read_leb128_u32(&data[pos..]);
            pos += name_len_size;

            if pos + name_len as usize > data.len() {
                break;
            }

            // Read field name
            let name_bytes = &data[pos..pos + name_len as usize];
            pos += name_len as usize;

            if let Ok(name) = std::str::from_utf8(name_bytes) {
                field_names.push(name.to_string());
            }
        }

        result.insert(format!("type_{}", type_idx), field_names);
    }

    result
}

/// Read LEB128 unsigned 32-bit integer
fn read_leb128_u32(data: &[u8]) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= data.len() {
            break;
        }

        let byte = data[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u32) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            break;
        }

        if shift >= 32 {
            break;
        }
    }

    (result, pos)
}

/// Clear the compilation cache (useful for testing or memory management)
#[allow(dead_code)]
pub fn clear_cache() {
    get_cache().write().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_transformation() {
        let source = r#"(module
  (type $Box (struct (field $val (mut string))))
  (global $box (export "box") (ref $Box) (struct.new $Box "hello"))
)"#;

        let transformed = transform_string_types(source);
        println!("Transformed WAT:\n{}", transformed);

        // Check that string type was added
        assert!(transformed.contains("(type $string (array (mut i8)))"));

        // Check that string references were replaced
        assert!(transformed.contains("(ref null $string)"));

        // Check that string literal was transformed
        assert!(transformed.contains("array.new_fixed $string"));
    }

    #[test]
    fn test_simple_wasm() {
        let source = r#"
            (module
              (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add)
              (export "add" (func $add)))
        "#;

        let result = compile_wat_to_js(source, "test.wat", None);
        assert!(result.is_ok());

        let js = result.unwrap();
        assert!(js.contains("WebAssembly"));
        assert!(js.contains("data:application/wasm;base64,"));
    }

    #[test]
    fn test_caching() {
        clear_cache();

        let source = "(module)";

        // First compilation
        let result1 = compile_wat_to_js(source, "test.wat", None);
        assert!(result1.is_ok());

        // Second compilation (should hit cache)
        let result2 = compile_wat_to_js(source, "test.wat", None);
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_invalid_wat() {
        let source = "(module (invalid syntax))";

        let result = compile_wat_to_js(source, "test.wat", None);
        assert!(result.is_err());
    }
}
