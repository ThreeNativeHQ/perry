use perry_diagnostics::SourceCache;
use perry_hir::{lower_module, set_unimplemented_strict_mode, Expr, Module, Stmt};
use perry_parser::parse_typescript_with_cache;

fn lower(src: &str, strict: bool) -> anyhow::Result<Module> {
    let mut cache = SourceCache::new();
    let parsed = parse_typescript_with_cache(src, "strict_source.ts", &mut cache)?;
    set_unimplemented_strict_mode(strict);
    perry_hir::ir::set_current_module_source(src.to_owned());
    let result = lower_module(&parsed.module, "test", "strict_source.ts");
    perry_hir::ir::clear_current_module_source();
    set_unimplemented_strict_mode(false);
    result
}

#[test]
fn strict_omits_function_class_and_folded_constructor_source() {
    let src = "x=>{\"use strict\";\n}; function f(){return 7} class C {} const g = new Function('return 8');";
    let ordinary = lower(src, false).expect("ordinary compile");
    assert!(!ordinary.closure_source_text.is_empty());
    assert!(!ordinary.class_source_text.is_empty());
    let strict = lower(src, true).expect("strict compile");
    assert!(strict.closure_source_text.is_empty());
    assert!(strict.class_source_text.is_empty());
    assert!(strict.functions.iter().any(|f| f.name == "f"));
    assert!(strict.init.iter().any(|stmt| matches!(
        stmt,
        Stmt::Expr(Expr::Closure {
            is_strict: true,
            ..
        })
    )));
}

#[test]
fn strict_reflection_is_refused_by_name_for_provable_callables() {
    let mut failures = Vec::new();
    for src in [
        "function f(){return 7} f.toString()",
        "function f(){return 7} const g = f; g.toString()",
        "function f(){return 7} String(f)",
        "new Function('function f(){return 7} return f.toString()')",
        "function f(){return 7} String?.(f)",
        "function f(){return 7} `${f}`",
        "class C {} C.toString()",
        "class C {} const B = C; B.toString()",
        "const C = class {}; C.toString()",
        "(function f(){return 7}).toString()",
        "(class C {}).toString()",
        "class C { method(){return 7} } const c = new C(); c.method.toString()",
        "class C { method(x: any){return x} } const c = new C(); c.method.toString()",
        "class C { static method(){return 7} } C.method.toString()",
        "const o = {method: ()=>7}; o.method.toString()",
        "class C {} String(C)",
        "class C {} `${C}`",
        "Function.prototype.toString",
        "Function.prototype.toString.call(()=>7)",
        "Function.prototype.toString.bind(()=>7)",
        "const show = Function.prototype.toString; show.call(()=>7)",
        "const f = ()=>7; f?.toString()",
        "const f = ()=>7; f['toString']()",
    ] {
        match lower(src, true) {
            Err(error) if error.to_string().contains("ERR_STRICT_SOURCE_REFLECTION") => {}
            result => failures.push(format!("{src}: {result:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn uncertain_receivers_primitive_radix_and_object_tag_remain_allowed() {
    for src in [
        "const n: number = 31; n.toString(16)",
        "const n: any = 5; n.toString(36)",
        "class C { get n(){return 5} } const c = new C(); c.n.toString(36)",
        "function reflect(x: any){return x.toString()}",
        "function reflect(x: any){return x.toString(16)}",
        "function reflect(x: any){return String(x)}",
        "function reflect(x: unknown){return String(x)}",
        "function reflect(x: object){return x.toString()}",
        "class C {} const B = C; function reflect(B: any){return B.toString()}",
        "class C {} const B = C; function reflect(B){return String(B)}",
        "class C { static method(){return 7} } function reflect(C: any){return C.method.toString()}",
        "function reflect(x: any){return `${x}`}",
        "function f(){return 7} const x: any = f; x.toString()",
        "function f(){return 7} const x: any = f; const y = x; y.toString()",
        "class C {} const x: any = C; const y = x; String(y)",
        "class C {} const x: any = C; String(x)",
        "const C: any = class {}; String(C)",
        "const stringify = String; stringify(()=>7)",
        "const p = Function.prototype",
        "String(7); String('ok'); `${7}`; String(7, ()=>8)",
        "Object.prototype.toString.call(()=>7)",
        "const tag = Object.prototype.toString; tag.call({})",
        "function String(x: any){return 7} String(()=>7)",
    ] {
        lower(src, true).expect(src);
    }
}
