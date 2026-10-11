//! WebAssembly code generation backend for Perry
//!
//! Compiles HIR modules to WebAssembly binary format for `--target wasm`.
//! Produces a self-contained HTML file with embedded WASM (base64) and JS runtime bridge.
//!
//! All JSValues use NaN-boxing (f64) consistent with perry-runtime.
//! Runtime operations (strings, console, objects) are imported from JavaScript.

pub mod emit;

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use perry_hir::ir::Module;

/// Embedded WASM runtime JavaScript (bridge between WASM and browser APIs)
const WASM_RUNTIME_JS: &str = include_str!("wasm_runtime.js");

/// Compile multiple HIR modules into a self-contained HTML file with embedded WASM.
pub fn compile_modules_to_wasm_html(
    modules: &[(String, Module)],
    title: &str,
    minify: bool,
) -> Result<String> {
    let output = emit::compile_to_wasm_with_async(modules);
    validate_wasm(&output.wasm_bytes)?;
    check_unsupported(&output.unsupported)?;
    check_no_async_js(&output.async_js)?;
    let wasm_b64 = BASE64.encode(&output.wasm_bytes);

    let runtime_js = if minify {
        perry_codegen_js::minify::minify_js(WASM_RUNTIME_JS)
    } else {
        WASM_RUNTIME_JS.to_string()
    };


    // If there are FFI imports, generate a comment listing them for the host to provide
    let ffi_comment = if output.ffi_imports.is_empty() {
        String::new()
    } else {
        format!(
            "\n// === FFI imports required (provide via __ffiImports or bootPerryWasm 2nd arg) ===\n// {}\n",
            output.ffi_imports.join(", ")
        )
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    html, body {{ width: 100vw; height: 100vh; overflow: hidden; }}
    #perry-root {{ width: 100%; flex: 1 1 0%; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }}
  </style>
</head>
<body>
  <div id="perry-root"></div>
  <script>
{runtime_js}{ffi_comment}
  </script>
  <script>
window.__perryWasmB64 = "{wasm_b64}";
bootPerryWasm("{wasm_b64}").catch(e => {{
  document.getElementById("perry-root").textContent = "WASM Error: " + e.message;
  console.error("Boot error:", e);
}});
  </script>
</body>
</html>"#,
        title = html_escape(title),
        runtime_js = runtime_js,
        ffi_comment = ffi_comment,
        wasm_b64 = wasm_b64,
    );

    Ok(html)
}

/// Get the list of FFI function names that a WASM module requires as imports.
pub fn get_ffi_imports(modules: &[(String, Module)]) -> Vec<String> {
    let output = emit::compile_to_wasm_with_async(modules);
    output.ffi_imports
}

/// Get the raw WASM binary (for non-HTML output)
pub fn compile_modules_to_wasm(modules: &[(String, Module)]) -> Result<Vec<u8>> {
    let output = emit::compile_to_wasm_with_async(modules);
    validate_wasm(&output.wasm_bytes)?;
    check_unsupported(&output.unsupported)?;
    check_no_async_js(&output.async_js)?;
    Ok(output.wasm_bytes)
}

/// Expressions the emitter lowered to `undefined` change behaviour silently: report them by kind,
/// and refuse the build under PERRY_WASM_STRICT=1.
fn check_unsupported(unsupported: &[(String, usize)]) -> Result<()> {
    if unsupported.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = unsupported.iter().map(|(kind, n)| format!("{kind} x{n}")).collect();
    let message = format!("PERRY_WASM_UNSUPPORTED: {}", list.join(", "));
    if std::env::var_os("PERRY_WASM_STRICT").is_some_and(|v| v == "1") {
        anyhow::bail!(message);
    }
    eprintln!("Warning: {message}");
    Ok(())
}

/// Async bodies run as Wasm step closures; one left for the JS translator would ship game code
/// as JS, so it fails the build (PRD-582).
fn check_no_async_js(async_js: &str) -> Result<()> {
    if async_js.is_empty() {
        return Ok(());
    }
    let first = async_js.lines().next().unwrap_or_default();
    anyhow::bail!("PERRY_WASM_ASYNC_JS: an async body was not lowered to Wasm: {first}")
}

