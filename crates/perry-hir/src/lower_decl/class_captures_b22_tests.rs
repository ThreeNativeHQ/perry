//! B22: an instance member rebinds only the class captures it reads, and a
//! symbol-keyed computed member does not mark the class as string-keyable.

use crate::analysis::collect_local_refs_stmt;
use crate::ir::{Class, Expr, Function, Module, Stmt};
use crate::lower_module;
use perry_diagnostics::SourceCache;
use perry_parser::parse_typescript_with_cache;

fn lower_src(src: &str) -> Module {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut cache = SourceCache::new();
            let parsed = parse_typescript_with_cache(&src, "test.ts", &mut cache)
                .expect("parse should succeed");
            lower_module(&parsed.module, "test", "test.ts").expect("lowering should succeed")
        })
        .expect("spawn")
        .join()
        .expect("lowering thread")
}

const SOURCE: &str = r#"
export function make(k: number, tag: string) {
  class V {
    declare x: number;
    constructor(x = 0) { this.x = x; }
    set(x: number): this { this.x = x; return this; }
    spin(): number { return k + this.x; }
    clone(): V { return new V(this.x); }
    *[Symbol.iterator](): Generator<number> { yield this.x; }
  }
  return [V, tag] as const;
}
export function calls(k: number) {
  class C {
    declare y: number;
    constructor() { this.y = this.read(); }
    read(): number { return k; }
  }
  return C;
}
export function keyed(name: string) {
  class W {
    w = 1;
    get [name](): number { return 2; }
  }
  return W;
}
"#;

fn class<'a>(module: &'a Module, name: &str) -> &'a Class {
    module.classes.iter().find(|c| c.name == name).expect("class")
}

fn method<'a>(class: &'a Class, name: &str) -> &'a Function {
    class.methods.iter().find(|m| m.name == name).expect("method")
}

/// Capture rebinds are the leading `__perry_cap_*` lets.
fn capture_lets(func: &Function) -> usize {
    func.body
        .iter()
        .take_while(|s| matches!(s, Stmt::Let { name, .. } if name.starts_with("__perry_cap_")))
        .count()
}

#[test]
fn unused_captures_are_not_rebound() {
    let module = lower_src(SOURCE);
    let v = class(&module, "V");
    assert_eq!(capture_lets(method(v, "set")), 0, "set reads no capture");
    assert_eq!(capture_lets(method(v, "spin")), 1, "spin reads k only");
    // Every remaining rebind is referenced by the body after the prologue.
    let spin = method(v, "spin");
    let Stmt::Let { id, .. } = &spin.body[0] else { panic!("let") };
    let mut refs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for stmt in &spin.body[1..] {
        collect_local_refs_stmt(stmt, &mut refs, &mut seen);
    }
    assert!(refs.contains(id));
    // A self `new` keeps every rebind it appends as a constructor argument.
    let clone = method(v, "clone");
    let mut refs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for stmt in &clone.body[capture_lets(clone)..] {
        collect_local_refs_stmt(stmt, &mut refs, &mut seen);
    }
    for stmt in &clone.body[..capture_lets(clone)] {
        let Stmt::Let { id, .. } = stmt else { unreachable!() };
        assert!(refs.contains(id), "clone keeps only referenced rebinds");
    }
    assert!(capture_lets(clone) >= 1, "clone forwards its captures to new V");
}

#[test]
fn symbol_keyed_computed_members_are_not_string_keyable() {
    let module = lower_src(SOURCE);
    let v = class(&module, "V");
    assert!(!v.computed_members.is_empty(), "iterator registers a computed member");
    assert!(v
        .computed_members
        .iter()
        .all(|m| matches!(&m.key_expr, Expr::SymbolFor(_))));
    assert!(!v.has_string_keyable_computed_members());
    assert!(class(&module, "W").has_string_keyable_computed_members());
}


/// Index of the first `this.__perry_cap_* = …` stash in the constructor.
fn first_stash(class: &Class) -> usize {
    class
        .constructor
        .as_ref()
        .expect("ctor")
        .body
        .iter()
        .position(|s| {
            matches!(s, Stmt::Expr(Expr::PropertySet { object, property, .. })
                if matches!(object.as_ref(), Expr::This) && property.starts_with("__perry_cap_"))
        })
        .expect("stash")
}

#[test]
fn plain_constructor_stashes_captures_after_its_fields() {
    let module = lower_src(SOURCE);
    let v = class(&module, "V");
    let body = &v.constructor.as_ref().unwrap().body;
    let field_store = body
        .iter()
        .position(|s| matches!(s, Stmt::Expr(Expr::PutValueSet { .. } | Expr::PropertySet { .. })))
        .unwrap();
    assert!(first_stash(v) > field_store, "stash follows this.x, matching the declared layout");
    let order: Vec<_> = v.fields.iter().map(|f| f.name.starts_with("__perry_cap_")).collect();
    assert_eq!(order.first(), Some(&false), "layout declares x before the hidden fields");
    // A constructor that calls a method still stashes before running it.
    let c = class(&module, "C");
    let body = &c.constructor.as_ref().unwrap().body;
    let user_store = body
        .iter()
        .position(|s| matches!(s, Stmt::Expr(Expr::PutValueSet { .. })) || matches!(s, Stmt::Expr(Expr::PropertySet { property, .. }) if property == "y"))
        .expect("this.y store");
    assert!(first_stash(c) < user_store, "early stash precedes the method call");
}
