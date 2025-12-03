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
///
/// # Returns
/// JavaScript code that loads the WASM module and exports its functions
pub fn compile_wat_to_js(source: &str, filename: &str) -> Result<String, CompileError> {
    log::info!("WASM: Compiling {} ({} bytes)", filename, source.len());

    // Check cache first
    let cache_key = calculate_hash(source);
    let wasm_binary = {
        let cache = get_cache().read();
        if let Some(cached) = cache.get(&cache_key) {
            log::info!("WASM: Cache hit for {}", filename);
            cached.clone()
        } else {
            // Compile WAT to WASM binary
            let binary = compile_wat_internal(source, filename)?;
            log::info!("WASM: Successfully compiled {} to {} bytes of WASM", filename, binary.len());

            // Store in cache
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

    // Convert binary to base64 data URL
    let base64_wasm = base64::encode(&wasm_binary);
    let data_url = format!("data:application/wasm;base64,{}", base64_wasm);

    // Generate JavaScript that uses Blob URL to load WASM
    // This avoids issues with Uint8Array and WebAssembly.instantiate()
    let js_code = format!(
        r#"
(function() {{
    try {{
        console.log('WASM: Starting module load');

        // Create binary data from base64
        const base64 = '{}';
        const binaryString = atob(base64);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {{
            bytes[i] = binaryString.charCodeAt(i);
        }}

        console.log('WASM: Creating Blob from ' + bytes.length + ' bytes');

        // Create a Blob and Blob URL
        const blob = new Blob([bytes], {{ type: 'application/wasm' }});
        const blobUrl = URL.createObjectURL(blob);

        console.log('WASM: Fetching from Blob URL...');

        // Fetch from Blob URL (this is how most WASM is loaded)
        fetch(blobUrl)
            .then(function(response) {{ return response.arrayBuffer(); }})
            .then(function(buffer) {{
                console.log('WASM: Instantiating module...');
                return WebAssembly.instantiate(buffer);
            }})
            .then(function(result) {{
                console.log('WASM: Module instantiated successfully');
                URL.revokeObjectURL(blobUrl);

                // Export all WASM functions to window
                if (result.instance && result.instance.exports) {{
                    for (const name in result.instance.exports) {{
                        const func = result.instance.exports[name];
                        if (typeof func === 'function') {{
                            window[name] = func;
                            console.log('WASM: Exported function ' + name);
                        }}
                    }}
                }}

                console.log('WASM module loaded successfully');
            }})
            .catch(function(e) {{
                console.error('WASM loading error:', e);
                URL.revokeObjectURL(blobUrl);
            }});

    }} catch (e) {{
        console.error('WASM error:', e);
    }}
}})();
"#,
        base64_wasm
    );

    Ok(js_code)
}

/// Internal compilation function using wat crate
fn compile_wat_internal(source: &str, filename: &str) -> Result<Vec<u8>, CompileError> {
    wat::parse_str(source)
        .map_err(|e| CompileError::ParseError(format!("in {}: {}", filename, e)))
}

/// Calculate hash for caching
fn calculate_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
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
    fn test_simple_wasm() {
        let source = r#"
            (module
              (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add)
              (export "add" (func $add)))
        "#;

        let result = compile_wat_to_js(source, "test.wat");
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
        let result1 = compile_wat_to_js(source, "test.wat");
        assert!(result1.is_ok());

        // Second compilation (should hit cache)
        let result2 = compile_wat_to_js(source, "test.wat");
        assert!(result2.is_ok());

        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_invalid_wat() {
        let source = "(module (invalid syntax))";

        let result = compile_wat_to_js(source, "test.wat");
        assert!(result.is_err());
    }
}
