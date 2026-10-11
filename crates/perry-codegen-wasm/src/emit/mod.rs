//! HIR → WebAssembly bytecode emitter
//!
//! Translates HIR modules to WebAssembly binary format using wasm-encoder.
//! All JSValues are represented as i64 using NaN-boxing bit patterns.
//! Arithmetic operations temporarily convert to f64 and back.
//! Runtime operations (strings, console, objects) are imported from a JS bridge.
//!
//! ## Module layout
//!
//! `mod.rs` is the entry point and the cross-sibling re-export hub. Every
//! sibling file in this directory does `use super::*;` to pull in the shared
//! prelude built up below.
//!
//! - `constants` — NaN-boxing tag constants + `EnumResolvedValue` + `f64_const`
//! - `runtime_imports` — `RuntimeImports` struct (JS-bridge function indices)
//! - `ui_method_map` — `map_ui_method` perry/ui name → bridge function map
//! - `module_emitter` — `WasmModuleEmitter` struct + `new` / `intern_string` / `get_type_idx`
//! - `compile` — the giant `WasmModuleEmitter::compile` orchestration method
//! - `func_emit_ctx` — `FuncEmitCtx` struct + `new`
//! - `binary`, `closures`, `expr/`, `function`, `js_fallback`, `locals`,
//!   `memcall`, `method_call`, `stmt`, `string_collection` — pre-existing
//!   wave-1 sibling files

mod binary;
mod closures;
mod compile;
mod constants;
mod expr;
mod func_emit_ctx;
mod function;
mod js_fallback;
mod locals;
mod memcall;
mod method_call;
mod module_emitter;
mod runtime_imports;
mod stmt;
mod string_collection;
mod ui_method_map;

// Shared prelude — each sibling re-imports these via `use super::*;`.
use perry_hir::ir::*;
use perry_hir::types::{FuncId, GlobalId, LocalId};
use std::collections::BTreeMap;
use wasm_encoder::{
    CodeSection, DataSection, ElementSection, Elements, EntityType, ExportKind, ExportSection,
    Function, FunctionSection, GlobalSection, GlobalType, Ieee64, ImportSection, Instruction,
    MemorySection, MemoryType, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

// Explicit named re-exports of cross-sibling items. Sub-modules with
// `use super::*;` see these names at module scope.
use closures::{collect_closures_from_expr, collect_closures_from_stmts, THIS_CAPTURE};
// `f64_const_bits` is held alive for future use (matches the pre-split
// `#[allow(dead_code)]` annotation on its original definition).
use constants::{
    f64_const, EnumResolvedValue, STRING_TAG, TAG_FALSE, TAG_NULL, TAG_TRUE, TAG_UNDEFINED,
};
use func_emit_ctx::FuncEmitCtx;
use locals::{
    collect_exported_names, collect_locals, collect_module_let_ids, resolve_export_to_func,
    resolve_export_to_let, resolve_source_module_idx,
};
use module_emitter::WasmModuleEmitter;
use runtime_imports::RuntimeImports;
use stmt::has_return;
use ui_method_map::map_ui_method;

/// Output from WASM compilation: binary + extra JS for async functions.
pub struct WasmCompileOutput {
    pub wasm_bytes: Vec<u8>,
    pub async_js: String,
    /// FFI function names that must be provided as imports under the "ffi" namespace.
    pub ffi_imports: Vec<String>,
    /// HIR expression kinds lowered to `undefined` because the emitter has no lowering, with counts.
    pub unsupported: Vec<(String, usize)>,
}

/// Compile HIR modules to a WebAssembly binary.
pub fn compile_to_wasm(modules: &[(String, perry_hir::ir::Module)]) -> Vec<u8> {
    compile_to_wasm_with_async(modules).wasm_bytes
}

/// Compile HIR modules to WASM binary + generated JS for async functions.
pub fn compile_to_wasm_with_async(
    modules: &[(String, perry_hir::ir::Module)],
) -> WasmCompileOutput {
    let mut emitter = WasmModuleEmitter::new();
    emitter.compile(&with_implicit_constructors(modules))
}

/// The emitter runs field initializers only inside a compiled constructor, so a class without
/// one never initialized `x = 1` fields (PRD-582). Give such a class the implicit constructor:
/// empty for a base class, `constructor(a, b) { super(a, b) }` with the parent's arity for a
/// derived class whose parent is in the graph.
// ponytail: derived classes forward the parent's declared arity, not `...args`; spread when needed.
fn with_implicit_constructors(
    modules: &[(String, perry_hir::ir::Module)],
) -> Vec<(String, perry_hir::ir::Module)> {
    let mut modules = modules.to_vec();
    let mut arity: BTreeMap<String, usize> = modules
        .iter()
        .flat_map(|(_, m)| &m.classes)
        .filter_map(|c| Some((c.name.clone(), c.constructor.as_ref()?.params.len())))
        .collect();
    {
        for (_, module) in &mut modules {
            for class in &mut module.classes {
                if class.constructor.is_some() {
                    continue;
                }
                let has_inits = class.fields.iter().any(|f| f.init.is_some());
                let parent = class.extends_name.as_ref().or(class.native_extends.as_ref().map(|n| &n.0));
                let params: Vec<Param> = match parent {
                    None if has_inits => Vec::new(),
                    Some(name) if class.native_extends.is_none() && class.extends_expr.is_none() => {
                        // A parent without any constructor takes no arguments.
                        let n = arity.get(name).copied().unwrap_or(0);
                        if n == 0 && !has_inits && !arity.contains_key(name) {
                            continue;
                        }
                        (0..n)
                            .map(|i| Param {
                                id: THIS_CAPTURE - 1 - i as LocalId,
                                name: format!("__arg{i}"),
                                ty: perry_hir::types::Type::Any,
                                default: None,
                                decorators: Vec::new(),
                                is_rest: false,
                                arguments_object: None,
                            })
                            .collect()
                    }
                    _ => continue,
                };
                let body = if parent.is_some() {
                    vec![Stmt::Expr(Expr::SuperCall(params.iter().map(|p| Expr::LocalGet(p.id)).collect()))]
                } else {
                    Vec::new()
                };
                arity.insert(class.name.clone(), params.len());
                class.constructor = Some(perry_hir::ir::Function {
                    id: FuncId::MAX,
                    name: "constructor".into(),
                    type_params: Vec::new(),
                    params,
                    return_type: perry_hir::types::Type::Void,
                    body,
                    is_async: false,
                    is_generator: false,
                    is_strict: true,
                    is_exported: false,
                    captures: Vec::new(),
                    decorators: Vec::new(),
                    was_plain_async: false,
                    was_unrolled: false,
                });
            }
        }
    }
    modules
}