fn validate_wasm(bytes: &[u8]) -> Result<()> {
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(bytes)
        .map_err(|error| anyhow::anyhow!("PERRY_WASM_INVALID: {error}"))?;
    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::WASM_RUNTIME_JS;

    fn compile_source(source: &str) -> Vec<u8> {
        let ast = perry_parser::parse_typescript(source, "prd582.ts").unwrap();
        let module = perry_hir::lower_module(&ast, "prd582", "prd582.ts").unwrap();
        super::emit::compile_to_wasm(&[("prd582".into(), module)])
    }

    #[test]
    fn prd582_foreach_statement_leaves_no_stack_value() {
        use perry_hir::ir::{Expr, Module, Stmt};
        let mut module = Module::new("prd582");
        module.init.push(Stmt::Expr(Expr::NativeMethodCall {
            module: "array".into(), class_name: None,
            object: Some(Box::new(Expr::Array(vec![Expr::Number(1.0)]))),
            method: "forEach".into(), args: vec![Expr::Undefined],
        }));
        let bytes = super::emit::compile_to_wasm(&[("prd582".into(), module)]);
        wasmparser::Validator::new().validate_all(&bytes).unwrap();
    }

    #[test]
    fn prd582_prefix_property_update_leaves_one_stack_value() {
        let bytes = compile_source("let item = {count: 2}; console.log(++item.count); console.log(item.count);");
        wasmparser::Validator::new().validate_all(&bytes).unwrap();
    }

    #[test]
    fn prd582_invalid_wasm_is_rejected() {
        use wasm_encoder::{CodeSection, Function, FunctionSection, Instruction, Module, TypeSection, ValType};
        let mut module = Module::new();
        let mut types = TypeSection::new();
        types.ty().function([], [ValType::I64]);
        module.section(&types);
        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);
        let mut body = Function::new([]);
        body.instruction(&Instruction::End);
        let mut code = CodeSection::new();
        code.function(&body);
        module.section(&code);
        let error = super::validate_wasm(&module.finish()).unwrap_err();
        assert!(error.to_string().contains("PERRY_WASM_INVALID"));
    }

    #[test]
    fn prd582_async_body_left_as_js_fails_the_build() {
        assert!(super::check_no_async_js("").is_ok());
        let error = super::check_no_async_js("__async_load: function(n) {}").unwrap_err();
        assert!(error.to_string().contains("PERRY_WASM_ASYNC_JS"));
        assert!(!WASM_RUNTIME_JS.contains("__asyncFuncImpls"));
    }

    #[test]
    fn text_input_callbacks_pass_plain_js_strings_to_wasm_closures() {
        assert_eq!(
            WASM_RUNTIME_JS
                .matches("callWasmClosure(el._perryCallback, el.value)")
                .count(),
            3,
            "TextField, SecureField, and TextArea must pass their DOM strings directly"
        );
        assert!(
            !WASM_RUNTIME_JS
                .lines()
                .any(|line| { line.contains("callWasmClosure") && line.contains("fromJsValue") }),
            "callWasmClosure owns JS-value conversion; callers must not pre-box arguments"
        );
    }

    /// Emitted memcall names with no `__memDispatch` entry (PRD-582 survey): mem_call falls
    /// through to class dispatch and silently returns undefined. Shrink this list, never grow it.
    const UNMAPPED_MEMCALLS: &[&str] = &[
        "array_copy_within", "array_entries", "array_from_mapped", "array_keys", "array_push_hole",
        "array_to_reversed", "array_to_sorted", "array_to_sorted_cmp", "array_to_spliced",
        "array_values", "array_with", "date_set_date", "date_set_full_year", "date_set_hours",
        "date_set_milliseconds", "date_set_minutes", "date_set_month", "date_set_seconds",
        "date_set_time", "date_to_utc_string", "finreg_new", "finreg_register", "finreg_unregister",
        "map_new_from_array", "path_delimiter", "path_format", "path_matches_glob", "path_normalize",
        "path_parse", "path_relative", "path_resolve_join", "path_sep", "path_to_namespaced_path",
        "path_win32_join", "string_char_code_at", "weakref_deref", "weakref_new",
    ];

    #[test]
    fn prd582_every_emitted_memcall_has_a_dispatch_entry() {
        fn walk(dir: &std::path::Path, out: &mut String) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push_str(&std::fs::read_to_string(path).unwrap());
                }
            }
        }
        let mut source = String::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/emit"), &mut source);
        let start = WASM_RUNTIME_JS.find("const __memDispatch = {").unwrap();
        let dispatch = &WASM_RUNTIME_JS[start..];
        let mut missing = std::collections::BTreeSet::new();
        for call in source.split("emit_memcall").skip(1) {
            let mut parts = call.split('"');
            let head = parts.next().unwrap();
            let head: String = head.chars().filter(|c| !c.is_whitespace()).collect();
            let direct = ["(func,", "_void(func,", "_i32(func,"].contains(&head.as_str());
            let (true, Some(name)) = (direct, parts.next()) else { continue };
            if !dispatch.contains(&format!("\n  {name}:")) && !dispatch.contains(&format!("\n  {name}(")) {
                missing.insert(name.to_string());
            }
        }
        let known: std::collections::BTreeSet<String> =
            UNMAPPED_MEMCALLS.iter().map(|s| s.to_string()).collect();
        assert_eq!(missing, known, "memcall names without a __memDispatch entry");
    }

    #[test]
    fn prd582_every_emitted_memcall_name_is_interned() {
        let emit_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/emit");
        let interned = std::fs::read_to_string(emit_dir.join("string_collection.rs")).unwrap();
        let mut source = String::new();
        for entry in std::fs::read_dir(&emit_dir).unwrap().chain(std::fs::read_dir(emit_dir.join("expr")).unwrap()) {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "rs") {
                source.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
        let mut missing = Vec::new();
        for call in source.split("emit_memcall").skip(1) {
            let mut parts = call.split('"');
            let head: String = parts.next().unwrap().chars().filter(|c| !c.is_whitespace()).collect();
            let direct = ["(func,", "_void(func,", "_i32(func,"].contains(&head.as_str());
            if let (true, Some(name)) = (direct, parts.next()) {
                if !interned.contains(&format!("\"{name}\"")) {
                    missing.push(name.to_string());
                }
            }
        }
        assert!(missing.is_empty(), "memcall names not interned: {missing:?}");
    }
}
